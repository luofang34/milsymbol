//! Typed symbology values.
//!
//! [`SymbolInfo`] is the typed view of a rendered symbol; the string-valued
//! [`Metadata`] remains available as the milsymbol.js-compatible view
//! (including its `"undefined"` sentinels).

use crate::metadata::Metadata;

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

/// Typed description of a rendered symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolInfo {
    /// Frame affiliation; `None` when the SIDC's identity is not recognised.
    pub affiliation: Option<Affiliation>,
    /// Frame dimension; `None` when not recognised.
    pub dimension: Option<Dimension>,
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
    /// Leadership amplifier, if any.
    pub leadership: Option<Leadership>,
    /// Headquarters.
    pub headquarters: bool,
    /// Task force.
    pub task_force: bool,
    /// Feint or dummy.
    pub feint_dummy: bool,
    /// Installation.
    pub installation: bool,
    /// Civilian.
    pub civilian: bool,
    /// Joker.
    pub joker: bool,
    /// Faker.
    pub faker: bool,
    /// Rendered with MIL-STD-2525 (rather than APP-6) rules.
    pub std2525: bool,
    /// Numeric (rather than letter) SIDC.
    pub numeric_sidc: bool,
}

impl SymbolInfo {
    /// Typed view of upstream-compatible metadata.
    pub fn from_metadata(md: &Metadata) -> Self {
        let affiliation = match md.affiliation.as_deref() {
            Some("Friend") => Some(Affiliation::Friend),
            Some("Hostile") => Some(Affiliation::Hostile),
            Some("Neutral") => Some(Affiliation::Neutral),
            Some("Unknown") => Some(Affiliation::Unknown),
            _ => None,
        };
        let dimension = match md.dimension.as_str() {
            "Air" => Some(Dimension::Air),
            "Ground" => Some(Dimension::Ground),
            "Sea" => Some(Dimension::Sea),
            "Subsurface" => Some(Dimension::Subsurface),
            "LandDismountedIndividual" => Some(Dimension::LandDismountedIndividual),
            _ => None,
        };
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
        SymbolInfo {
            affiliation,
            dimension,
            context,
            condition,
            not_present: !md.notpresent.is_empty(),
            echelon: md.echelon.as_deref().and_then(Echelon::from_name),
            mobility: md.mobility.as_deref().and_then(Mobility::from_name),
            leadership,
            headquarters: md.headquarters,
            task_force: md.task_force,
            feint_dummy: md.flags.feint_dummy == Some(true),
            installation: md.installation,
            civilian: md.civilian,
            joker: md.joker,
            faker: md.faker,
            std2525: md.std2525,
            numeric_sidc: md.number_sidc,
        }
    }
}
