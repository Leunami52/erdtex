//! Small string helpers that behave like the Python functions the file format was first
//! written with, so that both apps read and write files identically.

/// Python's `str.isspace()`: Unicode white space, plus the four ASCII "information separators".
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// Python's `str.strip()`.
pub fn strip(s: &str) -> &str {
    s.trim_matches(is_space)
}

/// Python's `str.isalpha()`, close enough for TeX control words.
pub fn is_alpha(c: char) -> bool {
    c.is_alphabetic()
}

/// `s.get(a..b)`, or "" where that isn't a valid slice.  Positions in this crate are byte
/// offsets at ASCII characters, which are always char boundaries.
pub fn slice(s: &str, a: usize, b: usize) -> &str {
    s.get(a..b.min(s.len())).unwrap_or("")
}

/// `s.get(a..)`, or "".
pub fn from(s: &str, a: usize) -> &str {
    s.get(a.min(s.len())..).unwrap_or("")
}

/// Formats like Python's `f"{v:.3f}".rstrip("0").rstrip(".")`, and "-0" as "0".
pub fn fmt_num(v: f64) -> String {
    if !v.is_finite() {
        return py_special(v);
    }
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() { "0".to_string() } else { s.to_string() }
}

/// Python's `f"{v:g}"`: six significant digits, no trailing zeros, scientific notation for
/// exponents below -4 or from 6 on.
pub fn fmt_g(v: f64) -> String {
    if !v.is_finite() {
        return py_special(v);
    }
    if v == 0.0 {
        return if v.is_sign_negative() { "-0".into() } else { "0".into() };
    }
    let sci = format!("{v:.5e}"); // d.ddddde<exp>
    let (mantissa, exp) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    if !(-4..6).contains(&exp) {
        let m = mantissa.trim_end_matches('0').trim_end_matches('.');
        let sign = if exp < 0 { '-' } else { '+' };
        return format!("{m}e{sign}{:02}", exp.abs());
    }
    let decimals = usize::try_from(5 - exp).unwrap_or(0);
    let s = format!("{v:.decimals$}");
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

fn py_special(v: f64) -> String {
    if v.is_nan() {
        "nan".into()
    } else if v > 0.0 {
        "inf".into()
    } else {
        "-inf".into()
    }
}

/// Python's `round()`: halves go to the even neighbour.
pub fn round_half_even(v: f64) -> f64 {
    let r = v.round();
    if (v - v.trunc()).abs() == 0.5 { 2.0 * (v / 2.0).round() } else { r }
}

/// Python's `float(s)` for the plain decimal numbers that diagram files contain.
pub fn parse_float(s: &str) -> Option<f64> {
    let t = strip(s);
    let body = t.strip_prefix(['+', '-']).unwrap_or(t);
    let lower = body.to_ascii_lowercase();
    if lower.contains("inf") || lower.contains("nan") || body.contains('_') {
        return match lower.as_str() {
            "inf" | "infinity" => Some(if t.starts_with('-') { f64::NEG_INFINITY } else { f64::INFINITY }),
            "nan" => Some(f64::NAN),
            _ => None,
        };
    }
    t.parse().ok()
}

/// Python's `repr()` of a string, for error messages that match the Python app's.
pub fn py_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
    let mut out = String::from(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => out.push_str(&format!("\\x{:02x}", c as u32)),
            c if c.is_control() => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// Python's `float(s)`, with its error message.
pub fn py_float(s: &str) -> Result<f64, String> {
    parse_float(s).ok_or_else(|| format!("could not convert string to float: {}", py_repr(s)))
}

/// Python's `int(s)`, with its error message.
pub fn py_int(s: &str) -> Result<i64, String> {
    let t = strip(s);
    let body = t.strip_prefix(['+', '-']).unwrap_or(t);
    let ok = !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit());
    ok.then(|| t.parse::<i64>().ok())
        .flatten()
        .ok_or_else(|| format!("invalid literal for int() with base 10: {}", py_repr(s)))
}

/// Python's `x % m` for floats: the result has the sign of `m`.
pub fn py_mod(x: f64, m: f64) -> f64 {
    let r = x % m;
    if r != 0.0 && (r < 0.0) != (m < 0.0) { r + m } else { r }
}

/// Python's `sum()` of floats: compensated (Neumaier) summation, as Python does it since 3.12.
pub fn py_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let (mut f, mut c) = (0.0f64, 0.0f64);
    for x in values {
        let t = f + x;
        if f.abs() >= x.abs() {
            c += (f - t) + x;
        } else {
            c += (x - t) + f;
        }
        f = t;
    }
    if c != 0.0 && c.is_finite() { f + c } else { f }
}

