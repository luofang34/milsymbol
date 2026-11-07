//! Numeric context digits and their exercise/simulation component.

use super::Context;

impl Context {
    /// Reality, exercise or simulation, independently of target restrictions.
    pub fn environment(self) -> Self {
        match self {
            Self::Exercise | Self::RestrictedExercise | Self::NoStrikeExercise => Self::Exercise,
            Self::Simulation | Self::RestrictedSimulation | Self::NoStrikeSimulation => {
                Self::Simulation
            }
            _ => Self::Reality,
        }
    }

    /// Whether this context requires the restricted-target amplifier.
    pub fn restricted(self) -> bool {
        matches!(
            self,
            Self::RestrictedReality | Self::RestrictedExercise | Self::RestrictedSimulation
        )
    }

    /// Whether this context requires the no-strike amplifier.
    pub fn no_strike(self) -> bool {
        matches!(
            self,
            Self::NoStrikeReality | Self::NoStrikeExercise | Self::NoStrikeSimulation
        )
    }

    /// Context digit in an edition E numeric SIDC.
    pub fn digit(self) -> char {
        match self {
            Self::Reality => '0',
            Self::Exercise => '1',
            Self::Simulation => '2',
            Self::RestrictedReality => '3',
            Self::NoStrikeReality => '4',
            Self::RestrictedExercise => '5',
            Self::NoStrikeExercise => '6',
            Self::RestrictedSimulation => '7',
            Self::NoStrikeSimulation => '8',
        }
    }

    pub(crate) fn from_digit(digit: usize) -> Option<Self> {
        [
            Self::Reality,
            Self::Exercise,
            Self::Simulation,
            Self::RestrictedReality,
            Self::NoStrikeReality,
            Self::RestrictedExercise,
            Self::NoStrikeExercise,
            Self::RestrictedSimulation,
            Self::NoStrikeSimulation,
        ]
        .get(digit)
        .copied()
    }
}
