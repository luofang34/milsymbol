//! Letter SIDC interpretation (upstream `lettersidc/metadata.js`).

use super::{ParseInput, STATUS, echelon_mobility};
use crate::js::JsStr;
use crate::metadata::Metadata;
use alloc::borrow::Cow;
use alloc::string::String;

fn char_or_dash<'a>(s: &JsStr<'a>, i: usize) -> Cow<'a, str> {
    let c = s.char_at(i);
    if c.is_empty() { Cow::Borrowed("-") } else { c }
}

fn set(slot: &mut Option<String>, v: &str) {
    *slot = Some(String::from(v));
}

const EMS_SEA_FRAMED_O: [&str; 22] = [
    "AB----", "AE----", "AF----", "BB----", "CB----", "CC----", "DB----", "DDB---", "DEB---",
    "DFB---", "DGB---", "DHB---", "DIB---", "DJB---", "DLB---", "DMB---", "DOB---", "EA----",
    "EB----", "EC----", "ED----", "EE----",
];
const UNFRAMED_SEA: [&str; 7] = [
    "O-----", "ED----", "EP----", "EV----", "ZM----", "ZN----", "ZI----",
];
const UNFRAMED_EMS_N: [&str; 16] = [
    "AA----", "AB----", "AC----", "AD----", "AE----", "AG----", "BB----", "BC----", "BF----",
    "BM----", "-C-----", "CA----", "CB----", "CC----", "CD----", "CE----",
];
const UNFRAMED_WEATHER: [&str; 10] = [
    "WSVE--", "WSD-LI", "WSFGSO", "WSGRL-", "WSR-LI", "WSDSLM", "WSS-LI", "WSTMH-", "WST-FC",
    "WSTSS-",
];
const MINES: [&str; 49] = [
    "WM----", "WMD---", "WMG---", "WMGD--", "WMGX--", "WMGE--", "WMGC--", "WMGR--", "WMGO--",
    "WMM---", "WMMD--", "WMMX--", "WMME--", "WMMC--", "WMMR--", "WMMO--", "WMF---", "WMFD--",
    "WMFX--", "WMFE--", "WMFC--", "WMFR--", "WMFO--", "WMO---", "WMOD--", "WMX---", "WME---",
    "WMA---", "WMC---", "WMR---", "WMB---", "WMBD--", "WMN---", "WMS---", "WMSX--", "WMSD--",
    "WD----", "WDM---", "WDMG--", "WDMM--", "ND----", "E-----", "V-----", "X-----", "NBS---",
    "NBR---", "NBW---", "NM----", "NA----",
];

struct Codes<'a> {
    scheme: Cow<'a, str>,
    aff: Cow<'a, str>,
    dim: Cow<'a, str>,
    status: Cow<'a, str>,
    fid: Cow<'a, str>,
    m11: Cow<'a, str>,
    m12: Cow<'a, str>,
}

pub(super) fn interpret(sidc: &str, md: &mut Metadata, input: &ParseInput<'_>) {
    let s = JsStr::new(sidc);
    let fid = s.substr(4, 6);
    let c = Codes {
        scheme: char_or_dash(&s, 0),
        aff: char_or_dash(&s, 1),
        dim: char_or_dash(&s, 2),
        status: char_or_dash(&s, 3),
        fid: if fid.is_empty() {
            Cow::Borrowed("------")
        } else {
            fid
        },
        m11: char_or_dash(&s, 10),
        m12: char_or_dash(&s, 11),
    };
    md.function_id = String::from(&*c.fid);
    identity_and_dimension(&c, md, input);
    md.base_dimension = md.dimension.clone();
    md.base_affiliation = md.affiliation.clone();
    remap(&c, md);
    amplifiers(&c, md);
    civilian_and_unknown(&c, md);
    framing(&c, sidc, md);
}

fn identity_and_dimension(c: &Codes<'_>, md: &mut Metadata, input: &ParseInput<'_>) {
    let a = &*c.aff;
    match a {
        "H" | "S" | "J" | "K" => set(&mut md.affiliation, "Hostile"),
        "F" | "A" | "D" | "M" => set(&mut md.affiliation, "Friend"),
        "N" | "L" => set(&mut md.affiliation, "Neutral"),
        "P" | "U" | "G" | "W" | "O" => set(&mut md.affiliation, "Unknown"),
        _ => {}
    }
    match &*c.dim {
        "P" | "A" => md.dimension = String::from("Air"),
        "G" | "Z" | "F" | "X" => md.dimension = String::from("Ground"),
        "S" => md.dimension = String::from("Sea"),
        "U" => md.dimension = String::from("Subsurface"),
        _ => {}
    }
    if c.dim == "P" && c.scheme != "O" {
        md.space = true;
    }
    if c.scheme == "O" && matches!(&*c.dim, "V" | "O" | "R") {
        md.activity = true;
    }
    if c.scheme == "G" {
        md.flags.control_measure = Some(true);
    }
    if c.m11 == "H" {
        md.installation = true;
    }
    if input.style_frame && c.status == "A" {
        md.notpresent = String::from(input.dashes.anticipated);
    }
    if input.style_frame && matches!(a, "P" | "A" | "S" | "G" | "M") {
        md.notpresent = String::from(input.dashes.pending);
    }
    let condition = match &*c.status {
        "C" => STATUS.get(2),
        "D" => STATUS.get(3),
        "X" => STATUS.get(4),
        "F" => STATUS.get(5),
        _ => None,
    };
    if let Some(cond) = condition {
        md.condition = String::from(*cond);
    }
    if matches!(a, "G" | "W" | "D" | "L" | "M" | "J" | "K") {
        set(&mut md.context, "Exercise");
    }
    if c.scheme == "O" || c.scheme == "E" {
        md.dimension = String::from("Ground");
    }
}

