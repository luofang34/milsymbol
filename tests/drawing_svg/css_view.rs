//! The SVG as a CSS-conforming reader sees its paint, for usvg to read.
//!
//! A `fill` or `stroke` value that is not a paint is an invalid
//! declaration, which CSS ignores, so the property inherits. usvg instead
//! falls back to the initial value (black fill, no stroke), and svgtypes
//! trims values with Rust's `str::trim`, which also strips U+0085 where CSS
//! strips only space, tab, LF, CR and FF. The drawing view follows CSS, as
//! browsers do. Removing the ignored declarations, and swapping U+0085 for
//! U+0080 (which nothing trims), lets usvg read the paint as CSS does.

fn unescape(v: &str) -> String {
    v.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Whether a CSS reader keeps this paint value.
fn is_paint(value: &str) -> bool {
    svgtypes::Paint::from_str(&unescape(value)).is_ok()
}

/// The rewritten SVG and how many declarations were removed.
pub(crate) fn css_view(svg: &str) -> (String, usize) {
    let svg = svg.replace('\u{85}', "\u{80}");
    let mut out = String::with_capacity(svg.len());
    let mut removed = 0;
    let mut rest = svg.as_str();
    while let Some(at) = [" fill=\"", " stroke=\""]
        .iter()
        .filter_map(|a| rest.find(a).map(|i| (i, a.len())))
        .min()
    {
        let (start, prefix) = at;
        let (head, tail) = rest.split_at(start);
        out.push_str(head);
        let Some(value_end) = tail.get(prefix..).and_then(|t| t.find('"')) else {
            rest = tail;
            break;
        };
        let (attribute, after) = tail.split_at(prefix + value_end + 1);
        let value = attribute
            .get(prefix..prefix + value_end)
            .unwrap_or_default();
        if is_paint(value) {
            out.push_str(attribute);
        } else {
            removed += 1;
        }
        rest = after;
    }
    out.push_str(rest);
    (out, removed)
}
