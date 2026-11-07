//! Current frame proportions: MIL-STD-2525E Change 1 Figure 4 (p. 11),
//! APP-6(E)(2) Table 1-14 (p. 78). Coordinates use a 100-unit octagon.

use super::{BaseGeometry, GeomShape};
use crate::domain::{Affiliation, Dimension};
use crate::metadata::Metadata;

pub(super) static GEOMETRIES: [(&str, BaseGeometry); 3] = [
    (
        "StandardsUnknown",
        BaseGeometry {
            shape: GeomShape::Path(
                "M61.5306859206,61.5306859206 C61.5306859206,16.8231046931 138.469314079,16.8231046931 138.469314079,61.5306859206 C183.176895307,61.5306859206 183.176895307,138.469314079 138.469314079,138.469314079 C138.469314079,183.176895307 61.5306859206,183.176895307 61.5306859206,138.469314079 C16.8231046931,138.469314079 16.8231046931,61.5306859206 61.5306859206,61.5306859206 Z",
            ),
            bbox: [28.0, 28.0, 172.0, 172.0],
        },
    ),
    (
        "MilDismountedFriend",
        BaseGeometry {
            shape: GeomShape::Path(
                "M100,40 L155,67.27272727272727 155,132.72727272727272 100,160 45,132.72727272727272 45,67.27272727272727 Z",
            ),
            bbox: [45.0, 40.0, 155.0, 160.0],
        },
    ),
    (
        "AppSeaFriend",
        BaseGeometry {
            shape: GeomShape::Circle {
                cx: 100.0,
                cy: 100.0,
                r: 50.0,
            },
            bbox: [50.0, 50.0, 150.0, 150.0],
        },
    ),
];

pub(crate) fn frame_name(md: &Metadata) -> Option<&'static str> {
    match (md.dimension.known(), md.affiliation.known()) {
        (
            Some(Dimension::Ground | Dimension::Sea | Dimension::LandDismountedIndividual),
            Some(Affiliation::Unknown),
        ) => Some("StandardsUnknown"),
        (Some(Dimension::LandDismountedIndividual), Some(Affiliation::Friend)) if md.std2525 => {
            Some("MilDismountedFriend")
        }
        (Some(Dimension::Sea), Some(Affiliation::Friend)) if !md.std2525 => Some("AppSeaFriend"),
        (dimension, affiliation) => super::frame_name(dimension, affiliation),
    }
}
