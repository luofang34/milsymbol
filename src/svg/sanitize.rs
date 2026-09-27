//! Escaping and sanitization rules of upstream `assvg.js`.

use alloc::string::String;

/// JavaScript regular-expression `\s`.
fn is_js_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{FEFF}'
}

/// Attribute escaping: `& " ' < >` become entities, CR/LF/TAB become spaces.
pub(super) fn escape_attr(out: &mut String, s: &str) {
    escape(out, s, |b| match b {
        b'&' => Some("&amp;"),
        b'"' => Some("&quot;"),
        b'\'' => Some("&apos;"),
        b'<' => Some("&lt;"),
        b'>' => Some("&gt;"),
        b'\r' | b'\n' | b'\t' => Some(" "),
        _ => None,
    });
}

/// Copies `s` to `out`, replacing the ASCII bytes `map` selects. Only ASCII
/// bytes are replaced, so slicing at them keeps UTF-8 boundaries intact.
fn escape(out: &mut String, s: &str, map: impl Fn(u8) -> Option<&'static str>) {
    let mut start = 0;
    for (i, b) in s.bytes().enumerate() {
        if let Some(rep) = map(b) {
            out.push_str(s.get(start..i).unwrap_or(""));
            out.push_str(rep);
            start = i + 1;
        }
    }
    out.push_str(s.get(start..).unwrap_or(""));
}

/// Text-content escaping: `& < >` become entities.
pub(super) fn escape_text(out: &mut String, s: &str) {
    escape(out, s, |b| match b {
        b'&' => Some("&amp;"),
        b'<' => Some("&lt;"),
        b'>' => Some("&gt;"),
        _ => None,
    });
}

fn trim_js(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// `^[0-9.,\s-]+$` after trimming; `None` drops the attribute.
pub(super) fn sanitize_dash_array(v: &str) -> Option<&str> {
    let t = trim_js(v);
    (!t.is_empty()
        && t.chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-') || is_js_space(c)))
    .then_some(t)
}

pub(super) fn sanitize_line_cap(v: &str) -> Option<&'static str> {
    match v.to_lowercase().as_str() {
        "butt" => Some("butt"),
        "round" => Some("round"),
        "square" => Some("square"),
        _ => None,
    }
}

pub(super) fn sanitize_font_weight(v: &str) -> Option<String> {
    let l = v.to_lowercase();
    let ok = matches!(l.as_str(), "normal" | "bold" | "bolder" | "lighter")
        || (l.len() == 3
            && l.ends_with("00")
            && l.as_bytes()
                .first()
                .is_some_and(|b| (b'1'..=b'9').contains(b)));
    ok.then_some(l)
}

pub(super) fn sanitize_text_anchor(v: &str) -> Option<&'static str> {
    match v.to_lowercase().as_str() {
        "start" => Some("start"),
        "middle" => Some("middle"),
        "end" => Some("end"),
        _ => None,
    }
}

const BASELINES: [&str; 11] = [
    "auto",
    "text-bottom",
    "alphabetic",
    "ideographic",
    "middle",
    "central",
    "mathematical",
    "hanging",
    "text-top",
    "text-before-edge",
    "text-after-edge",
];

pub(super) fn sanitize_baseline(v: &str) -> Option<&'static str> {
    let l = v.to_lowercase();
    BASELINES.iter().find(|b| **b == l).copied()
}

/// `^[a-zA-Z0-9 ,"'_:-]+$` after trimming, else `sans-serif`.
pub(super) fn sanitize_font_family(v: Option<&str>) -> &str {
    let Some(v) = v else { return "sans-serif" };
    let t = trim_js(v);
    let ok = !t.is_empty()
        && t.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, ' ' | ',' | '"' | '\'' | '_' | ':' | '-')
        });
    if ok { t } else { "sans-serif" }
}

fn contains_ci(hay: &str, needle: &str) -> bool {
    hay.to_lowercase().contains(needle)
}

/// Rejects `url(`, `javascript:` and `data:` colours; returns the trimmed value.
pub(super) fn sanitize_color(v: &str) -> Option<&str> {
    let t = trim_js(v);
    if t.is_empty() {
        return None;
    }
    let lower = t.to_lowercase();
    let url = lower.match_indices("url").any(|(i, _)| {
        lower
            .get(i + 3..)
            .is_some_and(|r| r.trim_start_matches(is_js_space).starts_with('('))
    });
    if url || lower.contains("javascript:") || lower.starts_with("data:") {
        return None;
    }
    Some(t)
}

/// Upstream's raw-SVG blocklist:
/// `<\s*(script|foreignObject|iframe|object|embed)[\s>]|on[a-z]+\s*=|javascript:` (case-insensitive).
pub(super) fn svg_fragment_blocked(v: &str) -> bool {
    if contains_ci(v, "javascript:") {
        return true;
    }
    let lower = v.to_lowercase();
    let chars: alloc::vec::Vec<char> = lower.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '<' {
            let mut j = i + 1;
            while chars.get(j).is_some_and(|c| is_js_space(*c)) {
                j += 1;
            }
            for tag in ["script", "foreignobject", "iframe", "object", "embed"] {
                let end = j + tag.chars().count();
                let word: String = chars.get(j..end).unwrap_or(&[]).iter().collect();
                if word == tag && chars.get(end).is_some_and(|c| is_js_space(*c) || *c == '>') {
                    return true;
                }
            }
        }
        if c == 'o' && chars.get(i + 1) == Some(&'n') {
            let mut j = i + 2;
            let start = j;
            while chars.get(j).is_some_and(char::is_ascii_lowercase) {
                j += 1;
            }
            if j > start {
                // Backtracking: any split of the letter run followed by `\s*=` matches only
                // when the run itself is followed by optional spaces and `=`.
                let mut k = j;
                while chars.get(k).is_some_and(|c| is_js_space(*c)) {
                    k += 1;
                }
                if chars.get(k) == Some(&'=') {
                    return true;
                }
            }
        }
    }
    false
}

/// Upstream `sanitizeId`; `None` means "use a generated id".
pub(super) fn sanitize_id(v: &str) -> Option<String> {
    let t = trim_js(v);
    if t.is_empty() {
        return None;
    }
    let mut id: String = t
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if !id.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
        id.insert_str(0, "id_");
    }
    Some(id)
}
