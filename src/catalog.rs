//! Enumeration of the built-in icon codes.
//!
//! Lists what the generated tables define, e.g. to populate a symbol picker
//! or to render every supported icon. A listed code may still render as
//! invalid for some standard/edition combinations, as in upstream.

use crate::generated::tables;

/// Two-digit numeric symbol sets that define at least one main icon.
pub fn number_symbol_sets() -> impl Iterator<Item = &'static str> {
    (0..100u8)
        .filter(|&ss| !main_table(ss).is_empty())
        .map(symbol_set_name)
}

/// `"00"`–`"99"`.
const SYMBOL_SET_NAMES: [&str; 100] = [
    "00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15",
    "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31",
    "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45", "46", "47",
    "48", "49", "50", "51", "52", "53", "54", "55", "56", "57", "58", "59", "60", "61", "62", "63",
    "64", "65", "66", "67", "68", "69", "70", "71", "72", "73", "74", "75", "76", "77", "78", "79",
    "80", "81", "82", "83", "84", "85", "86", "87", "88", "89", "90", "91", "92", "93", "94", "95",
    "96", "97", "98", "99",
];

fn symbol_set_name(ss: u8) -> &'static str {
    SYMBOL_SET_NAMES.get(usize::from(ss)).copied().unwrap_or("")
}

fn main_table(ss: u8) -> &'static [(&'static str, u32)] {
    tables::NUMBER
        .get(usize::from(ss))
        .and_then(|kinds| kinds.first())
        .copied()
        .unwrap_or(&[])
}

fn number_table(symbol_set: &str, kind: usize) -> &'static [(&'static str, u32)] {
    symbol_set
        .parse::<usize>()
        .ok()
        .filter(|_| symbol_set.len() == 2)
        .and_then(|i| tables::NUMBER.get(i))
        .and_then(|kinds| kinds.get(kind))
        .copied()
        .unwrap_or(&[])
}

/// Six-digit entity codes with a main icon in `symbol_set`.
pub fn number_entities(symbol_set: &str) -> impl Iterator<Item = &'static str> {
    number_table(symbol_set, 0).iter().map(|(code, _)| *code)
}

/// Sector 1 modifier codes of `symbol_set`.
pub fn number_modifier1(symbol_set: &str) -> impl Iterator<Item = &'static str> {
    number_table(symbol_set, 1).iter().map(|(code, _)| *code)
}

/// Sector 2 modifier codes of `symbol_set`.
pub fn number_modifier2(symbol_set: &str) -> impl Iterator<Item = &'static str> {
    number_table(symbol_set, 2).iter().map(|(code, _)| *code)
}

/// Generic letter SIDCs (e.g. `S-G-UCI---`) with an icon.
pub fn letter_icons() -> impl Iterator<Item = &'static str> {
    tables::LETTER_ICONS.iter().map(|(code, _)| *code)
}

/// Names of the built-in icon parts usable from extensions.
pub fn icon_parts() -> impl Iterator<Item = &'static str> {
    tables::PARTS.iter().map(|(name, _)| *name)
}
