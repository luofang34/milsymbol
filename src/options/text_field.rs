//! Text amplifier fields.

use crate::ir::Str;
#[allow(unused_imports)]
use alloc::string::String;

macro_rules! text_fields {
    ($($(#[$doc:meta])* $variant:ident, $konst:ident => $name:literal,)*) => {
        /// Option names as constants, for crate-internal lookups.
        pub(crate) mod field {
            $(pub(crate) const $konst: &str = $name;)*
        }

        /// A text amplifier of a symbol.
        ///
        /// Each field has the option name milsymbol.js uses
        /// ([`TextField::name`]). Label overrides can read further fields;
        /// [`TextField::custom`] names those.
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(from = "String", into = "String"))]
        #[non_exhaustive]
        pub enum TextField {
            $($(#[$doc])* $variant,)*
            /// A field outside the standard set (see [`TextField::custom`]).
            Custom(CustomField),
        }

        impl TextField {
            /// The standard fields, in milsymbol.js order.
            pub const STANDARD: &'static [TextField] = &[$(TextField::$variant,)*];

            /// The milsymbol.js option name of the field.
            pub fn name(&self) -> &str {
                match self {
                    $(TextField::$variant => field::$konst,)*
                    TextField::Custom(c) => &c.0,
                }
            }

            fn standard_named(name: &str) -> Option<TextField> {
                match name {
                    $(field::$konst => Some(TextField::$variant),)*
                    _ => None,
                }
            }
        }
    };
}

/// The name of a custom text field. Built by [`TextField::custom`], which
/// keeps a standard field's name from becoming a custom field.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CustomField(Str);

text_fields! {
    /// Field C: quantity.
    Quantity, QUANTITY => "quantity",
    /// Field F: reinforced or reduced.
    ReinforcedReduced, REINFORCED_REDUCED => "reinforcedReduced",
    /// Field G: staff comments.
    StaffComments, STAFF_COMMENTS => "staffComments",
    /// Field H: additional information.
    AdditionalInformation, ADDITIONAL_INFORMATION => "additionalInformation",
    /// Field J: evaluation rating.
    EvaluationRating, EVALUATION_RATING => "evaluationRating",
    /// Field K: combat effectiveness.
    CombatEffectiveness, COMBAT_EFFECTIVENESS => "combatEffectiveness",
    /// Field L: signature equipment.
    SignatureEquipment, SIGNATURE_EQUIPMENT => "signatureEquipment",
    /// Field M: higher formation.
    HigherFormation, HIGHER_FORMATION => "higherFormation",
    /// Field N: hostile (enemy).
    Hostile, HOSTILE => "hostile",
    /// Field P: IFF/SIF.
    IffSif, IFF_SIF => "iffSif",
    /// Field R2: signals intelligence.
    Sigint, SIGINT => "sigint",
    /// Field T: unique designation.
    UniqueDesignation, UNIQUE_DESIGNATION => "uniqueDesignation",
    /// Field V: type.
    Type, TYPE => "type",
    /// Field W: date-time group.
    Dtg, DTG => "dtg",
    /// Field X: altitude or depth.
    AltitudeDepth, ALTITUDE_DEPTH => "altitudeDepth",
    /// Field Y: location.
    Location, LOCATION => "location",
    /// Field Z: speed.
    Speed, SPEED => "speed",
    /// Field AA: special headquarters.
    SpecialHeadquarters, SPECIAL_HEADQUARTERS => "specialHeadquarters",
    /// Country: APP-6(E) field AS, the geographical entity (milsymbol.js
    /// documents it as field AC).
    Country, COUNTRY => "country",
    /// Field AD: platform type.
    PlatformType, PLATFORM_TYPE => "platformType",
    /// Field AE: equipment teardown time.
    EquipmentTeardownTime, EQUIPMENT_TEARDOWN_TIME => "equipmentTeardownTime",
    /// Field AF: common identifier.
    CommonIdentifier, COMMON_IDENTIFIER => "commonIdentifier",
    /// Field AG: auxiliary equipment indicator.
    AuxiliaryEquipmentIndicator, AUXILIARY_EQUIPMENT_INDICATOR => "auxiliaryEquipmentIndicator",
    /// Headquarters element: APP-6(E) field AW (milsymbol.js documents it as
    /// field AH).
    HeadquartersElement, HEADQUARTERS_ELEMENT => "headquartersElement",
    /// Installation composition: APP-6(E) field AX (milsymbol.js documents it
    /// as field AI).
    InstallationComposition, INSTALLATION_COMPOSITION => "installationComposition",
    /// Field AO: engagement bar.
    EngagementBar, ENGAGEMENT_BAR => "engagementBar",
    /// Engagement bar type: `TARGET`, `NON-TARGET` or `EXPIRED`.
    EngagementType, ENGAGEMENT_TYPE => "engagementType",
    /// Field AQ: guarded unit.
    GuardedUnit, GUARDED_UNIT => "guardedUnit",
    /// Field AR: special designator.
    SpecialDesignator, SPECIAL_DESIGNATOR => "specialDesignator",
}

impl TextField {
    /// The field with this option name. Names outside the standard set give
    /// a custom field.
    pub fn from_name(name: &str) -> TextField {
        TextField::standard_named(name)
            .unwrap_or_else(|| TextField::custom(alloc::string::String::from(name)))
    }

    /// A field outside the standard set, such as `dtg1` for label overrides.
    /// A standard field's name gives that standard field. The name is a
    /// `String` or a `&'static str`; a borrowed `&String` does not compile, so
    /// pass `name.clone()`.
    pub fn custom(name: impl Into<Str>) -> TextField {
        let name = name.into();
        TextField::standard_named(&name).unwrap_or(TextField::Custom(CustomField(name)))
    }
}

impl From<&str> for TextField {
    fn from(name: &str) -> Self {
        TextField::from_name(name)
    }
}

impl From<&alloc::string::String> for TextField {
    fn from(name: &alloc::string::String) -> Self {
        TextField::from_name(name)
    }
}

impl From<alloc::string::String> for TextField {
    fn from(name: alloc::string::String) -> Self {
        TextField::custom(name)
    }
}

impl From<TextField> for alloc::string::String {
    fn from(f: TextField) -> alloc::string::String {
        alloc::string::String::from(f.name())
    }
}
