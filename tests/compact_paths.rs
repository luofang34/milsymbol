//! The `compact-paths` feature must not change any output: one digest over
//! the SVG, canonical JSON and drawing items of every built-in icon is
//! compared against the value the plain-path build produces, so the same
//! test passes with the feature off and on.

use milsymbol::options::SymbolOptions;
use milsymbol::{Renderer, catalog, compat};

const EXPECTED_DIGEST: u64 = 0x2c58_f751_15d9_a4ca;

fn fnv1a(hash: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *hash ^= u64::from(b);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

fn sidcs() -> Vec<String> {
    let mut out = Vec::new();
    for ss in catalog::number_symbol_sets() {
        for e in catalog::number_entities(ss) {
            out.push(format!("1003{ss}0000{e}0000"));
        }
    }
    for generic in catalog::letter_icons() {
        out.push(format!("SF{}", generic.get(2..).unwrap_or("")));
    }
    out
}

#[test]
fn every_builtin_icon_renders_identically_to_the_plain_path_build() {
    let renderer = Renderer::default();
    let ids = sidcs();
    assert!(ids.len() > 3000, "{}", ids.len());
    let mut digest = 0xcbf2_9ce4_8422_2325;
    let mut paths = 0usize;
    for id in &ids {
        let Ok(symbol) = renderer.render(id, SymbolOptions::default()) else {
            fnv1a(&mut digest, id.as_bytes());
            continue;
        };
        fnv1a(&mut digest, symbol.to_svg().as_bytes());
        fnv1a(
            &mut digest,
            compat::canonical_json_string(&symbol).as_bytes(),
        );
        let drawing = symbol.drawing();
        paths += drawing.items.len();
        fnv1a(&mut digest, format!("{:?}", drawing.items).as_bytes());
    }
    assert!(paths > 3000, "{paths}");
    assert_eq!(digest, EXPECTED_DIGEST, "digest {digest:#018x}");
}
