use super::sanitize::*;

fn esc(f: fn(&mut String, &str), s: &str) -> String {
    let mut out = String::new();
    f(&mut out, s);
    out
}

#[test]
fn escapes_like_upstream() {
    assert_eq!(
        esc(escape_attr, "a&\"'<>\t\n"),
        "a&amp;&quot;&apos;&lt;&gt;  "
    );
    assert_eq!(esc(escape_text, "a&\"<>"), "a&amp;\"&lt;&gt;");
}

#[test]
fn colors_block_script_urls() {
    assert_eq!(sanitize_color(" red "), Some("red"));
    assert_eq!(sanitize_color("url(#x)"), None);
    assert_eq!(sanitize_color("URL (x)"), None);
    assert_eq!(sanitize_color("javascript:alert(1)"), None);
    assert_eq!(sanitize_color("data:image/png"), None);
    assert_eq!(sanitize_color("  "), None);
}

#[test]
fn attribute_whitelists() {
    assert_eq!(sanitize_dash_array(" 4,4 "), Some("4,4"));
    assert_eq!(sanitize_dash_array("4;x"), None);
    assert_eq!(sanitize_line_cap("ROUND"), Some("round"));
    assert_eq!(sanitize_font_weight("700").as_deref(), Some("700"));
    assert_eq!(sanitize_font_weight("750"), None);
    assert_eq!(sanitize_text_anchor("left"), None);
    assert_eq!(sanitize_baseline("Middle"), Some("middle"));
    assert_eq!(
        sanitize_font_family(Some("Arial, 'Helvetica'")),
        "Arial, 'Helvetica'"
    );
    assert_eq!(sanitize_font_family(Some("x;y")), "sans-serif");
    assert_eq!(sanitize_font_family(None), "sans-serif");
}

#[test]
fn raw_svg_blocklist() {
    assert!(svg_fragment_blocked("<script>x</script>"));
    assert!(svg_fragment_blocked("< foreignObject >"));
    assert!(svg_fragment_blocked("<g onload = x>"));
    assert!(svg_fragment_blocked("<a href='JavaScript:1'>"));
    assert!(!svg_fragment_blocked(
        "<g transform='scale(2)'><path d='M0,0'/></g>"
    ));
    assert!(!svg_fragment_blocked("<objective>"));
}

#[test]
fn ids_are_sanitized() {
    assert_eq!(sanitize_id("my id").as_deref(), Some("my_id"));
    assert_eq!(sanitize_id("1x").as_deref(), Some("id_1x"));
    assert_eq!(sanitize_id(" "), None);
}
