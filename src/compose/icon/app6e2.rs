//! Which built-in icons a version `16` SIDC (APP-6(E) Version 2, drawn with
//! [`Standard::App6EVersion2`](crate::Standard::App6EVersion2)) may use.
//!
//! The built-in tables are milsymbol.js's, which merge several editions of
//! both standards. A code draws its built-in icon only when Annex A of
//! Version 2 lists it and the icon shows the meaning Version 2 gives it;
//! every other code draws as unknown.

mod codes;

/// The codes of one symbol set.
pub(super) struct Set {
    pub symbol_set: &'static str,
    /// Six-digit entity codes, ascending.
    pub entities: &'static [u32],
    /// Two-digit sector 1 modifier codes, ascending.
    pub modifier1: &'static [&'static str],
    /// Two-digit sector 2 modifier codes, ascending.
    pub modifier2: &'static [&'static str],
}

/// The part of the SIDC a code fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Entity,
    Modifier1,
    Modifier2,
}

/// Symbol sets Table A-4 (p. 715) marks as used by the US only. Version 2
/// lists no codes for them, so they draw as MIL-STD-2525E.
const US_ONLY: [&str; 9] = ["06", "45", "46", "47", "50", "51", "52", "53", "54"];

/// Codes Version 2 lists that the built-in tables, under APP-6 rules, draw
/// with another meaning (modifier codes with their modifier-set digit).
const OTHER_MEANING: [(&str, Kind, &str); 12] = [
    // Table A-21 (pp. 742–743): tactical satellite communications, mobility
    // assault, amphibious warfare ship, load handling and palletised load
    // systems. The APP-6 icons are APP-6(D)'s airmobile, accident, other,
    // civilian and antisubmarine warfare.
    ("10", Kind::Modifier1, "001"),
    ("10", Kind::Modifier1, "071"),
    ("10", Kind::Modifier1, "072"),
    ("10", Kind::Modifier1, "073"),
    ("10", Kind::Modifier1, "074"),
    // Table A-30 (p. 761): displaced persons and training, drawn as joint
    // network node and command post node.
    ("20", Kind::Modifier1, "015"),
    ("20", Kind::Modifier1, "016"),
    // Table A-37 (p. 797): functional staff area J10, drawn as mountain.
    ("27", Kind::Modifier2, "013"),
    // Table A-32 and Table 8-12 (p. 490): harbour entrance point O, drawn
    // with the letter Q.
    ("25", Kind::Entity, "212902"),
    // Table A-45 (pp. 809–813): an organized group, drawn as a meeting.
    ("40", Kind::Entity, "131000"),
    // Table A-48 (p. 815): criminal, drawn as advanced persistent threat.
    ("60", Kind::Entity, "120100"),
    // Table A-52 (p. 818): external defence measures, which the built-in
    // tables define without an icon.
    ("60", Kind::Modifier2, "004"),
];

/// Whether the built-in icon for `code` may be drawn: an entity code, or a
/// three-digit modifier code (`0` and the symbol set's own two digits, or a
/// common modifier).
pub(super) fn draws(symbol_set: &str, kind: Kind, code: &str) -> bool {
    if US_ONLY.contains(&symbol_set) {
        return true;
    }
    listed(symbol_set, kind, code)
        && !OTHER_MEANING
            .iter()
            .any(|&(ss, k, c)| ss == symbol_set && k == kind && c == code)
}

fn listed(symbol_set: &str, kind: Kind, code: &str) -> bool {
    let Some(set) = codes::SETS.iter().find(|s| s.symbol_set == symbol_set) else {
        return false;
    };
    let own = |table: &[&str]| {
        code.strip_prefix('0')
            .is_some_and(|c| table.binary_search(&c).is_ok())
    };
    let common = |table: &[&str]| code.starts_with('1') && table.binary_search(&code).is_ok();
    match kind {
        Kind::Entity => entity(code).is_some_and(|e| set.entities.binary_search(&e).is_ok()),
        Kind::Modifier1 => own(set.modifier1) || common(&codes::COMMON_MODIFIER1),
        Kind::Modifier2 => own(set.modifier2) || common(&codes::COMMON_MODIFIER2),
    }
}

/// A six-digit entity code as a number.
fn entity(code: &str) -> Option<u32> {
    if code.len() == 6 && code.bytes().all(|b| b.is_ascii_digit()) {
        code.parse().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
