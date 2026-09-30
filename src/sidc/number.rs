//! Numeric SIDC interpretation (upstream `numbersidc/metadata.js`).

use super::ParseInput;
use crate::domain::{
    Affiliation, Context, Dimension, Echelon, Edition, Leadership, Mobility, Status,
};
use crate::js::{self, JsStr};
use crate::metadata::Field;
use crate::metadata::Metadata;
use alloc::borrow::Cow;
use alloc::string::String;

fn affiliation(si2: &str) -> Option<Affiliation> {
    Some(match si2 {
        "0" | "1" => Affiliation::Unknown,
        "2" | "3" => Affiliation::Friend,
        "4" => Affiliation::Neutral,
        "5" | "6" => Affiliation::Hostile,
        _ => return None,
    })
}

fn dimension(symbol_set: &str) -> Field<Dimension> {
    match symbol_set {
        "00" | "30" | "53" => Field::Known(Dimension::Sea),
        "01" | "02" | "05" | "06" | "50" | "51" => Field::Known(Dimension::Air),
        "10" | "11" | "12" | "15" | "20" | "40" | "52" | "60" => Field::Known(Dimension::Ground),
        "35" | "36" | "39" | "54" => Field::Known(Dimension::Subsurface),
        _ => Field::Empty,
    }
}

/// Loose equality of a SIDC substring with a number (`"150000" == 150000`).
fn eq_num(s: &str, n: f64) -> bool {
    js::string_to_number(s) == n
}

pub(super) fn interpret(sidc: &str, md: &mut Metadata, input: &ParseInput<'_>) {
    let s = JsStr::new(sidc);
    let version = s.substr(0, 2);
    let si1 = s.substr(2, 1);
    let si2 = s.substr(3, 1);
    let symbol_set = s.substr(4, 2);
    let status = s.substr(6, 1);
    let hq_tf_dummy = s.substr(7, 1);
    let echelon_mob = s.substr(8, 2);
    let frameshape = non_empty_or(s.substr(22, 1), "0");

    if matches!(&*version, "10" | "11" | "12") {
        md.flags.edition = Some(Edition::D);
    }
    if matches!(&*version, "13" | "14") {
        md.flags.edition = Some(Edition::E);
    }
    if eq_num(&version, 13.0) && eq_num(&si2, 5.0) {
        md.flags.suspect = Some(true);
    }

    let function_id = s.substr(10, 10);
    let fid = JsStr::new(&function_id);
    md.function_id = String::from(&*function_id);
    let modifier = |sector: usize, code: usize| {
        let mut m = String::from(non_empty_or(s.substr(sector, 1), "0"));
        m.push_str(&non_empty_or(fid.substr(code, 2), "00"));
        m
    };
    md.flags.modifier1 = Some(modifier(20, 6));
    md.flags.modifier2 = Some(modifier(21, 8));

    md.context = js::parse_int(&si1)
        .and_then(|i| usize::try_from(i).ok())
        .and_then(|i| {
            [Context::Reality, Context::Exercise, Context::Simulation]
                .get(i)
                .copied()
        });
    md.affiliation = affiliation(&si2).into();
    md.dimension = dimension(&symbol_set);

    classify_symbol_set(&symbol_set, &fid, md, input);

    if status == "1" {
        md.notpresent = String::from(input.dashes.anticipated);
    }
    if matches!(&*si2, "0" | "2" | "5") {
        md.notpresent = String::from(input.dashes.pending);
    }
    let entity = fid.substr(0, 6);
    if (symbol_set == "30" && eq_num(&entity, 160000.0))
        || (symbol_set == "35" && (eq_num(&entity, 140000.0) || eq_num(&entity, 150000.0)))
    {
        md.notpresent = String::from(input.dashes.pending);
    }
    if matches!(&*status, "2" | "3" | "4" | "5") {
        let i = js::parse_int(&status)
            .and_then(|i| usize::try_from(i).ok())
            .unwrap_or(0);
        md.condition = [
            Status::Present,
            Status::Planned,
            Status::FullyCapable,
            Status::Damaged,
            Status::Destroyed,
            Status::FullToCapacity,
        ]
        .get(i)
        .copied();
    }

    md.base_dimension = md.dimension;
    md.base_affiliation = md.affiliation;
    identity(&si1, &si2, &symbol_set, md);
    civilian(&symbol_set, &fid, md);
    frame_shape(&frameshape, md);
    amplifiers(&hq_tf_dummy, &echelon_mob, md);
}

