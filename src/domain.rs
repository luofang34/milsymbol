//! Typed symbology values and [`Metadata`], the typed description of a
//! rendered symbol. milsymbol.js's string-valued metadata is available as
//! [`compat::JsMetadata`](crate::compat::JsMetadata).

use crate::geometry::BaseGeometry;

/// Standard identity (affiliation code of the SIDC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StandardIdentity {
    /// Pending.
    Pending,
    /// Unknown.
    Unknown,
    /// Assumed friend.
    AssumedFriend,
    /// Friend.
    Friend,
    /// Neutral.
    Neutral,
    /// Suspect.
    Suspect,
    /// Hostile.
    Hostile,
    /// Joker (exercise: friend acting as suspect).
    Joker,
    /// Faker (exercise: friend acting as hostile).
    Faker,
    /// None specified (letter SIDC `O`).
    NoneSpecified,
}

/// Context of the SIDC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Context {
    /// Reality.
    Reality,
    /// Exercise.
    Exercise,
    /// Simulation.
    Simulation,
}

/// Operational status or condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Status {
    /// Present.
    Present,
    /// Planned / anticipated.
    Planned,
    /// Present, fully capable.
    FullyCapable,
    /// Present, damaged.
    Damaged,
    /// Present, destroyed.
    Destroyed,
    /// Present, full to capacity.
    FullToCapacity,
}

/// Affiliation the frame is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Affiliation {
    /// Friend frame.
    Friend,
    /// Hostile frame.
    Hostile,
    /// Neutral frame.
    Neutral,
    /// Unknown frame.
    Unknown,
}

/// Dimension the frame is drawn for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Dimension {
    /// Air and space.
    Air,
    /// Land units and installations.
    Ground,
    /// Sea surface and land equipment.
    Sea,
    /// Subsurface.
    Subsurface,
    /// Land dismounted individual.
    LandDismountedIndividual,
}

/// Echelon indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Echelon {
    TeamCrew,
    Squad,
    Section,
    PlatoonDetachment,
    CompanyBatteryTroop,
    BattalionSquadron,
    RegimentGroup,
    Brigade,
    Division,
    CorpsMef,
    Army,
    ArmyGroupFront,
    RegionTheater,
    Command,
}

/// Mobility indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Mobility {
    WheeledLimitedCrossCountry,
    WheeledCrossCountry,
    Tracked,
    WheeledAndTracked,
    Towed,
    Rail,
    PackAnimals,
    OverSnow,
    Sled,
    Barge,
    Amphibious,
    ShortTowedArray,
    LongTowedArray,
}

/// Leadership indicator of dismounted individuals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Leadership {
    /// Leader individual.
    Leader,
    /// Deputy individual.
    Deputy,
}

impl Echelon {
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "11" => Self::TeamCrew,
            "12" => Self::Squad,
            "13" => Self::Section,
            "14" => Self::PlatoonDetachment,
            "15" => Self::CompanyBatteryTroop,
            "16" => Self::BattalionSquadron,
            "17" => Self::RegimentGroup,
            "18" => Self::Brigade,
            "21" => Self::Division,
            "22" => Self::CorpsMef,
            "23" => Self::Army,
            "24" => Self::ArmyGroupFront,
            "25" => Self::RegionTheater,
            "26" => Self::Command,
            _ => return None,
        })
    }
}

impl Mobility {
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "31" => Self::WheeledLimitedCrossCountry,
            "32" => Self::WheeledCrossCountry,
            "33" => Self::Tracked,
            "34" => Self::WheeledAndTracked,
            "35" => Self::Towed,
            "36" => Self::Rail,
            "37" => Self::PackAnimals,
            "41" => Self::OverSnow,
            "42" => Self::Sled,
            "51" => Self::Barge,
            "52" => Self::Amphibious,
            "61" => Self::ShortTowedArray,
            "62" => Self::LongTowedArray,
            _ => return None,
        })
    }
}

/// Edition of the standard a numeric SIDC follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Edition {
    /// MIL-STD-2525D / APP-6D (SIDC versions 10–12).
    D,
    /// MIL-STD-2525E / APP-6E (SIDC versions 13–14).
    E,
}

/// Typed description of a rendered symbol.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Metadata {
    /// Frame affiliation; `None` when the SIDC's identity is not recognised.
    pub affiliation: Option<Affiliation>,
    /// Affiliation before joker/faker remapping.
    pub base_affiliation: Option<Affiliation>,
    /// Frame dimension; `None` when not recognised.
    pub dimension: Option<Dimension>,
    /// Dimension before equipment/dismounted remapping.
    pub base_dimension: Option<Dimension>,
    /// The battle dimension is unknown (drawn with a question mark).
    pub dimension_unknown: bool,
    /// Context; `None` when not recognised.
    pub context: Option<Context>,
    /// Operational condition, if the SIDC encodes one.
    pub condition: Option<Status>,
    /// Planned/anticipated or pending (drawn with a dashed frame).
    pub not_present: bool,
    /// Echelon amplifier, if any.
    pub echelon: Option<Echelon>,
    /// Mobility amplifier, if any.
    pub mobility: Option<Mobility>,
    /// The echelon/mobility amplifier code is not recognised.
    pub amplifier_unknown: bool,
    /// Leadership amplifier, if any.
    pub leadership: Option<Leadership>,
    /// Edition of a numeric SIDC.
    pub edition: Option<Edition>,
    /// Base frame geometry, if the affiliation and dimension have one.
    pub geometry: Option<&'static BaseGeometry>,
    /// Headquarters.
    pub headquarters: bool,
    /// Task force.
    pub task_force: bool,
    /// Feint or dummy.
    pub feint_dummy: bool,
    /// Installation.
    pub installation: bool,
    /// Activity or event.
    pub activity: bool,
    /// Space.
    pub space: bool,
    /// Unit (as opposed to equipment).
    pub unit: bool,
    /// Land equipment.
    pub land_equipment: bool,
    /// Dismounted individual.
    pub dismounted: bool,
    /// Cyberspace.
    pub cyberspace: bool,
    /// Control measure (tactical graphic).
    pub control_measure: bool,
    /// Civilian.
    pub civilian: bool,
    /// Suspect (MIL-STD-2525E).
    pub suspect: bool,
    /// Joker.
    pub joker: bool,
    /// Faker.
    pub faker: bool,
    /// The frame is drawn.
    pub frame: bool,
    /// The frame is filled.
    pub fill: bool,
    /// Rendered with MIL-STD-2525 (rather than APP-6) rules.
    pub std2525: bool,
    /// Numeric (rather than letter) SIDC.
    pub numeric_sidc: bool,
}

impl Affiliation {
    /// Upstream's name, as used for colour-mode keys (`"Friend"`, …).
    pub fn as_str(self) -> &'static str {
        crate::metadata::Name::name(self)
    }
}

mod metadata;