fn remap(c: &Codes<'_>, md: &mut Metadata) {
    if c.aff == "J" {
        md.joker = true;
    }
    if c.aff == "K" {
        md.faker = true;
    }
    if md.joker || md.faker {
        set(&mut md.affiliation, "Friend");
    }
    let fid = &*c.fid;
    let sea = (c.scheme == "S" && c.dim == "G" && fid.starts_with('E'))
        || (c.scheme == "I" && c.dim == "G")
        || (c.scheme == "E"
            && ((c.dim == "O" && EMS_SEA_FRAMED_O.contains(&fid))
                || (c.dim == "F" && ["BA----", "MA----", "MC----"].contains(&fid))));
    if sea {
        md.dimension = String::from("Sea");
    }
}

fn amplifiers(c: &Codes<'_>, md: &mut Metadata) {
    let (m11, m12) = (&*c.m11, &*c.m12);
    if matches!(m11, "F" | "G" | "C" | "D") || (m11 == "H" && m12 == "B") {
        md.flags.feint_dummy = Some(true);
    }
    if matches!(m11, "A" | "B" | "C" | "D") || (c.dim == "G" && c.fid == "UH----") {
        md.headquarters = true;
    }
    if matches!(m11, "E" | "B" | "G" | "D") {
        md.task_force = true;
    }
    let echelon = match m12 {
        "A" => Some("11"),
        "B" if m11 != "H" => Some("12"),
        "C" => Some("13"),
        "D" => Some("14"),
        "E" => Some("15"),
        "F" => Some("16"),
        "G" => Some("17"),
        "H" => Some("18"),
        "I" => Some("21"),
        "J" => Some("22"),
        "K" => Some("23"),
        "L" if m11 != "N" => Some("24"),
        "M" => Some("25"),
        "N" => Some("26"),
        _ => None,
    };
    if let Some(code) = echelon {
        md.echelon = echelon_mobility(code).map(String::from);
    }
    let mobility = match (m11, m12) {
        ("M", "O") => Some(Some("31")),
        ("M", "P") => Some(Some("32")),
        ("M", "Q") => Some(Some("33")),
        ("M", "R") => Some(Some("34")),
        ("M", "S") => Some(Some("35")),
        ("M", "T") => Some(Some("36")),
        ("M", "U") => Some(Some("41")),
        ("M", "V") => Some(Some("42")),
        ("M", "W") => Some(Some("37")),
        ("M", "X") => Some(Some("51")),
        ("M", "Y") => Some(Some("52")),
        ("N", "S") => Some(Some("61")),
        ("N", "L") => Some(Some("62")),
        ("M" | "N", _) => Some(None),
        _ => None,
    };
    if let Some(code) = mobility {
        md.mobility = code.and_then(echelon_mobility).map(String::from);
    }
}

fn civilian_and_unknown(c: &Codes<'_>, md: &mut Metadata) {
    let fid = &*c.fid;
    if (c.dim == "A" && fid.starts_with('C'))
        || (c.dim == "G" && fid.starts_with("EVC"))
        || (c.dim == "S" && fid.starts_with('X'))
    {
        md.civilian = true;
    }
    if c.dim == "Z" || c.dim == "X" {
        let a = &*c.aff;
        if matches!(a, "P" | "U" | "F" | "N" | "H" | "A" | "S" | "G" | "W") {
            md.dimension_unknown = true;
        }
        if matches!(a, "F" | "A") {
            md.dimension = String::from("Sea");
        }
        if matches!(a, "D" | "L" | "M" | "J" | "K") {
            set(&mut md.affiliation, "none");
        }
    }
}

fn framing(c: &Codes<'_>, sidc: &str, md: &mut Metadata) {
    let fid = &*c.fid;
    if c.dim == "S" && UNFRAMED_SEA.contains(&fid) {
        md.frame = false;
    }
    if c.scheme == "E" && c.dim == "N" && UNFRAMED_EMS_N.contains(&fid) {
        md.frame = false;
    }
    if c.scheme == "W" && c.dim == "S" && UNFRAMED_WEATHER.contains(&fid) {
        md.frame = false;
    }
    if c.dim == "U" && MINES.contains(&fid) {
        mines(fid, md);
    }
    let prefix = crate::js::substr(sidc, 0, 3);
    if prefix == "WAS" || prefix == "WOS" || c.scheme == "G" {
        md.frame = false;
    }
    if c.scheme == "G" && c.dim == "O" && (fid.starts_with(['V', 'L', 'P', 'I'])) {
        md.frame = true;
        md.dimension = String::from("Ground");
    }
}

fn mines(fid: &str, md: &mut Metadata) {
    if md.std2525 {
        md.fill = false;
        if fid == "WD----" {
            md.fill = true;
        }
        if ["ND----", "NBS---", "NBR---", "NBW---", "NM----", "NA----"].contains(&fid) {
            md.fill = true;
            md.frame = false;
        }
    } else {
        md.frame = false;
        if ["E-----", "V-----", "X-----"].contains(&fid) {
            md.fill = false;
            md.frame = false;
        }
    }
}
