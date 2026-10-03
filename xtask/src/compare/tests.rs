use super::compare;
use std::io::Cursor;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A record whose semantic JSON is `sem` and whose SVG is `<svg/>`.
fn record(sem: &str) -> String {
    serde_json::json!({ "svg": "<svg/>", "sem": sem }).to_string()
}

/// Runs the comparison over one case with the given oracle and Rust records.
fn one(oracle: &str, rust: &str) -> Result<bool, Box<dyn std::error::Error>> {
    compare(
        Cursor::new("{}\n"),
        Cursor::new(format!("{oracle}\n")),
        Cursor::new(format!("{rust}\n")),
        10,
        &mut Vec::new(),
    )
}

#[test]
fn identical_records_agree() -> TestResult {
    assert!(one(&record(r#"{"x":1}"#), &record(r#"{"x":1}"#))?);
    Ok(())
}

#[test]
fn malformed_records_are_rejected_on_either_side() {
    let good = record("{}");
    for bad in [
        "null",
        "[]",
        "{}",
        r#"{"svg":123,"sem":null}"#,
        r#"{"svg":"<svg/>"}"#,
        r#"{"sem":"{}"}"#,
        r#"{"svg":"<svg/>","sem":"not JSON"}"#,
        r#"{"error":null}"#,
        r#"{"error":17}"#,
        r#"{"error":"bad","svg":"<svg/>","sem":"{}"}"#,
    ] {
        assert!(one(bad, bad).is_err(), "{bad}");
        assert!(one(bad, &good).is_err(), "{bad}");
        assert!(one(&good, bad).is_err(), "{bad}");
    }
}

#[test]
fn valid_error_records_compare_by_failure() -> TestResult {
    let err = r#"{"error":"oracle exception"}"#;
    assert!(one(
        err,
        r#"{"error":"input makes milsymbol.js throw: oracle exception"}"#
    )?);
    assert!(!one(err, r#"{"error":"another Rust error"}"#)?);
    assert!(!one(err, &record("{}"))?);
    assert!(!one(&record("{}"), err)?);
    Ok(())
}

#[test]
fn missing_member_differs_from_null() -> TestResult {
    assert!(!one(&record("{}"), &record(r#"{"x":null}"#))?);
    assert!(!one(&record(r#"{"x":null}"#), &record("{}"))?);
    Ok(())
}

#[test]
fn arrays_of_different_length_differ() -> TestResult {
    assert!(!one(&record("[1]"), &record("[1,null]"))?);
    Ok(())
}

#[test]
fn equal_values_with_different_text_differ() -> TestResult {
    assert!(!one(&record("[1]"), &record("[1.0]"))?);
    Ok(())
}

#[test]
fn empty_record_files_are_an_error() {
    let r = compare(
        Cursor::new("{}\n"),
        Cursor::new(""),
        Cursor::new(""),
        10,
        &mut Vec::new(),
    );
    assert!(r.is_err());
}

#[test]
fn empty_case_lists_cannot_pass_a_suite() {
    assert!(
        compare(
            Cursor::new("\n"),
            Cursor::new(""),
            Cursor::new(""),
            10,
            &mut Vec::new()
        )
        .is_err()
    );
}

#[test]
fn one_short_record_file_is_an_error() {
    let rec = record("{}");
    let r = compare(
        Cursor::new("{}\n{}\n"),
        Cursor::new(format!("{rec}\n{rec}\n")),
        Cursor::new(format!("{rec}\n")),
        10,
        &mut Vec::new(),
    );
    assert!(r.is_err());
}

#[test]
fn extra_records_are_an_error() {
    let rec = record("{}");
    let r = compare(
        Cursor::new("{}\n"),
        Cursor::new(format!("{rec}\n{rec}\n")),
        Cursor::new(format!("{rec}\n")),
        10,
        &mut Vec::new(),
    );
    assert!(r.is_err());
}

/// Runs the comparison over one case line with the given records.
fn declared(
    case: &str,
    oracle: &str,
    rust: &str,
) -> Result<(bool, String), Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    let agree = compare(
        Cursor::new(format!("{case}\n")),
        Cursor::new(format!("{oracle}\n")),
        Cursor::new(format!("{rust}\n")),
        10,
        &mut out,
    )?;
    Ok((agree, String::from_utf8(out)?))
}

/// `sem` of a record carrying lone surrogates (oracle) or U+FFFD (Rust).
fn surrogate_pair_records() -> (String, String) {
    (
        record(r#"{"m":"\ud83d00","n":"\ude00"}"#),
        record("{\"m\":\"\u{fffd}00\",\"n\":\"\u{fffd}\"}"),
    )
}

#[test]
fn lone_surrogates_must_be_declared() -> TestResult {
    let (oracle, rust) = surrogate_pair_records();
    assert!(!one(&oracle, &rust)?, "undeclared");
    let (agree, out) = declared(r#"{"known":"lone-surrogate"}"#, &oracle, &rust)?;
    assert!(agree, "{out}");
    assert!(out.contains("1 as documented, 0 not"), "{out}");
    Ok(())
}

#[test]
fn a_declared_difference_must_occur() -> TestResult {
    let same = record(r#"{"x":1}"#);
    for kind in ["lone-surrogate", "throws-upstream", "proto-key"] {
        let case =
            format!(r#"{{"known":"{kind}","options":{{"__proto__":"x","hasOwnProperty":"x"}}}}"#);
        assert!(!declared(&case, &same, &same)?.0, "{kind}");
    }
    assert!(declared(r#"{"known":"other"}"#, &same, &same).is_err());
    Ok(())
}

#[test]
fn declared_upstream_exceptions_and_proto_keys() -> TestResult {
    let throws = r#"{"known":"throws-upstream","options":{"hasOwnProperty":"x"}}"#;
    let err = r#"{"error":"options.hasOwnProperty is not a function"}"#;
    let expected = record(r#"{"options":{"hasOwnProperty":"x"}}"#);
    assert!(declared(throws, &with_expected(err, &expected)?, &expected)?.0);
    assert!(!declared(throws, err, &expected)?.0, "control is required");
    let proto = r#"{"known":"proto-key","options":{"__proto__":"x"}}"#;
    let oracle = record(r#"{"options":{"a":"A"}}"#);
    let rust = record(r#"{"options":{"__proto__":"x","a":"A"}}"#);
    let oracle = with_expected(&oracle, &rust)?;
    assert!(declared(proto, &oracle, &rust)?.0);
    let other = record(r#"{"options":{"__proto__":"x","a":"B"}}"#);
    assert!(
        !declared(proto, &oracle, &other)?.0,
        "other differences still fail"
    );
    Ok(())
}

#[test]
fn lone_surrogates_do_not_hide_other_differences() -> TestResult {
    let oracle = record(r#"{"m":"\ud83d00","x":1}"#);
    let rust = record("{\"m\":\"\u{fffd}00\",\"x\":2}");
    assert!(!declared(r#"{"known":"lone-surrogate"}"#, &oracle, &rust)?.0);
    // A valid surrogate pair is not lone; the texts must then match.
    let pair = record(r#"{"m":"😀"}"#);
    assert!(!one(&pair, &record("{\"m\":\"\u{fffd}\"}"))?);
    Ok(())
}

#[test]
fn surrogate_replacement_leaves_other_escapes_alone() {
    use super::surrogates::replace_lone;
    assert_eq!(replace_lone(r#""\\ud83d""#), None, "escaped backslash");
    assert_eq!(replace_lone(r#""😀""#), None, "valid pair");
    assert_eq!(
        replace_lone(r#""a\ud83db\ude00""#).as_deref(),
        Some("\"a\u{fffd}b\u{fffd}\"")
    );
}

fn with_expected(oracle: &str, expected: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut value: serde_json::Value = serde_json::from_str(oracle)?;
    value
        .as_object_mut()
        .ok_or("record must be an object")?
        .insert("expected".into(), serde_json::from_str(expected)?);
    Ok(value.to_string())
}

#[test]
fn known_surrogates_preserve_every_other_json_byte() -> TestResult {
    let (oracle, rust) = surrogate_pair_records();
    let parsed: serde_json::Value = serde_json::from_str(&rust)?;
    let sem = parsed
        .get("sem")
        .and_then(serde_json::Value::as_str)
        .ok_or("missing sem")?;
    let case = r#"{"known":"lone-surrogate"}"#;
    for bad in [
        format!(" {sem}"),
        sem.replace('\u{fffd}', r"\ud801"),
        sem.replace('\u{fffd}', r"\ufffd"),
    ] {
        assert!(!declared(case, &oracle, &record(&bad))?.0, "{bad}");
    }
    Ok(())
}

#[test]
fn known_proto_keys_preserve_the_input_value_and_exact_output() -> TestResult {
    let case = r#"{"known":"proto-key","options":{"__proto__":"x"}}"#;
    let original = record(r#"{"options":{"a":"A"}}"#);
    let correct = record(r#"{"options":{"__proto__":"x","a":"A"}}"#);
    let oracle = with_expected(&original, &correct)?;
    for bad in [
        r#"{"options":{"__proto__":"CORRUPTED","a":"A"}}"#,
        r#"{"options":{"a":"A","__proto__":"x"}}"#,
        r#" {"options":{"__proto__":"x","a":"A"}}"#,
    ] {
        let bad = record(bad);
        assert!(!declared(case, &oracle, &bad)?.0);
    }
    let wrong_value = record(r#"{"options":{"__proto__":"CORRUPTED","a":"A"}}"#);
    assert!(!declared(case, &with_expected(&original, &wrong_value)?, &wrong_value)?.0);
    let changed = record(r#"{"options":{"__proto__":"x","a":"B"}}"#);
    assert!(!declared(case, &with_expected(&original, &changed)?, &changed)?.0);
    Ok(())
}

#[test]
fn known_upstream_errors_require_the_exact_error_and_control_output() -> TestResult {
    let case = r#"{"known":"throws-upstream","options":{"hasOwnProperty":"x"}}"#;
    let expected = record(r#"{"options":{"hasOwnProperty":"x"}}"#);
    let oracle = with_expected(
        r#"{"error":"options.hasOwnProperty is not a function"}"#,
        &expected,
    )?;
    assert!(!declared(case, &oracle, &record("{}"))?.0);
    let wrong_svg =
        serde_json::json!({"svg":"WRONG","sem":r#"{"options":{"hasOwnProperty":"x"}}"#})
            .to_string();
    assert!(!declared(case, &oracle, &wrong_svg)?.0);
    let wrong_error = with_expected(r#"{"error":"unrelated oracle failure"}"#, &expected)?;
    assert!(!declared(case, &wrong_error, &expected)?.0);
    Ok(())
}

#[test]
fn malformed_cases_and_incomplete_known_declarations_are_errors() {
    let rec = record("{}");
    for case in [
        "not JSON",
        "null",
        "[]",
        r#"{"known":"proto-key"}"#,
        r#"{"known":"throws-upstream","options":{"hasOwnProperty":0}}"#,
    ] {
        assert!(declared(case, &rec, &rec).is_err(), "{case}");
    }
}

#[test]
fn prototype_colour_modes_render_upstream_and_fail_in_rust() -> TestResult {
    let case = r#"{"known":"prototype-color-mode","options":{"colorMode":"toString"}}"#;
    let oracle = record(r#"{"x":1}"#);
    let error = r#"{"error":"unknown colour mode \"toString\""}"#;
    assert!(declared(case, &oracle, error)?.0);
    let other = r#"{"error":"unknown colour mode \"valueOf\""}"#;
    assert!(
        !declared(case, &oracle, other)?.0,
        "the error names the mode"
    );
    assert!(!declared(case, error, error)?.0, "upstream must render");
    assert!(!declared(case, &oracle, &oracle)?.0, "Rust must fail");
    let ordinary = r#"{"known":"prototype-color-mode","options":{"colorMode":"Pink"}}"#;
    assert!(
        declared(ordinary, &oracle, error).is_err(),
        "only prototype names"
    );
    Ok(())
}

#[test]
fn errors_must_be_the_same_failure() {
    use super::same_failure;
    let missing = "Cannot read properties of undefined (reading 'Civilian')";
    assert!(same_failure(missing, "unknown colour mode \"x\""));
    assert!(!same_failure(missing, "option `size` is not finite"));
    assert!(same_failure("boom", "input makes milsymbol.js throw: boom"));
    assert!(!same_failure(
        "boom",
        "input makes milsymbol.js throw: other"
    ));
    assert!(!same_failure("other", "unknown colour mode \"x\""));
}
