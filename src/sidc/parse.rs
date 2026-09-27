//! Strict, typed SIDC parsing.
//!
//! Rendering accepts any string, as milsymbol.js does. [`Sidc::parse`]
//! instead checks every field against the code tables and reports the first
//! problem with its position, for validating input before rendering.

use crate::catalog;
use crate::domain::{Context, StandardIdentity, Status};
use alloc::string::String;
use core::fmt;
use core::str::FromStr;

/// A SIDC whose fields are all well formed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Sidc {
    /// Numeric SIDC (MIL-STD-2525D/E, APP-6D/E).
    Numeric(NumericSidc),
    /// Letter SIDC (MIL-STD-2525B/C, APP-6B).
    Letter(LetterSidc),
}

/// Why a SIDC is malformed. Positions are 1-based character positions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SidcError {
    /// The SIDC is empty.
    Empty,
    /// Wrong length (numeric SIDCs have 20 or 30 digits, letter SIDCs 10–15 characters).
    Length {
        /// Actual length in characters.
        len: usize,
    },
    /// A character or field value is not allowed at this position.
    InvalidField {
        /// Field name, e.g. `"standard identity"`.
        field: &'static str,
        /// First position of the field.
        position: usize,
        /// The offending text.
        value: String,
    },
}

impl fmt::Display for SidcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SidcError::Empty => f.write_str("empty SIDC"),
            SidcError::Length { len } => write!(f, "SIDC has {len} characters"),
            SidcError::InvalidField {
                field,
                position,
                value,
            } => {
                write!(f, "invalid {field} {value:?} at position {position}")
            }
        }
    }
}

impl core::error::Error for SidcError {}

fn invalid(field: &'static str, position: usize, value: &str) -> SidcError {
    SidcError::InvalidField {
        field,
        position,
        value: String::from(value),
    }
}

/// A numeric SIDC of 20 or 30 digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NumericSidc(String);

/// A letter SIDC of 10 to 15 characters (upper-cased, `*` read as `-`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LetterSidc(String);

impl Sidc {
    /// Parses and validates `s`. Spaces are ignored, as in rendering.
    pub fn parse(s: &str) -> Result<Sidc, SidcError> {
        let s: String = s.chars().filter(|&c| c != ' ').collect();
        let first = s.chars().next().ok_or(SidcError::Empty)?;
        if first.is_ascii_digit() {
            NumericSidc::parse(&s).map(Sidc::Numeric)
        } else {
            LetterSidc::parse(&s).map(Sidc::Letter)
        }
    }

    /// The SIDC text (normalized).
    pub fn as_str(&self) -> &str {
        match self {
            Sidc::Numeric(n) => &n.0,
            Sidc::Letter(l) => &l.0,
        }
    }
}

impl FromStr for Sidc {
    type Err = SidcError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Sidc::parse(s)
    }
}

impl fmt::Display for Sidc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn digits(s: &str, from: usize, len: usize) -> &str {
    s.get(from - 1..from - 1 + len).unwrap_or("")
}

fn char_at(s: &str, position: usize) -> char {
    s.as_bytes()
        .get(position - 1)
        .map_or('\0', |&b| char::from(b))
}

const AMPLIFIERS: [&str; 30] = [
    "00", "11", "12", "13", "14", "15", "16", "17", "18", "21", "22", "23", "24", "25", "26", "31",
    "32", "33", "34", "35", "36", "37", "41", "42", "51", "52", "61", "62", "71", "72",
];

impl NumericSidc {
    fn parse(s: &str) -> Result<Self, SidcError> {
        let len = s.chars().count();
        if len != 20 && len != 30 {
            return Err(SidcError::Length { len });
        }
        for (i, c) in s.chars().enumerate() {
            if !(c.is_ascii_digit() || (i == 22 && c == 'A')) {
                return Err(invalid("character", i + 1, c.encode_utf8(&mut [0; 4])));
            }
        }
        let check = |ok: bool, field, pos, n| {
            if ok {
                Ok(())
            } else {
                Err(invalid(field, pos, digits(s, pos, n)))
            }
        };
        check(
            matches!(digits(s, 1, 2), "10" | "11" | "12" | "13" | "14"),
            "version",
            1,
            2,
        )?;
        check(matches!(char_at(s, 3), '0'..='2'), "context", 3, 1)?;
        check(
            matches!(char_at(s, 4), '0'..='6'),
            "standard identity",
            4,
            1,
        )?;
        check(
            catalog::number_symbol_sets().any(|ss| ss == digits(s, 5, 2)),
            "symbol set",
            5,
            2,
        )?;
        check(matches!(char_at(s, 7), '0'..='5'), "status", 7, 1)?;
        check(
            matches!(char_at(s, 8), '0'..='7'),
            "headquarters/task force/dummy",
            8,
            1,
        )?;
        check(
            AMPLIFIERS.contains(&digits(s, 9, 2)),
            "echelon/mobility",
            9,
            2,
        )?;
        Ok(NumericSidc(String::from(s)))
    }

    /// Version (`10`–`12`: 2525D/APP-6D, `13`–`14`: 2525E/APP-6E).
    pub fn version(&self) -> u8 {
        digits(&self.0, 1, 2).parse().unwrap_or(0)
    }

