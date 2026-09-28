//! Symbol metadata (upstream `symbol.metadata`).
//!
//! Field values mirror upstream exactly, including its string sentinels:
//! `affiliation` is `Some("undefined")` for an unrecognised letter-SIDC
//! affiliation but `None` (JavaScript `undefined`) for an unmapped numeric one.

use crate::geometry::{self, BaseGeometry};

/// Flags upstream only sets for some SIDCs; `None` means "not present".
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct OptionalFlags<'a> {
    /// Standard edition for numeric SIDCs (`"D"` or `"E"`).
    pub edition: Option<&'a str>,
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
    pub leadership: Option<&'a str>,
    /// Sector 1 modifier code (`_modifier1`, numeric SIDCs).
    pub modifier1: Option<&'a str>,
    /// Sector 2 modifier code (`_modifier2`, numeric SIDCs).
    pub modifier2: Option<&'a str>,
}

/// Borrowed string view of parsed and derived properties of a symbol.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Metadata<'a> {
    /// Activity/event symbol.
    pub activity: bool,
    /// Affiliation the frame is drawn as (`Friend`, `Hostile`, …).
    pub affiliation: Option<&'a str>,
    /// Affiliation before joker/faker remapping (upstream `baseAffilation`).
    pub base_affiliation: Option<&'a str>,
    /// Dimension before equipment/dismounted remapping.
    pub base_dimension: &'a str,
    /// Base frame geometry name, if the dimension/affiliation has one.
    pub base_geometry: Option<&'static str>,
    /// Civilian symbol.
    pub civilian: bool,
    /// Operational condition (`FullyCapable`, `Damaged`, …) or empty.
    pub condition: &'a str,
    /// Context (`Reality`, `Exercise`, `Simulation`).
    pub context: Option<&'a str>,
    /// Dimension the frame is drawn as (`Air`, `Ground`, `Sea`, …).
    pub dimension: &'a str,
    /// Unknown battle dimension (question-mark icon).
    pub dimension_unknown: bool,
    /// Echelon name, empty when none.
    pub echelon: Option<&'a str>,
    /// Faker.
    pub faker: bool,
    /// Always `false`; kept for output compatibility (upstream typo field).
    pub fenint_dummy: bool,
    /// Frame is filled.
    pub fill: bool,
    /// Frame is drawn.
    pub frame: bool,
    /// Icon part of the SIDC (entity code for numeric SIDCs).
    pub function_id: &'a str,
    /// Headquarters.
    pub headquarters: bool,
    /// Installation.
    pub installation: bool,
    /// Joker.
    pub joker: bool,
    /// Mobility indicator name, empty when none; `None` for an invalid code.
    pub mobility: Option<&'a str>,
    /// Dash array of the planned/pending frame, empty when present.
    pub notpresent: &'a str,
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
    pub flags: OptionalFlags<'a>,
}

impl Metadata<'_> {
    /// Affiliation as a string slice (`""` for JavaScript `undefined`).
    pub fn affiliation_str(&self) -> &str {
        self.affiliation.unwrap_or("")
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
        self.flags.edition
    }
}

impl<'a> From<&'a crate::metadata::Metadata> for Metadata<'a> {
    fn from(md: &'a crate::metadata::Metadata) -> Self {
        use crate::metadata::Name;
        Self {
            activity: md.activity,
            affiliation: md.affiliation.as_str(),
            base_affiliation: md.base_affiliation.as_str(),
            base_dimension: md.base_dimension.as_str().unwrap_or(""),
            base_geometry: md.base_geometry,
            civilian: md.civilian,
            condition: md.condition.map_or("", Name::name),
            context: md.context.map(Name::name),
            dimension: md.dimension.as_str().unwrap_or(""),
            dimension_unknown: md.dimension_unknown,
            echelon: md.echelon.as_str(),
            faker: md.faker,
            fenint_dummy: md.fenint_dummy,
            fill: md.fill,
            frame: md.frame,
            function_id: &md.function_id,
            headquarters: md.headquarters,
            installation: md.installation,
            joker: md.joker,
            mobility: md.mobility.as_str(),
            notpresent: &md.notpresent,
            number_sidc: md.number_sidc,
            space: md.space,
            std2525: md.std2525,
            task_force: md.task_force,
            unit: md.unit,
            flags: OptionalFlags {
                edition: md.flags.edition.map(Name::name),
                suspect: md.flags.suspect,
                landequipment: md.flags.landequipment,
                control_measure: md.flags.control_measure,
                cyberspace: md.flags.cyberspace,
                dismounted: md.flags.dismounted,
                feint_dummy: md.flags.feint_dummy,
                leadership: md.flags.leadership.map(Name::name),
                modifier1: md.flags.modifier1.as_deref(),
                modifier2: md.flags.modifier2.as_deref(),
            },
        }
    }
}
