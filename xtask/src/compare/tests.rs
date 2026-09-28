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
