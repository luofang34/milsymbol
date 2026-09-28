//! Letter SIDCs (MIL-STD-2525B/C, APP-6B).

use super::{Code, SidcError, invalid};
use crate::catalog;
use crate::domain::{Context, StandardIdentity, Status};

/// Battle dimensions (or categories) each coding scheme defines.
fn dimensions(scheme: char) -> &'static str {
    match scheme {
        'S' => "PAGSUFXZ",
        'I' => "PAGSU",
        'O' => "VLOIPGR",
        'E' => "INOF",
        'G' => "TGMFSOC",
        'W' => "AOS",
        _ => "",
    }
}

/// Whether positions 11–12 hold a symbol modifier the scheme allows.
fn modifier_ok(scheme: char, m11: char, m12: char) -> bool {
    let echelon = |c: char| c == '-' || ('A'..='N').contains(&c);
    match scheme {
        'S' | 'I' | 'O' | 'E' => match m11 {
            '-' | 'A'..='G' => echelon(m12),
            'H' => matches!(m12, '-' | 'B'),
            'M' => ('O'..='Y').contains(&m12),
            'N' => matches!(m12, 'S' | 'L'),
            _ => false,
        },
        'G' => m11 == '-' && echelon(m12),
        // METOC: position 11 is the geometry (point, line or area).
        _ => matches!(m11, '-' | 'P' | 'L' | 'A') && m12 == '-',
    }
}

/// A letter SIDC of 10 to 15 characters (upper-cased, `*` read as `-`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LetterSidc(Code<15>);

impl LetterSidc {
    pub(super) fn parse(s: &str) -> Result<Self, SidcError> {
        let code = Code::<15>::collect(s, |c| {
            if c == '*' {
                '-'
            } else {
                c.to_ascii_uppercase()
            }
        })?;
        let len = usize::from(code.len);
        if len < 10 {
            return Err(SidcError::Length { len });
        }
        for (i, c) in code.as_str().chars().enumerate() {
            if !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-') {
                return Err(invalid("character", i + 1, code.field(i + 1, 1)));
            }
        }
        let scheme = code.at(1);
        if dimensions(scheme).is_empty() {
            return Err(invalid("coding scheme", 1, code.field(1, 1)));
        }
        if !"PUAFNSHGWMDLJKO".contains(code.at(2)) {
            return Err(invalid("standard identity", 2, code.field(2, 1)));
        }
        if !dimensions(scheme).contains(code.at(3)) {
            return Err(invalid("battle dimension", 3, code.field(3, 1)));
        }
        if !"PACDXF-".contains(code.at(4)) {
            return Err(invalid("status", 4, code.field(4, 1)));
        }
        if len >= 12 && !modifier_ok(scheme, code.at(11), code.at(12)) {
            return Err(invalid("symbol modifier", 11, code.field(11, 2)));
        }
        Ok(LetterSidc(code))
    }

    /// Coding scheme (`S` warfighting, `G` tactical graphics, `W` METOC,
    /// `I` intelligence, `O` stability operations, `E` emergency management).
    pub fn coding_scheme(&self) -> char {
        self.0.at(1)
    }

    /// Standard identity.
    pub fn standard_identity(&self) -> StandardIdentity {
        match self.0.at(2) {
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
        if "GWMDLJK".contains(self.0.at(2)) {
            Context::Exercise
        } else {
            Context::Reality
        }
    }

    /// Battle dimension code.
    pub fn battle_dimension(&self) -> char {
        self.0.at(3)
    }

    /// Status.
    pub fn status(&self) -> Status {
        match self.0.at(4) {
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
        self.0.field(5, 6)
    }

    /// Symbol modifier characters (positions 11 and 12; `'\0'` if absent).
    pub fn modifier(&self) -> (char, char) {
        (self.0.at(11), self.0.at(12))
    }

    /// Whether the built-in tables have an icon for this SIDC.
    pub fn has_builtin_icon(&self) -> bool {
        let (scheme, dim, fid) = (self.0.field(1, 1), self.0.field(3, 1), self.function_id());
        fid == "------"
            || catalog::letter_icons().any(|c| {
                let mut parts = c.splitn(3, '-');
                parts.next() == Some(scheme)
                    && parts.next() == Some(dim)
                    && parts.next() == Some(fid)
            })
    }

    /// The SIDC text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
