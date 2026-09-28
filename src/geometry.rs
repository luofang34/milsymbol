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
#[non_exhaustive]
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

/// Frame name selected by typed dimension and affiliation.
pub(crate) fn frame_name(
    dimension: Option<crate::domain::Dimension>,
    affiliation: Option<crate::domain::Affiliation>,
) -> Option<&'static str> {
    use crate::domain::{Affiliation::*, Dimension::*};
    Some(match (dimension?, affiliation?) {
        (Air, Friend) => "AirFriend",
        (Air, Hostile) => "AirHostile",
        (Air, Neutral) => "AirNeutral",
        (Air, Unknown) => "AirUnknown",
        (Ground, Friend) => "GroundFriend",
        (Ground, Hostile) => "GroundHostile",
        (Ground, Neutral) => "GroundNeutral",
        (Ground, Unknown) => "GroundUnknown",
        (Sea, Friend) => "SeaFriend",
        (Sea, Hostile) => "SeaHostile",
        (Sea, Neutral) => "SeaNeutral",
        (Sea, Unknown) => "SeaUnknown",
        (Subsurface, Friend) => "SubsurfaceFriend",
        (Subsurface, Hostile) => "SubsurfaceHostile",
        (Subsurface, Neutral) => "SubsurfaceNeutral",
        (Subsurface, Unknown) => "SubsurfaceUnknown",
        (LandDismountedIndividual, Friend) => "LandDismountedIndividualFriend",
        (LandDismountedIndividual, Hostile) => "LandDismountedIndividualHostile",
        (LandDismountedIndividual, Neutral) => "LandDismountedIndividualNeutral",
        (LandDismountedIndividual, Unknown) => "LandDismountedIndividualUnknown",
    })
}
