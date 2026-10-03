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

/// A CSS `<number>`: optional sign, digits with an optional fraction (or a
/// fraction alone), optional exponent.
fn is_number(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    let digits = |i: &mut usize| {
        let start = *i;
        while b.get(*i).is_some_and(u8::is_ascii_digit) {
            *i += 1;
        }
        *i > start
    };
    let mut any = digits(&mut i);
    if b.get(i) == Some(&b'.') {
        i += 1;
        any |= digits(&mut i);
    }
    if !any {
        return false;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        i += usize::from(matches!(b.get(i), Some(b'+' | b'-')));
        if !digits(&mut i) {
            return false;
        }
    }
    i == b.len()
}

/// A number or a percentage; for a hue also an angle.
fn is_component(s: &str, hue: bool) -> bool {
    let units: &[&str] = if hue {
        &["deg", "grad", "rad", "turn", ""]
    } else {
        &["%", ""]
    };
    units
        .iter()
        .any(|u| s.strip_suffix(u).is_some_and(is_number))
}

/// `rgb()`, `rgba()`, `hsl()` and `hsla()` with comma-separated or
/// space-separated components and an optional alpha.
fn is_function(s: &str) -> bool {
    let Some((name, rest)) = s.split_once('(') else {
        return false;
    };
    let hue = match name {
        "rgb" | "rgba" => false,
        "hsl" | "hsla" => true,
        _ => return false,
    };
    let Some(args) = rest.strip_suffix(')') else {
        return false;
    };
    let (main, alpha) = match args.split_once('/') {
        Some((m, a)) if !args.contains(',') => (m, Some(a.trim_matches(is_css_space))),
        Some(_) => return false,
        None => (args, None),
    };
    let commas = main.contains(',');
    let parts: alloc::vec::Vec<&str> = if commas {
        main.split(',')
            .map(|p| p.trim_matches(is_css_space))
            .collect()
    } else {
        main.split(is_css_space).filter(|p| !p.is_empty()).collect()
    };
    // Only the comma syntax takes the alpha as a fourth component.
    let channels = match (parts.len(), commas) {
        (3, _) => parts.as_slice(),
        (4, true) => parts.get(..3).unwrap_or(&[]),
        _ => return false,
    };
    let alpha_ok = alpha
        .or_else(|| parts.get(3).copied())
        .is_none_or(|a| is_component(a, false));
    alpha_ok
        && channels
            .iter()
            .enumerate()
            .all(|(i, c)| is_component(c, hue && i == 0))
}

/// Whether `s` is a CSS colour.
pub(crate) fn is_color(s: &str) -> bool {
    let t = s.trim_matches(is_css_space);
    if let Some(hex) = t.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit());
    }
    let lower = t.to_ascii_lowercase();
    NAMED.binary_search(&lower.as_str()).is_ok() || is_function(&lower)
}

#[cfg(test)]
mod tests;
