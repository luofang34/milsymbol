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
        Cursor::new("case\n"),
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
fn valid_error_records_compare_by_outcome() -> TestResult {
    let err = r#"{"error":"oracle exception"}"#;
    assert!(one(err, r#"{"error":"typed Rust error"}"#)?);
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
        Cursor::new("case\n"),
        Cursor::new(""),
        Cursor::new(""),
        10,
        &mut Vec::new(),
    );
    assert!(r.is_err());
}

#[test]
fn one_short_record_file_is_an_error() {
    let rec = record("{}");
    let r = compare(
        Cursor::new("a\nb\n"),
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
        Cursor::new("a\n"),
        Cursor::new(format!("{rec}\n{rec}\n")),
        Cursor::new(format!("{rec}\n")),
        10,
        &mut Vec::new(),
    );
    assert!(r.is_err());
}

#[test]
fn lone_surrogates_are_a_reported_known_difference() -> TestResult {
    // The oracle's sem text holds the escape `\ud83d`; Rust writes U+FFFD.
    let oracle = record(r#"{"m":"\ud83d00","n":"\ude00"}"#);
    let rust = record("{\"m\":\"\u{fffd}00\",\"n\":\"\u{fffd}\"}");
    let mut out = Vec::new();
    let agree = compare(
        Cursor::new("case\n"),
        Cursor::new(format!("{oracle}\n")),
        Cursor::new(format!("{rust}\n")),
        10,
        &mut out,
    )?;
    assert!(agree);
    assert!(String::from_utf8(out)?.contains("known differences (lone UTF-16 surrogates"));
    Ok(())
}

#[test]
fn lone_surrogates_do_not_hide_other_differences() -> TestResult {
    let oracle = record(r#"{"m":"\ud83d00","x":1}"#);
    let rust = record("{\"m\":\"\u{fffd}00\",\"x\":2}");
    assert!(!one(&oracle, &rust)?);
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
        Some(r#""a\ufffdb\ufffd""#)
    );
}
