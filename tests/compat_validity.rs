//! `compat::validity` is the verdict the oracle records as `valid` and
//! `validExtended`, which the differential corpus compares with milsymbol.js.

use milsymbol::options::TextField;
use milsymbol::{Renderer, Symbol, ValidityIssue, compat};
use serde_json::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn record(symbol: &Symbol) -> Result<Value, serde_json::Error> {
    serde_json::from_str(&compat::canonical_json_string(symbol))
}

fn field<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.get(key).unwrap_or(&Value::Null)
}

fn cases() -> Result<Vec<(&'static str, Symbol)>, milsymbol::RenderError> {
    let r = Renderer::default();
    let infantry = "10031000161211000000";
    let unknown = "10031000009999990000";
    Ok(vec![
        ("valid", r.symbol(infantry).render()?),
        (
            "null text",
            r.symbol(infantry)
                .text(TextField::UniqueDesignation, "null value")
                .render()?,
        ),
        ("unknown icon", r.symbol(unknown).render()?),
        (
            "hidden unknown icon",
            r.symbol(unknown).with(|o| o.style.icon = false).render()?,
        ),
        (
            "unknown affiliation",
            r.symbol("10001000161211000000").render()?,
        ),
        ("letter", r.symbol("SFGPUCI-----").render()?),
        ("malformed letter", r.symbol("OBVAMA------").render()?),
        ("garbage", r.symbol("not a sidc").render()?),
    ])
}

#[test]
fn compat_validity_matches_the_corpus_fields() -> TestResult {
    for (name, symbol) in cases()? {
        let record = record(&symbol)?;
        let issues = compat::validity(&symbol).issues;
        assert_eq!(field(&record, "valid"), compat::is_valid(&symbol), "{name}");
        assert_eq!(compat::is_valid(&symbol), issues.is_empty(), "{name}");
        let has = |i| issues.contains(&i);
        let extended = field(&record, "validExtended");
        assert_eq!(
            field(extended, "drawInstructions"),
            !has(ValidityIssue::MissingInstruction) && !has(ValidityIssue::NullInDrawing),
            "{name}"
        );
        assert_eq!(
            field(extended, "icon"),
            !has(ValidityIssue::UnknownIcon),
            "{name}"
        );
    }
    Ok(())
}

#[test]
fn compat_keeps_the_upstream_rules() -> TestResult {
    let cases = cases()?;
    let get = |n: &str| cases.iter().find(|(name, _)| *name == n).map(|(_, s)| s);
    let (Some(null), Some(hidden), Some(unknown)) = (
        get("null text"),
        get("hidden unknown icon"),
        get("unknown icon"),
    ) else {
        return Err("missing case".into());
    };
    assert!(!compat::is_valid(null));
    assert!(null.validity().is_valid());
    assert!(compat::is_valid(hidden));
    assert!(!hidden.validity().is_valid());
    assert!(!compat::is_valid(unknown));
    Ok(())
}
