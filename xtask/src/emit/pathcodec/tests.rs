use super::*;
use proptest::prelude::*;

fn round_trips(text: &str) -> Result<(), TestCaseError> {
    let (enc, _) = encode(text);
    prop_assert_eq!(
        decoded(&enc).map_err(|e| TestCaseError::fail(e.to_string()))?,
        text
    );
    Ok(())
}

fn number() -> impl Strategy<Value = String> {
    (
        prop::bool::ANY,
        prop_oneof![Just("0".to_string()), "[1-9][0-9]{0,4}", "[1-9][0-9]{5,24}"],
        prop::option::of("[0-9]{1,7}"),
        prop::option::of("[eE][-+]?[0-9]{1,2}"),
    )
        .prop_map(|(neg, int, frac, exp)| {
            let mut s = String::new();
            if neg {
                s.push('-');
            }
            s.push_str(&int);
            if let Some(f) = frac {
                s.push('.');
                s.push_str(&f);
            }
            s.push_str(&exp.unwrap_or_default());
            s
        })
}

fn path_like() -> impl Strategy<Value = String> {
    let sep = prop_oneof![
        Just(""),
        Just(" "),
        Just(","),
        Just("  "),
        Just(", "),
        Just("\t")
    ];
    let token = prop_oneof!["[MmLlHhVvCcSsQqTtAaZzxX]".prop_map(String::from), number(),];
    prop::collection::vec((sep, token), 0..40).prop_map(|items| {
        items
            .into_iter()
            .map(|(s, t)| format!("{s}{t}"))
            .collect::<String>()
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn structured_paths_round_trip(text in path_like()) {
        round_trips(&text)?;
    }

    #[test]
    fn arbitrary_text_round_trips(text in "[A-Za-z0-9.,+ \\t\\-]{0,120}") {
        round_trips(&text)?;
    }

    #[test]
    fn arbitrary_unicode_round_trips(text in "\\PC{0,40}") {
        round_trips(&text)?;
    }
}

#[test]
fn well_formed_upstream_style_paths_need_no_fallback() {
    for text in [
        "m 80,85 40,0 0,-20 -40,0 z",
        "M25,50 l150,0 ",
        "m-15,0 0,-15 30,0z",
        "M 100 55 L 70 80 z M 95 80",
        "m 99,62.5 -0,0 -0.9,0.1 1e-5,3",
    ] {
        assert!(!encode(text).1, "{text}");
    }
}

#[test]
fn integers_too_large_to_scale_fall_back_instead_of_overflowing() {
    for text in [
        "m 9000000000000000000,1",
        "m 9223372036854775807,1",
        "m 4611686018427387904,0",
        "m -9223372036854775807,1",
        "m 99999999999999999999999,1",
    ] {
        let (enc, _) = encode(text);
        assert_eq!(decoded(&enc).as_deref().ok(), Some(text), "{text}");
    }
}

#[test]
fn a_wrong_encoding_is_rejected_by_the_round_trip_check() {
    let paths = vec!["m 1,2 3,4 z".to_string()];
    let wrong = |_: &str| encode("m 1,2 3,5 z");
    assert!(pack_with(&paths, wrong).is_err());
    assert!(pack_with(&paths, encode).is_ok());
}
