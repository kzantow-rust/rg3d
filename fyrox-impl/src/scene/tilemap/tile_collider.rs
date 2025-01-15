// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Tile colliders provide shapes for tiles so that physics colliders may be automatically
//! constructed for tile maps. [`TileCollider`] divides the colliders into broad categories,
//! including no shape and a shape that covers the full tile, while [`CustomTileCollider`]
//! is a resource that contains triangles to allow an arbitrary shape to be constructed for
//! any tile.

use crate::{
    asset::{Resource, ResourceData},
    core::{
        algebra::{Matrix4, Point2, Point3, Vector2},
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
};
use std::{
    error::Error,
    fmt::{Debug, Display, Formatter},
    num::{ParseFloatError, ParseIntError},
    path::Path,
    str::FromStr,
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

use super::*;

/// Supported collider types for tiles.
#[derive(
    Clone,
    Hash,
    PartialEq,
    Eq,
    Default,
    Visit,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
    TypeUuidProvider,
)]
#[type_uuid(id = "04a44fec-394f-4497-97d5-fe9e6f915831")]
pub enum TileCollider {
    /// No collider.
    #[default]
    None,
    /// Rectangle collider that covers the full tile.
    Rectangle,
    /// User-defined collider containing a reference to a resource that contains the triangles.
    Custom(CustomTileColliderResource),
    /// Mesh collider, the mesh is autogenerated.
    Mesh,
}

impl Default for &TileCollider {
    fn default() -> Self {
        &TileCollider::None
    }
}

impl Debug for TileCollider {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Rectangle => write!(f, "Rectangle"),
            Self::Custom(r) => write!(f, "Custom({})", r.data_ref().deref()),
            Self::Mesh => write!(f, "Mesh"),
        }
    }
}

impl OrthoTransform for TileCollider {
    fn x_flipped(self) -> Self {
        if let Self::Custom(collider) = self {
            let collider = collider.data_ref().clone();
            Self::Custom(Resource::new_ok(
                ResourceKind::Embedded,
                collider.x_flipped(),
            ))
        } else {
            self
        }
    }
    fn rotated(self, amount: i8) -> Self {
        if let Self::Custom(collider) = self {
            let collider = collider.data_ref().clone();
            Self::Custom(Resource::new_ok(
                ResourceKind::Embedded,
                collider.rotated(amount),
            ))
        } else {
            self
        }
    }
}

impl TileCollider {
    /// This collider is empty.
    pub fn is_none(&self) -> bool {
        matches!(self, TileCollider::None)
    }
    /// This collider is a full rectangle.
    pub fn is_rectangle(&self) -> bool {
        matches!(self, TileCollider::Rectangle)
    }
    /// This collider is a custom mesh.
    pub fn is_custom(&self) -> bool {
        matches!(self, TileCollider::Custom(_))
    }

    /// Generate the mesh for this collider.
    pub fn build_collider_shape(
        &self,
        transform: &Matrix4<f32>,
        position: Vector3<f32>,
        vertices: &mut Vec<Point2<f32>>,
        triangles: &mut Vec<[u32; 3]>,
    ) {
        match self {
            TileCollider::None => (),
            TileCollider::Rectangle => {
                let origin = vertices.len() as u32;
                for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
                    let offset = Vector3::new(dx, dy, 0.0);
                    let point = Point3::from(position + offset);
                    vertices.push(transform.transform_point(&point).xy());
                }

                triangles.push([origin, origin + 1, origin + 2]);
                triangles.push([origin, origin + 2, origin + 3]);
            }
            TileCollider::Custom(resource) => {
                resource
                    .data_ref()
                    .build_collider_shape(transform, position, vertices, triangles);
            }
            TileCollider::Mesh => (), // TODO: Add image-to-mesh conversion
        }
    }
}

/// A resource to hold triangle data for a tile collider arranged in rectangle from (0,0) to (1,1).
pub type CustomTileColliderResource = Resource<CustomTileCollider>;
/// Triangle data for a tile collider arranged in rectangle from (0,0) to (1,1).
/// Custom tile colliders can be converted to and from strings, where the strings contain
/// 2D vertex positions and triangle index triples. A group of two numbers is taken to be
/// a 2D vertex position while a group of three numbers is taken to be an index triple,
/// and the numbers are therefore parsed as ints. For example, "0,0 1,1 1,0 0,1,2" would be
/// a valid string for a custom tile collider. The commas (,) are used to connect two numbers
/// as being part of the same group. Any other characters are ignored, so this would also be
/// accepted: "(0,0) (1,1) (1,0) \[0,1,2\]".
#[derive(Clone, PartialEq, Debug, Default, Visit, Reflect, TypeUuidProvider)]
#[type_uuid(id = "118da556-a444-4bd9-bd88-12d78d26107f")]
pub struct CustomTileCollider {
    /// The vertices of the triangles, with the boundaries of the tile being between (0,0) and (1,1).
    pub vertices: Vec<Vector2<f32>>,
    /// The indices of the vertices of each triangle
    pub triangles: Vec<TriangleDefinition>,
}

