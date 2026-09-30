//! JavaScript `Number.prototype.toString` for the emitter's float literals.
//!
//! The emitted tables must be byte-identical to what the extraction produced
//! in JavaScript, so numbers are formatted by the ECMAScript rules. This
//! mirrors the crate's private `js::write_number`; `xtask` does not depend on
//! the crate it generates.

use std::fmt::Write as _;

/// Formats `v` like JavaScript's `String(v)`.
pub(crate) fn to_string(v: f64) -> String {
    if v.is_nan() {
        return String::from("NaN");
    }
    if v == 0.0 {
        return String::from("0");
    }
    if v.is_infinite() {
        return String::from(if v > 0.0 { "Infinity" } else { "-Infinity" });
    }
    let mut out = String::new();
    if v < 0.0 {
        out.push('-');
    }
    // Shortest digit count from `{:e}`, then the correctly rounded value at
    // that count, which breaks ties to even as ECMAScript requires.
    let shortest = format!("{:e}", v.abs());
    let k = shortest
        .split_once('e')
        .map_or(0, |(m, _)| m.chars().filter(char::is_ascii_digit).count())
        .max(1);
    let even = format!("{:.prec$e}", v.abs(), prec = k - 1);
    let sci = if even.parse::<f64>().ok() == Some(v.abs()) {
        even
    } else {
        shortest
    };
    let (mantissa, exp) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let n = exp.parse::<i32>().unwrap_or(0) + 1;
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let k = i32::try_from(digits.len()).unwrap_or(i32::MAX);
    let split = |at: i32| {
        digits
            .split_at_checked(usize::try_from(at).unwrap_or(0))
            .unwrap_or((digits, ""))
    };
    if k <= n && n <= 21 {
        out.push_str(digits);
        out.extend(std::iter::repeat_n(
            '0',
            usize::try_from(n - k).unwrap_or(0),
        ));
    } else if 0 < n && n <= 21 {
        let (int, frac) = split(n);
        write!(out, "{int}.{frac}").ok();
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', usize::try_from(-n).unwrap_or(0)));
        out.push_str(digits);
    } else {
        let (first, rest) = split(1);
        out.push_str(first);
        if !rest.is_empty() {
            write!(out, ".{rest}").ok();
        }
        let e = n - 1;
        write!(
            out,
            "e{}{}",
            if e < 0 { '-' } else { '+' },
            e.unsigned_abs()
        )
        .ok();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::to_string;

    #[test]
    fn matches_javascript() {
        for (v, want) in [
            (1.0, "1"),
            (-0.5, "-0.5"),
            (63.2, "63.2"),
            (0.1 + 0.2, "0.30000000000000004"),
            (1e21, "1e+21"),
            (1.5e-7, "1.5e-7"),
            (0.000_001, "0.000001"),
            (123_456_789_012_345_680_000.0, "123456789012345680000"),
            (f64::from_bits(0xc2d1_29ac_71f6_c9e8), "-75482737924903.62"),
        ] {
            assert_eq!(to_string(v), want, "{v:e}");
        }
    }
}
