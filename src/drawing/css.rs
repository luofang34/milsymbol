//! Whether text is a CSS colour, decided the way SVG readers decide it for
//! a `fill` or `stroke` attribute. A value that is not one is ignored, and
//! the property keeps the value it inherits; the drawing view resolves
//! paint the same way.

/// CSS named colours (CSS Color 4), lower case and sorted.
const NAMED: [&str; 149] = [
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "green",
    "greenyellow",
    "grey",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "transparent",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
];

fn is_css_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}

/// The end of the CSS `<number>` starting at `i` (optional sign, digits with
/// an optional fraction or a fraction alone, optional exponent), or `None`.
fn number_end(b: &[u8], mut i: usize) -> Option<usize> {
    let digits = |i: &mut usize| {
        let start = *i;
        while b.get(*i).is_some_and(u8::is_ascii_digit) {
            *i += 1;
        }
        *i > start
    };
    i += usize::from(matches!(b.get(i), Some(b'+' | b'-')));
    let mut any = digits(&mut i);
    if b.get(i) == Some(&b'.') && b.get(i + 1).is_some_and(u8::is_ascii_digit) {
        i += 1;
        any |= digits(&mut i);
    } else if any && b.get(i) == Some(&b'.') {
        i += 1;
    }
    if !any {
        return None;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        let mut j = i + 1;
        j += usize::from(matches!(b.get(j), Some(b'+' | b'-')));
        if b.get(j).is_some_and(u8::is_ascii_digit) {
            i = j;
            digits(&mut i);
        }
    }
    Some(i)
}

/// Whether `unit` may follow a number in a colour function: an angle for the
/// hue, a percentage elsewhere, or nothing.
fn is_unit(unit: &[u8], hue: bool) -> bool {
    let allowed: &[&[u8]] = if hue {
        &[b"", b"deg", b"grad", b"rad", b"turn"]
    } else {
        &[b"", b"%"]
    };
    allowed.iter().any(|u| u.eq_ignore_ascii_case(unit))
}

fn is_space_byte(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0c)
}

/// How a colour function separates its components.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Separator {
    Unknown,
    Comma,
    Space,
}

/// The arguments of `rgb()`, `rgba()`, `hsl()` or `hsla()`: three
/// components separated by commas or by spaces, and an optional alpha (a
/// fourth comma-separated component, or after `/` in the space syntax).
/// One pass over the bytes, no allocation.
fn is_arguments(b: &[u8], hue: bool) -> bool {
    let skip = |i: &mut usize| {
        let start = *i;
        while b.get(*i).copied().is_some_and(is_space_byte) {
            *i += 1;
        }
        *i > start
    };
    let (mut i, mut count, mut slash) = (0, 0, false);
    let mut separator = Separator::Unknown;
    skip(&mut i);
    loop {
        let Some(end) = number_end(b, i) else {
            return false;
        };
        let mut unit_end = end;
        while b
            .get(unit_end)
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'%')
        {
            unit_end += 1;
        }
        if !is_unit(b.get(end..unit_end).unwrap_or(&[]), hue && count == 0) {
            return false;
        }
        count += 1;
        i = unit_end;
        let spaced = skip(&mut i);
        let Some(&next) = b.get(i) else { break };
        let comma = next == b',';
        if next == b'/' {
            if separator == Separator::Comma || count != 3 || slash {
                return false;
            }
            slash = true;
        } else if comma && separator != Separator::Space && !slash {
            separator = Separator::Comma;
        } else if spaced && !comma && separator != Separator::Comma && !slash {
            separator = Separator::Space;
            continue;
        } else {
            return false;
        }
        i += 1;
        skip(&mut i);
    }
    match count {
        3 => !slash,
        4 => slash || separator == Separator::Comma,
        _ => false,
    }
}

/// `rgb()`, `rgba()`, `hsl()` and `hsla()`, names in any case.
fn is_function(s: &str) -> bool {
    let Some((name, rest)) = s.split_once('(') else {
        return false;
    };
    let hue = if name.eq_ignore_ascii_case("rgb") || name.eq_ignore_ascii_case("rgba") {
        false
    } else if name.eq_ignore_ascii_case("hsl") || name.eq_ignore_ascii_case("hsla") {
        true
    } else {
        return false;
    };
    rest.strip_suffix(')')
        .is_some_and(|args| is_arguments(args.as_bytes(), hue))
}

/// `name` against a lower-case table entry, ignoring ASCII case.
fn cmp_name(entry: &str, name: &str) -> core::cmp::Ordering {
    entry
        .bytes()
        .cmp(name.bytes().map(|b| b.to_ascii_lowercase()))
}

fn is_named(t: &str) -> bool {
    NAMED.binary_search(&t).is_ok()
        || (t.bytes().any(|b| b.is_ascii_uppercase())
            && NAMED.binary_search_by(|entry| cmp_name(entry, t)).is_ok())
}

/// Whether `s` is a CSS colour. Allocates nothing.
pub(crate) fn is_color(s: &str) -> bool {
    let t = s.trim_matches(is_css_space);
    if let Some(hex) = t.strip_prefix('#') {
        matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit())
    } else if t.ends_with(')') {
        is_function(t)
    } else {
        is_named(t)
    }
}

#[cfg(test)]
mod tests;