impl ResourceData for CustomTileCollider {
    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut visitor = Visitor::new();
        self.visit("CustomTileCollider", &mut visitor)?;
        visitor.save_binary(path)?;
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        false
    }
}

impl OrthoTransform for CustomTileCollider {
    fn x_flipped(self) -> Self {
        Self {
            vertices: self
                .vertices
                .iter()
                .map(|v| Vector2::new(1.0 - v.x, v.y))
                .collect(),
            ..self
        }
    }

    fn rotated(self, amount: i8) -> Self {
        let center = Vector2::new(0.5, 0.5);
        Self {
            vertices: self
                .vertices
                .iter()
                .map(|v| (v - center).rotated(amount) + center)
                .collect(),
            ..self
        }
    }
}

impl CustomTileCollider {
    /// Construct triangles to represent this collider by appending to the given
    /// vectors, starting from the given lower-left corner of the tile and finally
    /// applying the given transformation matrix.
    /// The transformation and position are in 3D, but the resulting 2D vertices
    /// ignore the z-coordinate.
    pub fn build_collider_shape(
        &self,
        transform: &Matrix4<f32>,
        position: Vector3<f32>,
        vertices: &mut Vec<Point2<f32>>,
        triangles: &mut Vec<[u32; 3]>,
    ) {
        let origin = vertices.len() as u32;
        triangles.extend(self.triangles.iter().map(|d| d.0.map(|i| i + origin)));
        vertices.extend(self.vertices.iter().map(|p| {
            transform
                .transform_point(&Point3::from(position + p.to_homogeneous()))
                .xy()
        }));
    }
}

impl Display for CustomTileCollider {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for v in self.vertices.iter() {
            if !first {
                write!(f, " ")?;
            }
            first = false;
            write!(f, "({}, {})", v.x, v.y)?;
        }
        for TriangleDefinition(t) in self.triangles.iter() {
            if !first {
                write!(f, " ")?;
            }
            first = false;
            write!(f, "[{}, {}, {}]", t[0], t[1], t[2])?;
        }
        Ok(())
    }
}

/// Errors that may occur while parsing a custom tile collider.
#[derive(Debug)]
pub enum CustomTileColliderStrError {
    /// A group is shorter than 2. For example: "7"
    GroupTooShort,
    /// A group is longer than 3. For example: "7,7,8,9"
    GroupTooLong(usize),
    /// A comma (,) was found without a number. For example: "7,"
    MissingNumber,
    /// A triangle does not match any of the given vertices.
    /// For example: "0,0 1,1 0,1,2". The final "2" is illegal because there are only two vertices given.
    IndexOutOfBounds(u32),
    /// Failed to parse an entry in a length-2 group as an f32. For example: "0,0.2.3"
    IndexParseError(ParseIntError),
    /// Failed to parse an entry in a length-3 group as a u32. For example: "0,1.2,3"
    CoordinateParseError(ParseFloatError),
}

impl From<ParseIntError> for CustomTileColliderStrError {
    fn from(value: ParseIntError) -> Self {
        Self::IndexParseError(value)
    }
}

impl From<ParseFloatError> for CustomTileColliderStrError {
    fn from(value: ParseFloatError) -> Self {
        Self::CoordinateParseError(value)
    }
}

impl Error for CustomTileColliderStrError {}

impl Display for CustomTileColliderStrError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CustomTileColliderStrError::GroupTooShort => {
                write!(f, "Each group must have at least 2 numbers.")
            }
            CustomTileColliderStrError::GroupTooLong(n) => {
                write!(f, "A group has {n} numbers. No group may be longer than 3.")
            }
            CustomTileColliderStrError::IndexOutOfBounds(n) => {
                write!(
                    f,
                    "Triangle index {n} does not match any of the given vertices."
                )
            }
            CustomTileColliderStrError::MissingNumber => {
                write!(f, "Numbers in a group must be separated by commas.")
            }
            CustomTileColliderStrError::IndexParseError(parse_int_error) => {
                write!(f, "Index parse failure: {parse_int_error}")
            }
            CustomTileColliderStrError::CoordinateParseError(parse_float_error) => {
                write!(f, "Coordinate parse failure: {parse_float_error}")
            }
        }
    }
}

impl FromStr for CustomTileCollider {
    type Err = CustomTileColliderStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut group = Vec::<&str>::default();
        let mut ready = true;
        let mut vertices = Vec::<Vector2<f32>>::default();
        let mut triangles = Vec::<TriangleDefinition>::default();
        for token in TokenIter::new(s) {
            if ready {
                if token != "," {
                    group.push(token);
                    ready = false;
                } else {
                    return Err(CustomTileColliderStrError::MissingNumber);
                }
            } else if token != "," {
                process_group(&group, &mut vertices, &mut triangles)?;
                group.clear();
                group.push(token);
            } else {
                ready = true;
            }
        }
        if !group.is_empty() {
            process_group(&group, &mut vertices, &mut triangles)?;
        }
        let len = vertices.len() as u32;
        for TriangleDefinition(tri) in triangles.iter() {
            for &n in tri.iter() {
                if n >= len {
                    return Err(CustomTileColliderStrError::IndexOutOfBounds(n));
                }
            }
        }
        Ok(Self {
            vertices,
            triangles,
        })
    }
}