fn non_empty_or<'a>(s: Cow<'a, str>, fallback: &'a str) -> Cow<'a, str> {
    if s.is_empty() {
        Cow::Borrowed(fallback)
    } else {
        s
    }
}

fn classify_symbol_set(ss: &str, fid: &JsStr, md: &mut Metadata, input: &ParseInput<'_>) {
    if matches!(ss, "10" | "11" | "25" | "27" | "40") {
        md.unit = true;
    }
    if matches!(ss, "05" | "06" | "50") {
        md.space = true;
    }
    if ss == "40" {
        md.activity = true;
    }
    if ss == "15" {
        md.flags.landequipment = Some(true);
    }
    if ss == "20" {
        md.installation = true;
    }
    if ss == "25" {
        md.flags.control_measure = Some(true);
    }
    if ss == "60" {
        md.flags.cyberspace = Some(true);
    }
    if ss == "36" && !input.alternate_medal {
        md.fill = false;
    }
    if ss == "30" && eq_num(&fid.substr(0, 6), 150000.0) {
        md.frame = false;
    }
}

fn identity(si1: &str, si2: &str, ss: &str, md: &mut Metadata) {
    if si2 == "5" && si1 == "1" {
        md.joker = true;
    }
    if si2 == "6" && si1 == "1" {
        md.faker = true;
    }
    if md.joker || md.faker {
        md.affiliation = Field::Known(Affiliation::Friend);
    }
    if ss == "00" {
        md.dimension_unknown = true;
    }
    if ss == "00" && si1 == "1" && md.affiliation.known() != Some(Affiliation::Unknown) {
        md.affiliation = Field::Empty;
    }
    if ss == "27" {
        md.dimension = Field::Known(Dimension::LandDismountedIndividual);
        md.flags.dismounted = Some(true);
    }
    if ss == "15" || ss == "52" {
        md.dimension = Field::Known(Dimension::Sea);
    }
}

fn civilian(ss: &str, fid: &JsStr, md: &mut Metadata) {
    let f2 = fid.substr(0, 2);
    if (matches!(ss, "01" | "05" | "12" | "35") && f2 == "12")
        || ss == "11"
        || (ss == "15" && f2 == "16")
        || (ss == "30" && f2 == "14")
    {
        md.civilian = true;
    }
}

fn frame_shape(frameshape: &str, md: &mut Metadata) {
    if frameshape != "0" && md.flags.edition == Some(Edition::E) {
        md.civilian = false;
        md.flags.cyberspace = Some(false);
        md.installation = false;
        md.flags.landequipment = Some(false);
        md.activity = false;
        md.space = false;
        md.unit = false;
        let set = |md: &mut Metadata, dim| md.dimension = Field::Known(dim);
        match frameshape {
            "1" => {
                set(md, Dimension::Air);
                md.space = true;
            }
            "2" => set(md, Dimension::Air),
            "3" => {
                set(md, Dimension::Ground);
                md.unit = true;
            }
            "4" => {
                set(md, Dimension::Sea);
                md.flags.landequipment = Some(true);
            }
            "5" => {
                set(md, Dimension::Ground);
                md.installation = true;
            }
            "6" => {
                set(md, Dimension::LandDismountedIndividual);
                md.flags.dismounted = Some(true);
            }
            "7" => set(md, Dimension::Subsurface),
            "8" => {
                set(md, Dimension::Ground);
                md.activity = true;
                md.unit = true;
            }
            "9" => {
                set(md, Dimension::Ground);
                md.flags.cyberspace = Some(false);
                md.unit = true;
            }
            _ => {}
        }
    }
    if frameshape == "A" {
        md.frame = false;
    }
}

fn amplifiers(hq_tf_dummy: &str, echelon_mob: &str, md: &mut Metadata) {
    if matches!(hq_tf_dummy, "1" | "3" | "5" | "7") {
        md.flags.feint_dummy = Some(true);
    }
    if matches!(hq_tf_dummy, "2" | "3" | "6" | "7") {
        md.headquarters = true;
    }
    if matches!(hq_tf_dummy, "4" | "5" | "6" | "7") {
        md.task_force = true;
    }
    let n = js::string_to_number(echelon_mob);
    if n <= 30.0 {
        md.echelon = Echelon::from_code(echelon_mob).into();
    }
    if (30.0..70.0).contains(&n) {
        md.mobility = Mobility::from_code(echelon_mob).into();
    }
    if (70.0..80.0).contains(&n) {
        md.flags.leadership = match echelon_mob {
            "71" => Some(Leadership::Leader),
            "72" => Some(Leadership::Deputy),
            _ => None,
        };
    }
}
