//! Base frame geometries (upstream `symbolgeometries.js`).

use crate::BBox;
use crate::generated::misc;

/// Shape of a base frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeomShape {
    /// A path frame.
    Path(&'static str),
    /// A circular frame.
    Circle {
        /// Centre x.
        cx: f64,
        /// Centre y.
        cy: f64,
        /// Radius.
        r: f64,
    },
}

/// A base frame geometry with its bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaseGeometry {
    /// Frame shape.
    pub shape: GeomShape,
    /// `[x1, y1, x2, y2]`.
    pub bbox: [f64; 4],
}

impl BaseGeometry {
    /// Bounding box of the frame.
    pub fn bbox(&self) -> BBox {
        let [x1, y1, x2, y2] = self.bbox;
        BBox { x1, y1, x2, y2 }
    }
}

/// Looks up a geometry by upstream name (e.g. `GroundFriend`).
pub fn by_name(name: &str) -> Option<&'static BaseGeometry> {
    misc::GEOMETRIES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, g)| g)
}

/// Canonical `'static` name for a geometry name.
pub(crate) fn static_name(name: &str) -> Option<&'static str> {
    misc::GEOMETRIES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(n, _)| *n)
}