fn process_group(
    group: &[&str],
    vertices: &mut Vec<Vector2<f32>>,
    triangles: &mut Vec<TriangleDefinition>,
) -> Result<(), CustomTileColliderStrError> {
    use CustomTileColliderStrError as Error;
    let len = group.len();
    if len < 2 {
        return Err(Error::GroupTooShort);
    } else if len > 3 {
        return Err(Error::GroupTooLong(group.len()));
    } else if len == 2 {
        let v = Vector2::new(parse_f32(group[0])?, parse_f32(group[1])?);
        vertices.push(v);
    } else if len == 3 {
        let t = TriangleDefinition([
            u32::from_str(group[0])?,
            u32::from_str(group[1])?,
            u32::from_str(group[2])?,
        ]);
        triangles.push(t);
    }
    Ok(())
}

fn parse_f32(source: &str) -> Result<f32, ParseFloatError> {
    let value = f32::from_str(source)?;
    f32::from_str(&format!("{value:.3}"))
}

struct TokenIter<'a> {
    source: &'a str,
    position: usize,
}

impl<'a> TokenIter<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            position: 0,
        }
    }
}

fn is_number_char(c: char) -> bool {
    c.is_numeric() || c == '.' || c == '-'
}

fn is_ignore_char(c: char) -> bool {
    !is_number_char(c) && c != ','
}

impl<'a> Iterator for TokenIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let rest = self.source.get(self.position..)?;
        if rest.is_empty() {
            return None;
        }
        let mut initial_ignore = true;
        let mut start = 0;
        for (i, c) in rest.char_indices() {
            if initial_ignore {
                if is_ignore_char(c) {
                    continue;
                } else {
                    initial_ignore = false;
                    start = i;
                }
            }
            if c == ',' {
                if i == start {
                    self.position += i + 1;
                    return Some(&rest[start..i + 1]);
                } else {
                    self.position += i;
                    return Some(&rest[start..i]);
                }
            } else if is_ignore_char(c) {
                self.position += i + 1;
                return Some(&rest[start..i]);
            }
        }
        if initial_ignore {
            return None;
        }
        self.position = self.source.len();
        Some(&rest[start..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let mut iter = TokenIter::new("");
        assert_eq!(iter.next(), None);
    }
    #[test]
    fn empty2() {
        let mut iter = TokenIter::new("   ");
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn comma() {
        let mut iter = TokenIter::new("0,1");
        assert_eq!(iter.next().unwrap(), "0");
        assert_eq!(iter.next().unwrap(), ",");
        assert_eq!(iter.next().unwrap(), "1");
        assert_eq!(iter.next(), None);
    }
    #[test]
    fn comma2() {
        let mut iter = TokenIter::new(" 0.4 , -1 ");
        assert_eq!(iter.next().unwrap(), "0.4");
        assert_eq!(iter.next().unwrap(), ",");
        assert_eq!(iter.next().unwrap(), "-1");
        assert_eq!(iter.next(), None);
    }
    #[test]
    fn comma3() {
        let mut iter = TokenIter::new(",, ,");
        assert_eq!(iter.next().unwrap(), ",");
        assert_eq!(iter.next().unwrap(), ",");
        assert_eq!(iter.next().unwrap(), ",");
        assert_eq!(iter.next(), None);
    }
    #[test]
    fn number() {
        let mut iter = TokenIter::new("0");
        assert_eq!(iter.next().unwrap(), "0");
        assert_eq!(iter.next(), None);
    }
    #[test]
    fn number2() {
        let mut iter = TokenIter::new("-3.14");
        assert_eq!(iter.next().unwrap(), "-3.14");
        assert_eq!(iter.next(), None);
    }
    #[test]
    fn number3() {
        let mut iter = TokenIter::new("  -3.14 ");
        assert_eq!(iter.next().unwrap(), "-3.14");
        assert_eq!(iter.next(), None);
    }
    #[test]
    fn collider() {
        let col = CustomTileCollider::from_str("0,0; 1,1; 1,0; 0,1,2").unwrap();
        assert_eq!(col.vertices.len(), 3);
        assert_eq!(col.vertices[0], Vector2::new(0.0, 0.0));
        assert_eq!(col.vertices[1], Vector2::new(1.0, 1.0));
        assert_eq!(col.vertices[2], Vector2::new(1.0, 0.0));
        assert_eq!(col.triangles.len(), 1);
        assert_eq!(col.triangles[0], TriangleDefinition([0, 1, 2]));
    }
    #[test]
    fn collider_display() {
        let col = CustomTileCollider::from_str("0,0; 1,1; 1,0.333; 0,1,2").unwrap();
        assert_eq!(col.to_string(), "(0, 0) (1, 1) (1, 0.333) [0, 1, 2]");
    }
}