/// Python's `round(v, digits)`: the exact binary value rounded half to even, in decimal.
pub fn py_round(v: f64, digits: usize) -> f64 {
    if !v.is_finite() {
        return v;
    }
    // the exact decimal expansion, then rounding on the digits
    let s = format!("{:.60}", v.abs());
    let Some((int, frac)) = s.split_once('.') else { return v };
    let (keep, rest) = frac.split_at(digits.min(frac.len()));
    let mut digits_str: Vec<u8> = int.bytes().chain(keep.bytes()).collect();
    let first = rest.bytes().next().unwrap_or(b'0');
    let tail_nonzero = rest.bytes().skip(1).any(|c| c != b'0');
    let last_odd = digits_str.last().is_some_and(|c| (c - b'0') % 2 == 1);
    let up = first > b'5' || (first == b'5' && (tail_nonzero || last_odd));
    if up {
        let mut i = digits_str.len();
        loop {
            if i == 0 {
                digits_str.insert(0, b'1');
                break;
            }
            i -= 1;
            match digits_str.get_mut(i) {
                Some(c) if *c == b'9' => *c = b'0',
                Some(c) => {
                    *c += 1;
                    break;
                }
                None => break,
            }
        }
    }
    let n = digits_str.len();
    let (a, b) = digits_str.split_at(n - keep.len());
    let text = format!("{}.{}", String::from_utf8_lossy(a), String::from_utf8_lossy(b));
    let r: f64 = text.parse().unwrap_or(v.abs());
    if v < 0.0 { -r } else { r }
}

/// The first code points of Unicode's blocks of decimal digits (category Nd), each followed by
/// the other nine: what Python's `\d` and `int()` accept.
const DECIMAL_ZEROS: &[u32] = &[
    0x30, 0x660, 0x6f0, 0x7c0, 0x966, 0x9e6, 0xa66, 0xae6, 0xb66, 0xbe6, 0xc66, 0xce6, 0xd66, 0xde6, 0xe50,
    0xed0, 0xf20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0, 0x1a80, 0x1a90, 0x1b50, 0x1bb0, 0x1c40,
    0x1c50, 0xa620, 0xa8d0, 0xa900, 0xa9d0, 0xa9f0, 0xaa50, 0xabf0, 0xff10, 0x104a0, 0x10d30, 0x10d40,
    0x11066, 0x110f0, 0x11136, 0x111d0, 0x112f0, 0x11450, 0x114d0, 0x11650, 0x116c0, 0x116d0, 0x116da,
    0x11730, 0x118e0, 0x11950, 0x11bf0, 0x11c50, 0x11d50, 0x11da0, 0x11f50, 0x16130, 0x16a60, 0x16ac0,
    0x16b50, 0x16d70, 0x1ccf0, 0x1d7ce, 0x1d7d8, 0x1d7e2, 0x1d7ec, 0x1d7f6, 0x1e140, 0x1e2f0, 0x1e4f0,
    0x1e5f1, 0x1e950, 0x1fbf0,
];

/// The value of a decimal digit as Python's regular expressions (`\d`) and `int()` see it,
/// in any script.
pub fn py_decimal(c: char) -> Option<u32> {
    let c = c as u32;
    DECIMAL_ZEROS.iter().find(|z| (**z..**z + 10).contains(&c)).map(|z| c - z)
}

/// Python's `str.isupper()`: at least one cased character, and no lowercase one.  (Titlecase
/// letters such as ǅ count as cased for Python, not here; names in diagrams don't use them.)
pub fn py_isupper(s: &str) -> bool {
    s.chars().any(|c| c.is_uppercase()) && !s.chars().any(|c| c.is_lowercase())
}

/// Python's `str[:1].upper() + str[1:]`.
pub fn cap_first(s: &str) -> String {
    let mut ch = s.chars();
    match ch.next() {
        Some(c) => c.to_uppercase().chain(ch).collect(),
        None => String::new(),
    }
}

/// Python's `str[:1].lower() + str[1:]`.
pub fn lower_first(s: &str) -> String {
    let mut ch = s.chars();
    match ch.next() {
        Some(c) => c.to_lowercase().chain(ch).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_as_python_writes_them() {
        assert_eq!(fmt_num(2.0), "2");
        assert_eq!(fmt_num(2.5), "2.5");
        assert_eq!(fmt_num(-0.0), "0");
        assert_eq!(fmt_num(-0.0001), "0");
        assert_eq!(fmt_g(1.5), "1.5");
        assert_eq!(py_round(2.675, 2), 2.67);
        assert_eq!(py_round(0.0625, 3), 0.062);
        assert_eq!(py_round(0.0635, 3), 0.064);
        assert_eq!(py_round(-1.0005, 3), -1.0);
        assert_eq!(py_round(9.9996, 3), 10.0);
        assert_eq!(py_round(9.25, 0), 9.0);
        assert_eq!(fmt_g(1e-7), "1e-07");
        assert_eq!(fmt_g(123456789.0), "1.23457e+08");
        assert_eq!(fmt_g(-2.0), "-2");
        assert_eq!(round_half_even(2.5), 2.0);
        assert_eq!(round_half_even(3.5), 4.0);
    }

    #[test]
    fn python_messages_and_modulo() {
        assert_eq!(py_float("x").unwrap_err(), "could not convert string to float: 'x'");
        assert_eq!(py_repr("it's"), "\"it's\"");
        assert_eq!(py_int(" 2 "), Ok(2));
        assert_eq!(py_int("2.0").unwrap_err(), "invalid literal for int() with base 10: '2.0'");
        assert_eq!(py_mod(-30.0, 360.0), 330.0);
        assert_eq!(py_mod(370.0, 360.0), 10.0);
    }

    #[test]
    fn strip_and_floats() {
        assert_eq!(strip(" \u{1c}a b\t"), "a b");
        assert_eq!(parse_float(" 1.5 "), Some(1.5));
        assert_eq!(parse_float("1."), Some(1.0));
        assert_eq!(parse_float("x"), None);
    }
}