    /// Context.
    pub fn context(&self) -> Context {
        match char_at(&self.0, 3) {
            '1' => Context::Exercise,
            '2' => Context::Simulation,
            _ => Context::Reality,
        }
    }

    /// Standard identity (joker and faker in exercise context).
    pub fn standard_identity(&self) -> StandardIdentity {
        let exercise = self.context() == Context::Exercise;
        match char_at(&self.0, 4) {
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
        digits(&self.0, 5, 2)
    }

    /// Status.
    pub fn status(&self) -> Status {
        match char_at(&self.0, 7) {
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
        let d = u8::try_from(char_at(&self.0, 8))
            .unwrap_or(b'0')
            .wrapping_sub(b'0');
        (d & 2 != 0, d & 4 != 0, d & 1 != 0)
    }

    /// Two-digit echelon/mobility/leadership amplifier code.
    pub fn amplifier(&self) -> &str {
        digits(&self.0, 9, 2)
    }

    /// Six-digit entity code.
    pub fn entity(&self) -> &str {
        digits(&self.0, 11, 6)
    }

    /// Sector 1 and sector 2 modifier codes.
    pub fn modifiers(&self) -> (&str, &str) {
        (digits(&self.0, 17, 2), digits(&self.0, 19, 2))
    }

    /// Whether the built-in tables have an icon for the entity (entity
    /// `000000` means "no icon" and counts as present).
    pub fn has_builtin_icon(&self) -> bool {
        let e = self.entity();
        let base = alloc::format!("{}00", digits(e, 1, 4));
        e == "000000"
            || catalog::number_entities(self.symbol_set())
                .any(|c| c == e || (e.get(4..).is_some_and(|t| t >= "95") && c == base))
    }

    /// The SIDC text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl LetterSidc {
    fn parse(s: &str) -> Result<Self, SidcError> {
        let s: String = s
            .chars()
            .map(|c| {
                if c == '*' {
                    '-'
                } else {
                    c.to_ascii_uppercase()
                }
            })
            .collect();
        let len = s.chars().count();
        if !(10..=15).contains(&len) {
            return Err(SidcError::Length { len });
        }
        for (i, c) in s.chars().enumerate() {
            if !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-') {
                return Err(invalid("character", i + 1, c.encode_utf8(&mut [0; 4])));
            }
        }
        let field = |pos: usize| digits(&s, pos, 1);
        if !matches!(char_at(&s, 1), 'S' | 'G' | 'W' | 'I' | 'O' | 'E') {
            return Err(invalid("coding scheme", 1, field(1)));
        }
        if !"PUAFNSHGWMDLJKO".contains(char_at(&s, 2)) {
            return Err(invalid("standard identity", 2, field(2)));
        }
        if !"PACDXF-".contains(char_at(&s, 4)) {
            return Err(invalid("status", 4, field(4)));
        }
        Ok(LetterSidc(s))
    }

    /// Coding scheme (`S` warfighting, `G` tactical graphics, `W` METOC,
    /// `I` intelligence, `O` stability operations, `E` emergency management).
    pub fn coding_scheme(&self) -> char {
        char_at(&self.0, 1)
    }

    /// Standard identity.
    pub fn standard_identity(&self) -> StandardIdentity {
        match char_at(&self.0, 2) {
            'P' | 'G' => StandardIdentity::Pending,
            'U' | 'W' => StandardIdentity::Unknown,
            'A' | 'M' => StandardIdentity::AssumedFriend,
            'F' | 'D' => StandardIdentity::Friend,
            'N' | 'L' => StandardIdentity::Neutral,
            'S' => StandardIdentity::Suspect,
            'H' => StandardIdentity::Hostile,
            'J' => StandardIdentity::Joker,
            'K' => StandardIdentity::Faker,
            _ => StandardIdentity::NoneSpecified,
        }
    }

    /// Context (exercise identities `G W M D L J K`).
    pub fn context(&self) -> Context {
        if "GWMDLJK".contains(char_at(&self.0, 2)) {
            Context::Exercise
        } else {
            Context::Reality
        }
    }

    /// Battle dimension code.
    pub fn battle_dimension(&self) -> char {
        char_at(&self.0, 3)
    }

    /// Status.
    pub fn status(&self) -> Status {
        match char_at(&self.0, 4) {
            'A' => Status::Planned,
            'C' => Status::FullyCapable,
            'D' => Status::Damaged,
            'X' => Status::Destroyed,
            'F' => Status::FullToCapacity,
            _ => Status::Present,
        }
    }

    /// Six-character function identifier.
    pub fn function_id(&self) -> &str {
        self.0.get(4..10).unwrap_or("")
    }

    /// Symbol modifier characters (positions 11 and 12).
    pub fn modifier(&self) -> (char, char) {
        (char_at(&self.0, 11), char_at(&self.0, 12))
    }

    /// Whether the built-in tables have an icon for this SIDC.
    pub fn has_builtin_icon(&self) -> bool {
        let s = &self.0;
        let generic = alloc::format!(
            "{}-{}-{}",
            digits(s, 1, 1),
            digits(s, 3, 1),
            self.function_id()
        );
        self.function_id() == "------" || catalog::letter_icons().any(|c| c == generic)
    }

    /// The SIDC text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
