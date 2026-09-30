use super::Value;
use crate::compat::{canonical_json, canonical_json_string, write_canonical_json};
use crate::ir::{Node, Num, Paint, PathData, Style};
use crate::{Renderer, options::SymbolOptions};
use alloc::{string::String, vec};

type TestResult = Result<(), alloc::boxed::Box<dyn core::error::Error>>;

#[test]
fn canonical_options_record_enabled_icon_font_override() -> TestResult {
    for enabled in [false, true] {
        let mut options = SymbolOptions::default();
        options.style.icon_text_uses_font_family = enabled;
        let symbol = Renderer::default().render("10031100001100000000", options)?;
        let out = canonical_json_string(&symbol);
        assert_eq!(out, canonical_json(&symbol).to_canonical_string());
        let record: serde_json::Value = serde_json::from_str(&out)?;
        assert_eq!(
            record
                .get("options")
                .and_then(|o| o.get("iconTextUsesFontFamily")),
            enabled.then_some(&serde_json::Value::Bool(true))
        );
    }
    Ok(())
}

#[test]
fn canonical_options_keep_native_values_without_duplicate_keys() -> TestResult {
    let mut options = SymbolOptions::default();
    for key in ["size", "sidc", "fill", "direction", "uniqueDesignation"] {
        options.set_text(key, "custom label");
    }
    options.direction = Some(45.0);
    let symbol = Renderer::default().render("10031000001211000000", options)?;
    let super::Json::Obj(root) = canonical_json(&symbol) else {
        return Err("missing canonical object".into());
    };
    let Some((_, super::Json::Obj(fields))) = root.iter().find(|(key, _)| key == "options") else {
        return Err("missing canonical options".into());
    };
    let mut seen = alloc::collections::BTreeSet::new();
    for (key, _) in fields {
        assert!(seen.insert(key), "duplicate option {key}");
    }
    let out = canonical_json_string(&symbol);
    assert_eq!(out, canonical_json(&symbol).to_canonical_string());
    let record: serde_json::Value = serde_json::from_str(&out)?;
    let opts = record.get("options").ok_or("missing options")?;
    assert_eq!(
        opts.get("size").and_then(serde_json::Value::as_f64),
        Some(100.0)
    );
    assert_eq!(opts.get("sidc"), Some(&serde_json::json!(symbol.sidc())));
    assert_eq!(opts.get("fill"), Some(&serde_json::json!(true)));
    assert_eq!(
        opts.get("direction").and_then(serde_json::Value::as_f64),
        Some(45.0)
    );
    assert_eq!(
        opts.get("uniqueDesignation"),
        Some(&serde_json::json!("custom label"))
    );
    assert_eq!(symbol.options().text_named("size"), "custom label");
    Ok(())
}

#[test]
fn canonical_keys_follow_javascript_index_order() -> TestResult {
    let keys = [
        "10",
        "2",
        "0",
        "4294967294",
        "4294967295",
        "01",
        "+1",
        "-0",
        "1.0",
        "1e0",
    ];
    let mut options = SymbolOptions::default();
    for key in keys {
        options.set_text(key, "v");
    }
    let s = Renderer::default().render("10031000001211000000", options)?;
    let out = canonical_json_string(&s);
    assert_eq!(out, canonical_json(&s).to_canonical_string());
    let expected = r#""0":"v","2":"v","10":"v","4294967294":"v","+1":"v","-0":"v","01":"v","1.0":"v","1e0":"v","4294967295":"v""#;
    assert!(
        out.contains(&alloc::format!("\"options\":{{{expected},")),
        "{out}"
    );
    let object = super::Json::Obj(
        keys.into_iter()
            .map(|key| (String::from(key), super::Json::Str(String::from("v"))))
            .collect(),
    );
    assert_eq!(
        object.to_canonical_string(),
        alloc::format!("{{{expected}}}")
    );
    Ok(())
}

#[test]
fn streaming_matches_owned_records_and_appends() -> TestResult {
    for sidc in [
        "10031000001211000000",
        "SFGPUCI----D",
        "SZGPUCI-----",
        "10130000000000000000",
        "",
    ] {
        let s = Renderer::default().symbol(sidc).render()?;
        let expected = canonical_json(&s).to_canonical_string();
        let mut out = String::from("prefix");
        write_canonical_json(&s, &mut out);
        assert_eq!(out.strip_prefix("prefix"), Some(expected.as_str()));
        assert_eq!(canonical_json_string(&s), expected);
    }
    Ok(())
}

#[test]
fn streaming_orders_utf16_keys_and_spills_without_losing_fields() -> TestResult {
    let mut options = SymbolOptions::default();
    for i in 0..150 {
        options.set_text(alloc::format!("custom-{i}"), alloc::format!("value-{i}"));
    }
    options.set_text("\u{e000}", "bmp");
    options.set_text("\u{10000}", "astral");
    options.set_text("escaped", "\"\\\n\r\t\u{8}\u{c}\u{1}\0😀");
    let s = Renderer::default().render("10031000001211000000", options)?;
    let out = canonical_json_string(&s);
    assert_eq!(out, canonical_json(&s).to_canonical_string());
    assert!(out.find("\u{10000}") < out.find("\u{e000}"));
    let parsed: serde_json::Value = serde_json::from_str(&out)?;
    let fields = parsed.get("options").ok_or("missing options")?;
    for i in 0..150 {
        let expected = alloc::format!("value-{i}");
        assert_eq!(
            fields
                .get(alloc::format!("custom-{i}"))
                .and_then(serde_json::Value::as_str),
            Some(expected.as_str())
        );
    }
    assert_eq!(
        fields.get("escaped").and_then(serde_json::Value::as_str),
        Some("\"\\\n\r\t\u{8}\u{c}\u{1}\0😀")
    );
    Ok(())
}

#[test]
fn streaming_covers_extension_instructions_and_nonfinite_attributes() -> TestResult {
    let mut text = crate::ir::TextNode::new(1.0, 2.0, "\"<>&\u{1}");
    text.font_size = Some(Num::Text("45".into()));
    text.font_family = Some("font".into());
    text.font_weight = Some("bold".into());
    text.text_anchor = Some("end".into());
    text.alignment_baseline = Some("middle".into());
    text.style = Style {
        fill: Some(Paint::None),
        stroke: Some(Paint::color("red")),
        fill_opacity: Some(Num::Bool(false)),
        stroke_width: Some(Num::Number(f64::NAN)),
        stroke_dasharray: Some("4,4".into()),
        line_cap: Some("round".into()),
        non_scaling_stroke: Some(2.0),
        style_fill: Some(true),
        icon: Some(true),
        clip_path: Some("M0,0".into()),
    };
    let nodes = vec![
        Node::Group(vec![
            Node::Missing,
            Node::Scalar(Num::Bool(false)),
            Node::TrustedSvg("<g/>".into()),
        ]),
        Node::clip(
            PathData::new("M0,0"),
            Some("clip".into()),
            vec![Node::rotate(
                45.0,
                0.0,
                0.0,
                vec![Node::scale(2.0, vec![Node::Text(text)])],
            )],
        ),
        Node::circle(f64::INFINITY, 0.0, 1.0),
        Node::Bare(Style::default()),
    ];
    let view = Value::Instructions(&nodes);
    let mut out = String::new();
    view.write(&mut out);
    assert_eq!(out, view.to_json().to_canonical_string());
    let parsed: serde_json::Value = serde_json::from_str(&out)?;
    assert!(
        parsed
            .get(2)
            .and_then(|n| n.get("cx"))
            .is_some_and(serde_json::Value::is_null)
    );
    Ok(())
}
