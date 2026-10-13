use super::{Kind, OTHER_MEANING, codes, draws, listed};
use crate::catalog;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Rows of each Annex A table (entities, sector 1, sector 2), as printed.
const ROWS: [(&str, usize, usize, usize); 15] = [
    ("01", 53, 26, 5),
    ("02", 2, 10, 16),
    ("05", 37, 7, 11),
    ("10", 191, 69, 70),
    ("11", 12, 10, 1),
    ("15", 185, 6, 2),
    ("20", 90, 9, 11),
    // Tables A-32 (645 rows) and A-50 (7 rows).
    ("25", 652, 52, 6),
    ("27", 49, 16, 37),
    ("30", 93, 6, 10),
    ("35", 20, 9, 14),
    ("36", 64, 0, 0),
    ("40", 91, 16, 3),
    ("60", 14, 11, 10),
    ("64", 32, 11, 10),
];

fn ascending<T: Ord>(codes: &[T]) -> bool {
    codes.windows(2).all(|w| matches!(w, [a, b] if a < b))
}

#[test]
fn every_table_has_its_printed_rows_in_search_order() {
    assert_eq!(codes::SETS.len(), ROWS.len());
    for (set, (ss, entities, m1, m2)) in codes::SETS.iter().zip(ROWS) {
        assert_eq!(set.symbol_set, ss);
        assert_eq!(
            (set.entities.len(), set.modifier1.len(), set.modifier2.len()),
            (entities, m1, m2),
            "symbol set {ss}"
        );
        assert!(ascending(set.entities), "{ss} entities");
        assert!(ascending(set.modifier1), "{ss} sector 1");
        assert!(ascending(set.modifier2), "{ss} sector 2");
    }
    assert!(ascending(&codes::SETS.map(|s| s.symbol_set)));
    assert_eq!(codes::COMMON_MODIFIER1.len(), 91);
    assert_eq!(codes::COMMON_MODIFIER2.len(), 35);
    assert!(ascending(&codes::COMMON_MODIFIER1));
    assert!(ascending(&codes::COMMON_MODIFIER2));
}

#[test]
fn codes_drawn_with_another_meaning_are_listed_and_built_in() {
    for (ss, kind, code) in OTHER_MEANING {
        assert!(listed(ss, kind, code), "{ss} {kind:?} {code}");
        assert!(!draws(ss, kind, code), "{ss} {kind:?} {code}");
        let builtin: Vec<&str> = match kind {
            Kind::Entity => catalog::number_entities(ss).collect(),
            Kind::Modifier1 => catalog::number_modifier1(ss).collect(),
            Kind::Modifier2 => catalog::number_modifier2(ss).collect(),
        };
        let key = if kind == Kind::Entity {
            code
        } else {
            code.get(1..).unwrap_or("")
        };
        assert!(
            builtin.contains(&key),
            "{ss} {kind:?} {code} has no built-in icon"
        );
    }
}

#[test]
fn modifier_codes_name_their_table() {
    // `0` and two digits: the symbol set's own table.
    assert!(draws("10", Kind::Modifier1, "002"));
    // `1` and two digits: the common tables.
    assert!(draws("10", Kind::Modifier1, "107"));
    assert!(draws("10", Kind::Modifier2, "102"));
    assert!(!draws("10", Kind::Modifier2, "135"), "sector 2 ends at 134");
    // Version 2 removed common sector 1 modifier 155.
    assert!(!draws("30", Kind::Modifier1, "155"));
    // No modifier set besides `0` and `1` has codes.
    assert!(!draws("10", Kind::Modifier1, "202"));
    assert!(!draws("10", Kind::Modifier1, "0"));
}

#[test]
fn entities_are_six_digits_of_a_listed_set() {
    assert!(draws("10", Kind::Entity, "121100"));
    assert!(draws("10", Kind::Entity, "000000"));
    // Version 2 has no special entity subtypes 95–98.
    assert!(!draws("10", Kind::Entity, "121195"));
    assert!(!draws("10", Kind::Entity, "12110"));
    assert!(!draws("10", Kind::Entity, "1211000"));
    assert!(!draws("10", Kind::Entity, "+12110"));
    // Not a Version 2 symbol set.
    assert!(!draws("12", Kind::Entity, "110000"));
}

#[test]
fn us_only_symbol_sets_draw_every_built_in_icon() {
    for ss in ["06", "45", "46", "47", "50", "51", "52", "53", "54"] {
        assert!(draws(ss, Kind::Entity, "999999"));
        assert!(draws(ss, Kind::Modifier1, "099"));
    }
}

/// Every built-in code of every symbol set as a version `vv` SIDC, with the
/// code's kind: entities alone, own modifiers on the unspecified entity, and
/// common modifiers in a 30-digit SIDC.
fn built_in_sidcs(vv: &str) -> Vec<(String, Kind, String, String)> {
    let mut out = Vec::new();
    for ss in catalog::number_symbol_sets() {
        let base = format!("{vv}03{ss}0000");
        for e in catalog::number_entities(ss) {
            out.push((format!("{base}{e}0000"), Kind::Entity, ss.into(), e.into()));
        }
        for (kind, codes) in [
            (
                Kind::Modifier1,
                catalog::number_modifier1(ss).collect::<Vec<_>>(),
            ),
            (
                Kind::Modifier2,
                catalog::number_modifier2(ss).collect::<Vec<_>>(),
            ),
        ] {
            for m in codes {
                let (field, set) = match m.len() {
                    2 => (m, '0'),
                    _ => (m.get(1..).unwrap_or(""), m.chars().next().unwrap_or('0')),
                };
                let sidc = match (kind, set) {
                    (Kind::Modifier1, '0') => format!("{base}000000{field}00"),
                    (_, '0') => format!("{base}00000000{field}"),
                    (Kind::Modifier1, _) => format!("{base}000000{field}00{set}000000000"),
                    _ => format!("{base}00000000{field}0{set}00000000"),
                };
                let code = if m.len() == 2 {
                    format!("0{m}")
                } else {
                    m.into()
                };
                out.push((sidc, kind, ss.into(), code));
            }
        }
    }
    out
}

#[test]
fn version_16_draws_version_14_icons_where_version_2_agrees() -> Result<(), crate::RenderError> {
    use crate::{Renderer, Standard, ValidityIssue};
    let v2 = Renderer::builder()
        .standard(Standard::App6EVersion2)
        .build();
    let app6 = Renderer::builder().standard(Standard::App6).build();
    let (mut same, mut unknown) = (0usize, 0usize);
    for ((sidc, kind, ss, code), (e, ..)) in
        built_in_sidcs("16").into_iter().zip(built_in_sidcs("14"))
    {
        let drawn = v2.symbol(sidc.as_str()).render()?;
        if draws(&ss, kind, &code) {
            let expected = app6.symbol(e.as_str()).render()?;
            assert_eq!(drawn.to_svg(), expected.to_svg(), "{sidc}");
            same += 1;
        } else {
            let issues = drawn.sidc_validity().issues;
            assert!(
                issues.contains(&ValidityIssue::UnknownIcon),
                "{sidc}: {issues:?}"
            );
            unknown += 1;
        }
    }
    assert_eq!((same, unknown), (3744, 527));
    Ok(())
}
