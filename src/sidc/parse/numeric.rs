//! Numeric SIDCs (MIL-STD-2525D/E, APP-6D/E).

use super::{Code, SidcError, SidcModifyError, invalid};
use crate::catalog;
use crate::domain::{Affiliation, Context, StandardIdentity, Status};

/// Symbol sets the standards define, plus those milsymbol.js interprets
/// (`12` and `39`). Whether a renderer has icons for them is separate.
const SYMBOL_SETS: [&str; 26] = [
    "00", "01", "02", "05", "06", "10", "11", "12", "15", "20", "25", "27", "30", "35", "36", "39",
    "40", "45", "46", "47", "50", "51", "52", "53", "54", "60",
];

const AMPLIFIERS: [&str; 30] = [
    "00", "11", "12", "13", "14", "15", "16", "17", "18", "21", "22", "23", "24", "25", "26", "31",
    "32", "33", "34", "35", "36", "37", "41", "42", "51", "52", "61", "62", "71", "72",
];

/// A complete three-digit modifier code (see [`NumericSidc::modifier_codes`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModifierCode([u8; 3]);

impl ModifierCode {
    /// The code as text, e.g. `"107"`.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.0).unwrap_or("000")
    }

    /// The modifier-set digit (`0` for the symbol set's own modifiers).
    pub fn set(&self) -> char {
        let [set, _, _] = self.0;
        char::from(set)
    }
}

impl core::fmt::Display for ModifierCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A numeric SIDC of 20 or 30 digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NumericSidc(Code<30>);

impl NumericSidc {
    pub(super) fn parse(
        s: &str,
        standard: crate::Standard,
        mode: crate::RenderMode,
    ) -> Result<Self, SidcError> {
        let code = Code::<30>::collect(s, |c| c)?;
        let len = usize::from(code.len);
        if len != 20 && len != 30 {
            return Err(SidcError::Length { len });
        }
        let standards = mode == crate::RenderMode::Standards;
        let edition_e = matches!(code.field(1, 2), "13" | "14" | "15" | "16");
        let modern = standards && matches!(code.field(1, 2), "15" | "16");
        let hex = standards && edition_e;
        for (i, c) in code.as_str().chars().enumerate() {
            // Edition E uses hexadecimal characters; header fields below
            // still accept only the values assigned to that field.
            let ok = c.is_ascii_digit()
                || (hex && matches!(c, 'A'..='F'))
                || (!hex && i == 22 && c == 'A');
            if !ok {
                return Err(invalid("character", i + 1, code.field(i + 1, 1)));
            }
        }
        let check = |ok: bool, field, pos, n| {
            if ok {
                Ok(())
            } else {
                Err(invalid(field, pos, code.field(pos, n)))
            }
        };
        check(
            modern || matches!(code.field(1, 2), "10" | "11" | "12" | "13" | "14"),
            "version",
            1,
            2,
        )?;
        check(
            matches!(code.at(3), '0'..='2') || (hex && matches!(code.at(3), '3'..='8')),
            "context",
            3,
            1,
        )?;
        check(matches!(code.at(4), '0'..='6'), "standard identity", 4, 1)?;
        // Neither pinned edition assigns symbol sets 12 and 39.
        let ss = code.field(5, 2);
        check(
            (SYMBOL_SETS.contains(&ss) || (hex && ss == "64"))
                && !(modern && matches!(ss, "12" | "39")),
            "symbol set",
            5,
            2,
        )?;
        check(matches!(code.at(7), '0'..='5'), "status", 7, 1)?;
        check(
            matches!(code.at(8), '0'..='7'),
            "headquarters/task force/dummy",
            8,
            1,
        )?;
        // APP-6(E) Version 2 Table A-8 defines leadership code 71, but not 72.
        let amplifier = code.field(9, 2);
        check(
            AMPLIFIERS.contains(&amplifier) && !(modern && amplifier == "72"),
            "echelon/mobility",
            9,
            2,
        )?;
        // Reserved in Version 2 (A.14, p. 723).
        check(
            !(standards && standard != crate::Standard::Mil2525)
                || matches!(code.field(23, 5), "" | "00000"),
            "reserved",
            23,
            5,
        )?;
        if standards && standard == crate::Standard::Mil2525 {
            check(
                matches!(code.at(23), '\0' | '0'..='9' | 'A'),
                "frame shape",
                23,
                1,
            )?;
            check(matches!(code.field(24, 4), "" | "0000"), "reserved", 24, 4)?;
        }
        Ok(NumericSidc(code))
    }

    /// Version: `10`–`12` edition D, `13` MIL-STD-2525E, `14` APP-6(E)(1),
    /// `15` MIL-STD-2525E Change 1, `16` APP-6(E)(2).
    pub fn version(&self) -> u8 {
        self.0.field(1, 2).parse().unwrap_or(0)
    }

    /// Context.
    pub fn context(&self) -> Context {
        Context::from_digit(self.0.at(3).to_digit(10).unwrap_or(0) as usize)
            .unwrap_or(Context::Reality)
    }

    /// Standard identity (joker and faker in exercise context).
    pub fn standard_identity(&self) -> StandardIdentity {
        let exercise = self.context().environment() == Context::Exercise;
        match self.0.at(4) {
            '0' => StandardIdentity::Pending,
            '1' => StandardIdentity::Unknown,
            '2' => StandardIdentity::AssumedFriend,
            '3' => StandardIdentity::Friend,
            '4' => StandardIdentity::Neutral,
            '5' if exercise => StandardIdentity::Joker,
            '5' => StandardIdentity::Suspect,
            _ if exercise => StandardIdentity::Faker,
            _ => StandardIdentity::Hostile,
        }
    }

