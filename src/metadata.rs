//! Symbol metadata (upstream `symbol.metadata`).
//!
//! Field values mirror upstream exactly, including its string sentinels:
//! `affiliation` is `Some("undefined")` for an unrecognised letter-SIDC
//! affiliation but `None` (JavaScript `undefined`) for an unmapped numeric one.

use crate::geometry::{self, BaseGeometry};
use alloc::string::String;

/// Flags upstream only sets for some SIDCs; `None` means "not present".
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OptionalFlags {
    /// Standard edition for numeric SIDCs (`"D"` or `"E"`).
    pub edition: Option<String>,
    /// Suspect standard identity (2525E).
    pub suspect: Option<bool>,
    /// Land equipment symbol set.
    pub landequipment: Option<bool>,
    /// Control measure (tactical graphic).
    pub control_measure: Option<bool>,
    /// Cyberspace symbol set.
    pub cyberspace: Option<bool>,
    /// Dismounted individual.
    pub dismounted: Option<bool>,
    /// Feint/dummy indicator.
    pub feint_dummy: Option<bool>,
    /// Leadership indicator (dismounted individuals).
    pub leadership: Option<String>,
    /// Sector 1 modifier code (`_modifier1`, numeric SIDCs).
    pub modifier1: Option<String>,
    /// Sector 2 modifier code (`_modifier2`, numeric SIDCs).
    pub modifier2: Option<String>,
}

/// Parsed and derived properties of a symbol.
#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    /// Activity/event symbol.
    pub activity: bool,
    /// Affiliation the frame is drawn as (`Friend`, `Hostile`, …).
    pub affiliation: Option<String>,
    /// Affiliation before joker/faker remapping (upstream `baseAffilation`).
    pub base_affiliation: Option<String>,
    /// Dimension before equipment/dismounted remapping.
    pub base_dimension: String,
    /// Base frame geometry name, if the dimension/affiliation has one.
    pub base_geometry: Option<&'static str>,
    /// Civilian symbol.
    pub civilian: bool,
    /// Operational condition (`FullyCapable`, `Damaged`, …) or empty.
    pub condition: String,
    /// Context (`Reality`, `Exercise`, `Simulation`).
    pub context: Option<String>,
    /// Dimension the frame is drawn as (`Air`, `Ground`, `Sea`, …).
    pub dimension: String,
    /// Unknown battle dimension (question-mark icon).
    pub dimension_unknown: bool,
    /// Echelon name, empty when none.
    pub echelon: Option<String>,
    /// Faker.
    pub faker: bool,
    /// Always `false`; kept for output compatibility (upstream typo field).
    pub fenint_dummy: bool,
    /// Frame is filled.
    pub fill: bool,
    /// Frame is drawn.
    pub frame: bool,
    /// Icon part of the SIDC (entity code for numeric SIDCs).
    pub function_id: String,
    /// Headquarters.
    pub headquarters: bool,
    /// Installation.
    pub installation: bool,
    /// Joker.
    pub joker: bool,
    /// Mobility indicator name, empty when none; `None` for an invalid code.
    pub mobility: Option<String>,
    /// Dash array of the planned/pending frame, empty when present.
    pub notpresent: String,
    /// The SIDC is numeric.
    pub number_sidc: bool,
    /// Space symbol.
    pub space: bool,
    /// Rendering follows MIL-STD-2525 (otherwise APP-6).
    pub std2525: bool,
    /// Task force.
    pub task_force: bool,
    /// Unit (as opposed to equipment).
    pub unit: bool,
    /// Flags present only for some SIDCs.
    pub flags: OptionalFlags,
}

impl Metadata {
    pub(crate) fn new(fill: bool, frame: bool, std2525: bool) -> Self {
        Metadata {
            activity: false,
            affiliation: Some(String::from("undefined")),
            base_affiliation: Some(String::new()),
            base_dimension: String::new(),
            base_geometry: None,
            civilian: false,
            condition: String::new(),
            context: Some(String::from("Reality")),
            dimension: String::from("undefined"),
            dimension_unknown: false,
            echelon: Some(String::new()),
            faker: false,
            fenint_dummy: false,
            fill,
            frame,
            function_id: String::new(),
            headquarters: false,
            installation: false,
            joker: false,
            mobility: Some(String::new()),
            notpresent: String::new(),
            number_sidc: false,
            space: false,
            std2525,
            task_force: false,
            unit: false,
            flags: OptionalFlags::default(),
        }
    }

    /// Affiliation as a string slice (`""` for JavaScript `undefined`).
    pub fn affiliation_str(&self) -> &str {
        self.affiliation.as_deref().unwrap_or("")
    }

    /// The base frame geometry, if any.
    pub fn geometry(&self) -> Option<&'static BaseGeometry> {
        self.base_geometry.and_then(geometry::by_name)
    }

    /// Bounding box of the base geometry (`100,100,100,100` without one).
    pub fn geometry_bbox(&self) -> crate::BBox {
        self.geometry()
            .map_or_else(crate::BBox::default, BaseGeometry::bbox)
    }

    /// Whether this is a control measure.
    pub fn control_measure(&self) -> bool {
        self.flags.control_measure == Some(true)
    }

    /// Whether this is a dismounted individual.
    pub fn dismounted(&self) -> bool {
        self.flags.dismounted == Some(true)
    }

    /// Standard edition of a numeric SIDC.
    pub fn edition(&self) -> Option<&str> {
        self.flags.edition.as_deref()
    }
}
