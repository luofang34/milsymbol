//! Typed metadata used by SIDC interpretation and composition.
//! Compatibility sentinels retain the distinctions observed by milsymbol.js.

use crate::domain::{
    Affiliation, Context, Dimension, Echelon, Edition, Leadership, Mobility, Status,
};
use crate::geometry::{self, BaseGeometry};
mod field;
use alloc::string::String;
pub(crate) use field::{Field, Name};

/// Flags upstream only sets for some SIDCs; `None` means "not present".
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct OptionalFlags {
    /// Standard edition for numeric SIDCs.
    pub edition: Option<Edition>,
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
    pub leadership: Option<Leadership>,
    /// Sector 1 modifier code (`_modifier1`, numeric SIDCs).
    pub modifier1: Option<String>,
    /// Sector 2 modifier code (`_modifier2`, numeric SIDCs).
    pub modifier2: Option<String>,
}

/// Parsed and derived properties of a symbol.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Metadata {
    /// Activity/event symbol.
    pub activity: bool,
    /// Affiliation the frame is drawn as (`Friend`, `Hostile`, …).
    pub affiliation: Field<Affiliation>,
    /// Affiliation before joker/faker remapping (upstream `baseAffilation`).
    pub base_affiliation: Field<Affiliation>,
    /// Dimension before equipment/dismounted remapping.
    pub base_dimension: Field<Dimension>,
    /// Base frame geometry name, if the dimension/affiliation has one.
    pub base_geometry: Option<&'static str>,
    /// Civilian symbol.
    pub civilian: bool,
    /// Operational condition, if specified.
    pub condition: Option<Status>,
    /// Context (`Reality`, `Exercise`, `Simulation`).
    pub context: Option<Context>,
    /// Dimension the frame is drawn as (`Air`, `Ground`, `Sea`, …).
    pub dimension: Field<Dimension>,
    /// Unknown battle dimension (question-mark icon).
    pub dimension_unknown: bool,
    /// Echelon, with absent and invalid codes kept distinct.
    pub echelon: Field<Echelon>,
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
    /// Mobility, with absent and invalid codes kept distinct.
    pub mobility: Field<Mobility>,
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
            affiliation: Field::Undefined,
            base_affiliation: Field::Empty,
            base_dimension: Field::Empty,
            base_geometry: None,
            civilian: false,
            condition: None,
            context: Some(Context::Reality),
            dimension: Field::Undefined,
            dimension_unknown: false,
            echelon: Field::Empty,
            faker: false,
            fenint_dummy: false,
            fill,
            frame,
            function_id: String::new(),
            headquarters: false,
            installation: false,
            joker: false,
            mobility: Field::Empty,
            notpresent: String::new(),
            number_sidc: false,
            space: false,
            std2525,
            task_force: false,
            unit: false,
            flags: OptionalFlags::default(),
        }
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
    pub fn edition(&self) -> Option<Edition> {
        self.flags.edition
    }
}
