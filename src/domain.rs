//! Typed symbology values and [`Metadata`], the typed description of a
//! rendered symbol. milsymbol.js's string-valued metadata is available as
//! [`compat::JsMetadata`](crate::compat::JsMetadata).

use crate::geometry::BaseGeometry;
use crate::metadata::Metadata as JsMetadata;

/// Standard identity (affiliation code of the SIDC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub(crate) fn from_name(s: &str) -> Option<Self> {
        use Echelon::*;
        Some(match s {
            "Team/Crew" => TeamCrew,
            "Squad" => Squad,
            "Section" => Section,
            "Platoon/detachment" => PlatoonDetachment,
            "Company/battery/troop" => CompanyBatteryTroop,
            "Battalion/squadron" => BattalionSquadron,
            "Regiment/group" => RegimentGroup,
            "Brigade" => Brigade,
            "Division" => Division,
            "Corps/MEF" => CorpsMef,
            "Army" => Army,
            "Army Group/front" => ArmyGroupFront,
            "Region/Theater" => RegionTheater,
            "Command" => Command,
            _ => return None,
        })
    }
}

impl Mobility {
    pub(crate) fn from_name(s: &str) -> Option<Self> {
        use Mobility::*;
        Some(match s {
            "Wheeled limited cross country" => WheeledLimitedCrossCountry,
            "Wheeled cross country" => WheeledCrossCountry,
            "Tracked" => Tracked,
            "Wheeled and tracked combination" => WheeledAndTracked,
            "Towed" => Towed,
            "Rail" => Rail,
            "Pack animals" => PackAnimals,
            "Over snow (prime mover)" => OverSnow,
            "Sled" => Sled,
            "Barge" => Barge,
            "Amphibious" => Amphibious,
            "Short towed array" => ShortTowedArray,
            "Long towed Array" => LongTowedArray,
            _ => return None,
        })
    }
}

/// Edition of the standard a numeric SIDC follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        match self {
            Affiliation::Friend => "Friend",
            Affiliation::Hostile => "Hostile",
            Affiliation::Neutral => "Neutral",
            Affiliation::Unknown => "Unknown",
        }
    }

    fn from_name(s: Option<&str>) -> Option<Self> {
        Some(match s? {
            "Friend" => Affiliation::Friend,
            "Hostile" => Affiliation::Hostile,
            "Neutral" => Affiliation::Neutral,
            "Unknown" => Affiliation::Unknown,
            _ => return None,
        })
    }
}

impl Dimension {
    fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "Air" => Dimension::Air,
            "Ground" => Dimension::Ground,
            "Sea" => Dimension::Sea,
            "Subsurface" => Dimension::Subsurface,
            "LandDismountedIndividual" => Dimension::LandDismountedIndividual,
            _ => return None,
        })
    }
}

impl Metadata {
    /// Typed view of milsymbol.js-compatible metadata.
    pub fn from_js(md: &JsMetadata) -> Self {
        let context = match md.context.as_deref() {
            Some("Reality") => Some(Context::Reality),
            Some("Exercise") => Some(Context::Exercise),
            Some("Simulation") => Some(Context::Simulation),
            _ => None,
        };
        let condition = match md.condition.as_str() {
            "FullyCapable" => Some(Status::FullyCapable),
            "Damaged" => Some(Status::Damaged),
            "Destroyed" => Some(Status::Destroyed),
            "FullToCapacity" => Some(Status::FullToCapacity),
            _ => None,
        };
        let leadership = match md.flags.leadership.as_deref() {
            Some("Leader Individual") => Some(Leadership::Leader),
            Some("Deputy Individual") => Some(Leadership::Deputy),
            _ => None,
        };
        let edition = match md.edition() {
            Some("D") => Some(Edition::D),
            Some("E") => Some(Edition::E),
            _ => None,
        };
        let flag = |f: Option<bool>| f == Some(true);
        Metadata {
            affiliation: Affiliation::from_name(md.affiliation.as_deref()),
            base_affiliation: Affiliation::from_name(md.base_affiliation.as_deref()),
            dimension: Dimension::from_name(&md.dimension),
            base_dimension: Dimension::from_name(&md.base_dimension),
            dimension_unknown: md.dimension_unknown,
            context,
            condition,
            not_present: !md.notpresent.is_empty(),
            echelon: md.echelon.as_deref().and_then(Echelon::from_name),
            mobility: md.mobility.as_deref().and_then(Mobility::from_name),
            amplifier_unknown: md.mobility.is_none(),
            leadership,
            edition,
            geometry: md.geometry(),
            headquarters: md.headquarters,
            task_force: md.task_force,
            feint_dummy: flag(md.flags.feint_dummy),
            installation: md.installation,
            activity: md.activity,
            space: md.space,
            unit: md.unit,
            land_equipment: flag(md.flags.landequipment),
            dismounted: flag(md.flags.dismounted),
            cyberspace: flag(md.flags.cyberspace),
            control_measure: flag(md.flags.control_measure),
            civilian: md.civilian,
            suspect: flag(md.flags.suspect),
            joker: md.joker,
            faker: md.faker,
            frame: md.frame,
            fill: md.fill,
            std2525: md.std2525,
            numeric_sidc: md.number_sidc,
        }
    }
}
