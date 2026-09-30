//! Assignment of amplifier values to the left (L1–L5) and right (R1–R5)
//! text positions, per dimension.

use super::SymbolState;
use crate::js;
use crate::options::field as f;
use alloc::string::String;
use alloc::vec::Vec;

/// Texts at the five left and five right positions.
#[derive(Default)]
pub(super) struct Strings {
    pub l: [String; 5],
    pub r: [String; 5],
}

struct Opts<'a>(&'a SymbolState<'a>);

impl Opts<'_> {
    fn get(&self, key: &str) -> String {
        String::from(self.0.options.text_named(key))
    }

    /// Upstream's `if (a || b) { x = [a, b].filter(Boolean).join("/") }`.
    fn join(&self, keys: &[&str]) -> Option<String> {
        let parts: Vec<&str> = keys
            .iter()
            .map(|k| self.0.options.text_named(k))
            .filter(|v| !v.is_empty())
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("/"))
        }
    }
}

fn set(slot: &mut String, v: Option<String>) {
    if let Some(v) = v {
        *slot = v;
    }
}

const IDENTITY: [&str; 5] = [
    f::EVALUATION_RATING,
    f::COMBAT_EFFECTIVENESS,
    f::SIGNATURE_EQUIPMENT,
    f::HOSTILE,
    f::IFF_SIF,
];

fn air(o: &Opts<'_>, g: &mut Strings) {
    let [r1, r2, r3, r4, r5] = &mut g.r;
    *r1 = o.get(f::UNIQUE_DESIGNATION);
    *r2 = o.get(f::IFF_SIF);
    *r3 = o.get(f::TYPE);
    set(r4, o.join(&[f::SPEED, f::ALTITUDE_DEPTH]));
    set(r5, o.join(&[f::STAFF_COMMENTS, f::ADDITIONAL_INFORMATION]));
}

fn ground(o: &Opts<'_>, g: &mut Strings, unit_like: bool, activity: bool) {
    let [l1, l2, l3, l4, l5] = &mut g.l;
    let [r1, r2, r3, r4, r5] = &mut g.r;
    *l1 = o.get(f::DTG);
    set(l2, o.join(&[f::ALTITUDE_DEPTH, f::LOCATION]));
    *l4 = o.get(f::UNIQUE_DESIGNATION);
    *l5 = o.get(f::SPEED);
    *r2 = o.get(f::STAFF_COMMENTS);
    *r4 = o.get(f::HIGHER_FORMATION);
    set(r5, o.join(&IDENTITY));
    if unit_like {
        set(
            l3,
            o.join(&[f::TYPE, f::PLATFORM_TYPE, f::EQUIPMENT_TEARDOWN_TIME]),
        );
        *r1 = o.get(if activity {
            f::COUNTRY
        } else {
            f::REINFORCED_REDUCED
        });
        set(
            r3,
            o.join(&[f::ADDITIONAL_INFORMATION, f::COMMON_IDENTIFIER]),
        );
    } else {
        set(
            l3,
            o.join(&[
                f::TYPE,
                f::PLATFORM_TYPE,
                f::COMMON_IDENTIFIER,
                f::INSTALLATION_COMPOSITION,
            ]),
        );
        *r1 = o.get(f::COUNTRY);
        set(
            r3,
            o.join(&[f::ADDITIONAL_INFORMATION, f::EQUIPMENT_TEARDOWN_TIME]),
        );
    }
}

fn dismounted(o: &Opts<'_>, g: &mut Strings) {
    let [l1, l2, l3, l4, l5] = &mut g.l;
    let [r1, r2, r3, r4, r5] = &mut g.r;
    *l1 = o.get(f::DTG);
    set(l2, o.join(&[f::ALTITUDE_DEPTH, f::LOCATION]));
    set(
        l3,
        o.join(&[f::TYPE, f::PLATFORM_TYPE, f::COMMON_IDENTIFIER]),
    );
    *l4 = o.get(f::UNIQUE_DESIGNATION);
    *l5 = o.get(f::SPEED);
    *r1 = o.get(f::COUNTRY);
    *r2 = o.get(f::STAFF_COMMENTS);
    set(r3, o.join(&[f::ADDITIONAL_INFORMATION]));
    *r4 = o.get(f::HIGHER_FORMATION);
    set(r5, o.join(&IDENTITY));
}

fn sea(o: &Opts<'_>, g: &mut Strings) {
    let [r1, r2, r3, r4, r5] = &mut g.r;
    set(
        &mut g.l[0],
        o.join(&[f::GUARDED_UNIT, f::SPECIAL_DESIGNATOR]),
    );
    *r1 = o.get(f::UNIQUE_DESIGNATION);
    *r2 = o.get(f::TYPE);
    *r3 = o.get(f::IFF_SIF);
    set(r4, o.join(&[f::STAFF_COMMENTS, f::ADDITIONAL_INFORMATION]));
    set(r5, o.join(&[f::LOCATION, f::SPEED]));
}

fn subsurface(o: &Opts<'_>, g: &mut Strings) {
    g.l[0] = o.get(f::SPECIAL_DESIGNATOR);
    let [r1, r2, r3, r4, r5] = &mut g.r;
    *r1 = o.get(f::UNIQUE_DESIGNATION);
    *r2 = o.get(f::TYPE);
    *r3 = o.get(f::ALTITUDE_DEPTH);
    *r4 = o.get(f::STAFF_COMMENTS);
    *r5 = o.get(f::ADDITIONAL_INFORMATION);
}

/// Upstream's sections run in this order; later ones overwrite earlier ones.
pub(super) fn compute(s: &SymbolState<'_>) -> Strings {
    let o = Opts(s);
    let md = s.metadata;
    let letter = js::is_nan_str(s.sidc);
    let dim = md.base_dimension.known();
    let mut g = Strings::default();
    if !letter && dim == Some(crate::domain::Dimension::Air) {
        air(&o, &mut g);
    }
    if letter || dim == Some(crate::domain::Dimension::Ground) {
        ground(&o, &mut g, letter || md.unit, md.activity);
    }
    if md.dismounted() {
        dismounted(&o, &mut g);
    }
    if !letter && dim == Some(crate::domain::Dimension::Sea) {
        sea(&o, &mut g);
    }
    if !letter && dim == Some(crate::domain::Dimension::Subsurface) {
        subsurface(&o, &mut g);
    }
    g
}