    /// Two-digit symbol set.
    pub fn symbol_set(&self) -> &str {
        self.0.field(5, 2)
    }

    /// Status.
    pub fn status(&self) -> Status {
        match self.0.at(7) {
            '1' => Status::Planned,
            '2' => Status::FullyCapable,
            '3' => Status::Damaged,
            '4' => Status::Destroyed,
            '5' => Status::FullToCapacity,
            _ => Status::Present,
        }
    }

    /// Headquarters, task force and feint/dummy flags.
    pub fn hq_task_force_dummy(&self) -> (bool, bool, bool) {
        let d = u8::try_from(self.0.at(8))
            .unwrap_or(b'0')
            .wrapping_sub(b'0');
        (d & 2 != 0, d & 4 != 0, d & 1 != 0)
    }

    /// Two-digit echelon/mobility/leadership amplifier code.
    pub fn amplifier(&self) -> &str {
        self.0.field(9, 2)
    }

    /// Six-digit entity code.
    pub fn entity(&self) -> &str {
        self.0.field(11, 6)
    }

    /// The raw two-digit sector 1 and sector 2 modifier fields (positions
    /// 17–18 and 19–20). See [`NumericSidc::modifier_codes`] for the codes
    /// including the modifier set of a 30-digit SIDC.
    pub fn modifiers(&self) -> (&str, &str) {
        (self.0.field(17, 2), self.0.field(19, 2))
    }

    /// The complete sector 1 and sector 2 modifier codes: the modifier-set
    /// digit (position 21 or 22 of a 30-digit SIDC, `0` otherwise) followed
    /// by the two-digit field, e.g. `"107"`. These are the codes milsymbol.js
    /// looks icons up by.
    pub fn modifier_codes(&self) -> (ModifierCode, ModifierCode) {
        let code = |set: usize, field: usize| {
            let digit = |c: char| u8::try_from(c).unwrap_or(b'0');
            let set = match self.0.at(set) {
                '\0' => b'0',
                c => digit(c),
            };
            let f = self.0.field(field, 2).as_bytes();
            ModifierCode([
                set,
                f.first().copied().unwrap_or(b'0'),
                f.get(1).copied().unwrap_or(b'0'),
            ])
        };
        (code(21, 17), code(22, 19))
    }

    /// Whether the built-in tables have an icon for the entity (entity
    /// `000000` means "no icon" and counts as present).
    pub fn has_builtin_icon(&self) -> bool {
        let e = self.entity();
        let version = self.version();
        if matches!(version, 15 | 16) || (version >= 13 && self.symbol_set() == "64") {
            use catalog::standards::{CodeKind, Scope, entry};
            let version = if self.symbol_set() == "64" {
                16
            } else {
                version
            };
            return e == "000000"
                || entry(version, self.symbol_set(), CodeKind::Entity, e)
                    .is_some_and(|e| e.scope != Scope::Multipoint);
        }
        let head = e.get(..4).unwrap_or("");
        let headquarters_variant = e.get(4..).is_some_and(|t| t >= "95");
        e == "000000"
            || catalog::number_entities(self.symbol_set()).any(|c| {
                c == e
                    || (headquarters_variant
                        && c.get(..4) == Some(head)
                        && c.get(4..) == Some("00"))
            })
    }

    /// The SIDC text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The same SIDC with another status.
    #[must_use]
    pub fn with_status(&self, status: Status) -> Self {
        let digit = match status {
            Status::Present => '0',
            Status::Planned => '1',
            Status::FullyCapable => '2',
            Status::Damaged => '3',
            Status::Destroyed => '4',
            Status::FullToCapacity => '5',
        };
        NumericSidc(self.0.with_char(7, digit))
    }

    /// The same SIDC in another context. Restricted and no-strike contexts
    /// require an edition E version.
    /// The identity digit is kept, so Suspect and Hostile become Joker and
    /// Faker in an exercise and back.
    pub fn with_context(&self, context: Context) -> Result<Self, SidcModifyError> {
        if self.version() < 13 && (context.restricted() || context.no_strike()) {
            return Err(SidcModifyError::UnsupportedContext { context });
        }
        let digit = context.digit();
        Ok(NumericSidc(self.0.with_char(3, digit)))
    }

    /// The same SIDC with another standard identity; fails when the SIDC's
    /// context has no code for it (see [`SidcModifyError`]).
    pub fn with_standard_identity(
        &self,
        identity: StandardIdentity,
    ) -> Result<Self, SidcModifyError> {
        let context = self.context();
        let exercise = context.environment() == Context::Exercise;
        let digit = match (identity, exercise) {
            (StandardIdentity::Pending, _) => '0',
            (StandardIdentity::Unknown, _) => '1',
            (StandardIdentity::AssumedFriend, _) => '2',
            (StandardIdentity::Friend, _) => '3',
            (StandardIdentity::Neutral, _) => '4',
            (StandardIdentity::Suspect, false) | (StandardIdentity::Joker, true) => '5',
            (StandardIdentity::Hostile, false) | (StandardIdentity::Faker, true) => '6',
            _ => return Err(SidcModifyError::Unrepresentable { identity, context }),
        };
        Ok(NumericSidc(self.0.with_char(4, digit)))
    }

    /// The same SIDC with another affiliation, using the identity code of
    /// its context (Hostile in an exercise is Faker).
    #[must_use]
    pub fn with_affiliation(&self, affiliation: Affiliation) -> Self {
        let digit = match affiliation {
            Affiliation::Unknown => '1',
            Affiliation::Friend => '3',
            Affiliation::Neutral => '4',
            Affiliation::Hostile => '6',
        };
        NumericSidc(self.0.with_char(4, digit))
    }
}
