//! Template-context bitfield construction for icon evaluation.

use super::SymbolState;
use crate::domain::{Affiliation, Edition};
use crate::generated::vars;
use crate::ir::Paint;
use crate::metadata::Field;
use crate::template::{IconContext, Var};

/// Affiliation the icon parts are built for (`metadata.affiliation || "Friend"`).
pub(super) fn part_affiliation(s: &SymbolState<'_>) -> Field<Affiliation> {
    match s.metadata.affiliation {
        Field::Missing | Field::Empty => Field::Known(Affiliation::Friend),
        a => a,
    }
}

pub(super) fn context(s: &SymbolState<'_>, part_aff: Field<Affiliation>) -> IconContext {
    let (md, st) = (s.metadata, &s.options.style);
    let mut c = IconContext::new();
    c.set_bool(Var::Std2525, md.std2525);
    c.set_bool(Var::Frame, md.frame);
    c.set_bool(Var::NumberSidc, md.number_sidc);
    c.set_bool(Var::AlternateMedal, st.alternate_medal);
    c.set(Var::Mono, u8::from(st.mono_color.is_some()));
    c.set(
        Var::Edition,
        match md.edition() {
            Some(Edition::D) => 1,
            Some(Edition::E) => 2,
            _ => 0,
        },
    );
    let np = &md.notpresent;
    let dashes = &s.config.dash_arrays;
    c.set(
        Var::NotPresent,
        if np.is_empty() {
            0
        } else if *np == dashes.pending {
            1
        } else if *np == dashes.anticipated {
            2
        } else {
            1
        },
    );
    let aff_index = vars::AFFILIATIONS
        .iter()
        .position(|&a| Some(a) == part_aff.as_str())
        .unwrap_or(0);
    c.set(Var::Affiliation, u8::try_from(aff_index).unwrap_or(0));
    let geom = md.base_geometry.unwrap_or("none");
    let geom_index = vars::GEOMETRIES
        .iter()
        .position(|&g| g == geom)
        .unwrap_or(vars::GEOMETRIES.len() - 1);
    c.set(Var::Geometry, u8::try_from(geom_index).unwrap_or(0));
    let key = part_aff.known().unwrap_or(Affiliation::Friend);
    let slot = |m: &crate::color::ColorMode| matches!(m.for_affiliation(key), Some(Paint::Color(c)) if !c.is_empty());
    let cs = s.colors;
    for (var, mode) in [
        (Var::SlotFill, &cs.fill_color),
        (Var::SlotFrame, &cs.frame_color),
        (Var::SlotIcon, &cs.icon_color),
        (Var::SlotIconFill, &cs.icon_fill_color),
        (Var::SlotBlack, &cs.black),
        (Var::SlotWhite, &cs.white),
    ] {
        c.set_bool(var, slot(mode));
    }
    c
}
