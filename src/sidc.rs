//! SIDC interpretation: numeric (2525D/E, APP-6D/E) and legacy letter
//! (2525B/C, APP-6B) symbol identification codes.
//!
//! Parsing never fails: like upstream, malformed codes produce metadata that
//! marks the symbol invalid rather than an error.

use crate::js;
use crate::metadata::Metadata;
use alloc::string::String;

mod letter;
mod number;

/// Upstream `mapping.echelonMobility`.
pub(crate) fn echelon_mobility(code: &str) -> Option<&'static str> {
    Some(match code {
        "11" => "Team/Crew",
        "12" => "Squad",
        "13" => "Section",
        "14" => "Platoon/detachment",
        "15" => "Company/battery/troop",
        "16" => "Battalion/squadron",
        "17" => "Regiment/group",
        "18" => "Brigade",
        "21" => "Division",
        "22" => "Corps/MEF",
        "23" => "Army",
        "24" => "Army Group/front",
        "25" => "Region/Theater",
        "26" => "Command",
        "31" => "Wheeled limited cross country",
        "32" => "Wheeled cross country",
        "33" => "Tracked",
        "34" => "Wheeled and tracked combination",
        "35" => "Towed",
        "36" => "Rail",
        "37" => "Pack animals",
        "41" => "Over snow (prime mover)",
        "42" => "Sled",
        "51" => "Barge",
        "52" => "Amphibious",
        "61" => "Short towed array",
        "62" => "Long towed Array",
        "71" => "Leader Individual",
        "72" => "Deputy Individual",
        _ => return None,
    })
}

/// Upstream `mapping.status`.
pub(crate) const STATUS: [&str; 6] = [
    "Present",
    "Planned",
    "FullyCapable",
    "Damaged",
    "Destroyed",
    "FullToCapacity",
];

/// Upstream `mapping.context`.
pub(crate) const CONTEXT: [&str; 3] = ["Reality", "Exercise", "Simulation"];

/// Dash arrays used for not-present frames.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Dashes<'a> {
    pub pending: &'a str,
    pub anticipated: &'a str,
}

/// Inputs to SIDC interpretation besides the code itself.
pub(crate) struct ParseInput<'a> {
    pub style_frame: bool,
    pub alternate_medal: bool,
    pub dashes: Dashes<'a>,
}

/// Normalizes a SIDC as upstream does (`*` → `-`, spaces removed).
pub(crate) fn normalize(sidc: &str) -> String {
    sidc.chars()
        .filter(|&c| c != ' ')
        .map(|c| if c == '*' { '-' } else { c })
        .collect()
}

/// Whether upstream treats the (normalized) SIDC as numeric.
pub(crate) fn is_number_sidc(sidc: &str) -> bool {
    !js::is_nan_str(&js::substr(sidc, 0, 2))
}

/// Interprets `sidc` (already normalized) into `md`. Returns the SIDC as
/// upstream stores it afterwards (letter codes are upper-cased).
pub(crate) fn interpret(sidc: &str, md: &mut Metadata, input: &ParseInput<'_>) -> String {
    md.number_sidc = is_number_sidc(sidc);
    if md.number_sidc {
        number::interpret(sidc, md, input);
        String::from(sidc)
    } else {
        let upper = sidc.to_uppercase();
        letter::interpret(&upper, md, input);
        upper
    }
}
