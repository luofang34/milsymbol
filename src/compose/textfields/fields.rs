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
        String::from(self.0.options.text(key))
    }

    /// Upstream's `if (a || b) { x = [a, b].filter(Boolean).join("/") }`.
    fn join(&self, keys: &[&str]) -> Option<String> {
        let parts: Vec<&str> = keys
            .iter()
            .map(|k| self.0.options.text(k))
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

pub(super) fn compute(s: &SymbolState<'_>) -> Strings {
    let o = Opts(s);
    let md = s.metadata;
    let letter = js::is_nan_str(s.sidc);
    let mut g = Strings::default();
    let [l1, l2, l3, l4, l5] = &mut g.l;
    let [r1, r2, r3, r4, r5] = &mut g.r;
    if !letter && md.base_dimension == "Air" {
        *r1 = o.get(f::UNIQUE_DESIGNATION);
        *r2 = o.get(f::IFF_SIF);
        *r3 = o.get(f::TYPE);
        set(r4, o.join(&[f::SPEED, f::ALTITUDE_DEPTH]));
        set(r5, o.join(&[f::STAFF_COMMENTS, f::ADDITIONAL_INFORMATION]));
    }
    let identity = [
        f::EVALUATION_RATING,
        f::COMBAT_EFFECTIVENESS,
        f::SIGNATURE_EQUIPMENT,
        f::HOSTILE,
        f::IFF_SIF,
    ];
    if letter || md.base_dimension == "Ground" {
        *l1 = o.get(f::DTG);
        set(l2, o.join(&[f::ALTITUDE_DEPTH, f::LOCATION]));
        *l4 = o.get(f::UNIQUE_DESIGNATION);
        *l5 = o.get(f::SPEED);
        *r2 = o.get(f::STAFF_COMMENTS);
        *r4 = o.get(f::HIGHER_FORMATION);
        set(r5, o.join(&identity));
        if letter || md.unit {
            set(
                l3,
                o.join(&[f::TYPE, f::PLATFORM_TYPE, f::EQUIPMENT_TEARDOWN_TIME]),
            );
            *r1 = o.get(f::REINFORCED_REDUCED);
            if md.activity {
                *r1 = o.get(f::COUNTRY);
            }
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
    if md.dismounted() {
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
        set(r5, o.join(&identity));
    }
    if !letter && md.base_dimension == "Sea" {
        set(l1, o.join(&[f::GUARDED_UNIT, f::SPECIAL_DESIGNATOR]));
        *r1 = o.get(f::UNIQUE_DESIGNATION);
        *r2 = o.get(f::TYPE);
        *r3 = o.get(f::IFF_SIF);
        set(r4, o.join(&[f::STAFF_COMMENTS, f::ADDITIONAL_INFORMATION]));
        set(r5, o.join(&[f::LOCATION, f::SPEED]));
    }
    if !letter && md.base_dimension == "Subsurface" {
        *l1 = o.get(f::SPECIAL_DESIGNATOR);
        *r1 = o.get(f::UNIQUE_DESIGNATION);
        *r2 = o.get(f::TYPE);
        *r3 = o.get(f::ALTITUDE_DEPTH);
        *r4 = o.get(f::STAFF_COMMENTS);
        *r5 = o.get(f::ADDITIONAL_INFORMATION);
    }
    g
}
