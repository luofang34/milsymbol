//! Closed-domain values plus the string sentinels needed for oracle parity.

use crate::domain::{
    Affiliation, Context, Dimension, Echelon, Edition, Leadership, Mobility, Status,
};

pub(crate) trait Name: Copy {
    fn name(self) -> &'static str;
}

/// Missing is JavaScript `undefined`; Undefined and None are literal strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Field<T> {
    Known(T),
    Missing,
    Empty,
    Undefined,
    None,
}

impl<T: Name> Field<T> {
    pub(crate) fn known(self) -> Option<T> {
        match self {
            Self::Known(v) => Some(v),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> Option<&'static str> {
        match self {
            Self::Known(v) => Some(v.name()),
            Self::Missing => None,
            Self::Empty => Some(""),
            Self::Undefined => Some("undefined"),
            Self::None => Some("none"),
        }
    }
}

impl<T> From<Option<T>> for Field<T> {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Missing, Self::Known)
    }
}

impl Name for Echelon {
    fn name(self) -> &'static str {
        match self {
            Self::TeamCrew => "Team/Crew",
            Self::Squad => "Squad",
            Self::Section => "Section",
            Self::PlatoonDetachment => "Platoon/detachment",
            Self::CompanyBatteryTroop => "Company/battery/troop",
            Self::BattalionSquadron => "Battalion/squadron",
            Self::RegimentGroup => "Regiment/group",
            Self::Brigade => "Brigade",
            Self::Division => "Division",
            Self::CorpsMef => "Corps/MEF",
            Self::Army => "Army",
            Self::ArmyGroupFront => "Army Group/front",
            Self::RegionTheater => "Region/Theater",
            Self::Command => "Command",
        }
    }
}

impl Name for Mobility {
    fn name(self) -> &'static str {
        match self {
            Self::WheeledLimitedCrossCountry => "Wheeled limited cross country",
            Self::WheeledCrossCountry => "Wheeled cross country",
            Self::Tracked => "Tracked",
            Self::WheeledAndTracked => "Wheeled and tracked combination",
            Self::Towed => "Towed",
            Self::Rail => "Rail",
            Self::PackAnimals => "Pack animals",
            Self::OverSnow => "Over snow (prime mover)",
            Self::Sled => "Sled",
            Self::Barge => "Barge",
            Self::Amphibious => "Amphibious",
            Self::ShortTowedArray => "Short towed array",
            Self::LongTowedArray => "Long towed Array",
        }
    }
}

impl Name for Affiliation {
    fn name(self) -> &'static str {
        match self {
            Self::Friend => "Friend",
            Self::Hostile => "Hostile",
            Self::Neutral => "Neutral",
            Self::Unknown => "Unknown",
        }
    }
}

impl Name for Dimension {
    fn name(self) -> &'static str {
        match self {
            Self::Air => "Air",
            Self::Ground => "Ground",
            Self::Sea => "Sea",
            Self::Subsurface => "Subsurface",
            Self::LandDismountedIndividual => "LandDismountedIndividual",
        }
    }
}

impl Name for Context {
    fn name(self) -> &'static str {
        match self {
            Self::Reality => "Reality",
            Self::Exercise => "Exercise",
            Self::Simulation => "Simulation",
        }
    }
}

impl Name for Status {
    fn name(self) -> &'static str {
        match self {
            Self::Present => "Present",
            Self::Planned => "Planned",
            Self::FullyCapable => "FullyCapable",
            Self::Damaged => "Damaged",
            Self::Destroyed => "Destroyed",
            Self::FullToCapacity => "FullToCapacity",
        }
    }
}

impl Name for Edition {
    fn name(self) -> &'static str {
        match self {
            Self::D => "D",
            Self::E => "E",
        }
    }
}

impl Name for Leadership {
    fn name(self) -> &'static str {
        match self {
            Self::Leader => "Leader Individual",
            Self::Deputy => "Deputy Individual",
        }
    }
}
