//! xcolor expressions (`red!70!black`, `blue!20`, `gray!15`) -> RGB.

use crate::text::{parse_float, strip};

pub type Rgb = (f64, f64, f64);

/// xcolor's base colours and the dvipsnames used most in practice (approximate).
pub const NAMED: &[(&str, Rgb)] = &[
    ("red", (1.0, 0.0, 0.0)),
    ("green", (0.0, 1.0, 0.0)),
    ("blue", (0.0, 0.0, 1.0)),
    ("cyan", (0.0, 1.0, 1.0)),
    ("magenta", (1.0, 0.0, 1.0)),
    ("yellow", (1.0, 1.0, 0.0)),
    ("black", (0.0, 0.0, 0.0)),
    ("white", (1.0, 1.0, 1.0)),
    ("gray", (0.5, 0.5, 0.5)),
    ("darkgray", (0.25, 0.25, 0.25)),
    ("lightgray", (0.75, 0.75, 0.75)),
    ("brown", (0.75, 0.5, 0.25)),
    ("lime", (0.75, 1.0, 0.0)),
    ("olive", (0.5, 0.5, 0.0)),
    ("orange", (1.0, 0.5, 0.0)),
    ("pink", (1.0, 0.75, 0.75)),
    ("purple", (0.75, 0.0, 0.25)),
    ("teal", (0.0, 0.5, 0.5)),
    ("violet", (0.5, 0.0, 0.5)),
    ("ForestGreen", (0.13, 0.55, 0.13)),
    ("NavyBlue", (0.06, 0.46, 0.74)),
    ("RoyalBlue", (0.0, 0.5, 1.0)),
    ("Maroon", (0.68, 0.0, 0.16)),
    ("BrickRed", (0.71, 0.14, 0.0)),
    ("Orange", (1.0, 0.39, 0.13)),
    ("Goldenrod", (1.0, 0.9, 0.16)),
    ("Gray", (0.5, 0.5, 0.5)),
    ("Cerulean", (0.06, 0.89, 1.0)),
    ("Plum", (0.5, 0.0, 1.0)),
    ("OliveGreen", (0.24, 0.55, 0.0)),
];

fn named(name: &str) -> Option<Rgb> {
    let name = strip(strip(name).trim_matches(['{', '}']));
    let find = |n: &str| NAMED.iter().find(|(k, _)| *k == n).map(|(_, c)| *c);
    if let Some(c) = find(name).or_else(|| find(&name.to_lowercase())) {
        return Some(c);
    }
    if name.starts_with('#') && name.chars().count() == 7 {
        let channel = |a: usize| name.get(a..a + 2).and_then(|h| u8::from_str_radix(h, 16).ok());
        if let (Some(r), Some(g), Some(b)) = (channel(1), channel(3), channel(5)) {
            return Some((f64::from(r) / 255.0, f64::from(g) / 255.0, f64::from(b) / 255.0));
        }
    }
    None
}

/// Evaluates an xcolor mix expression: `a!p!b` is p% of a and the rest b; `a!p` is mixed with
/// white; `-a` is the complement.  `none` -> `None`; what can't be read -> `default`.
pub fn parse_color(expr: Option<&str>, default: Option<Rgb>) -> Option<Rgb> {
    let Some(expr) = expr else { return default };
    let expr = strip(expr);
    if expr.is_empty() {
        return default;
    }
    if expr == "none" {
        return None;
    }
    if let Some(rest) = expr.strip_prefix('-') {
        return parse_color(Some(rest), default).map(|(r, g, b)| (1.0 - r, 1.0 - g, 1.0 - b));
    }
    let parts: Vec<&str> = expr.split('!').map(strip).collect();
    let Some(mut cur) = parts.first().and_then(|p| named(p)) else { return default };
    let mut k = 1;
    while let Some(p) = parts.get(k) {
        let Some(pct) = parse_float(p).map(|v| v / 100.0) else { return default };
        let other = match parts.get(k + 1) {
            Some(name) => match named(name) {
                Some(c) => c,
                None => return default,
            },
            None => (1.0, 1.0, 1.0),
        };
        cur = (
            pct * cur.0 + (1.0 - pct) * other.0,
            pct * cur.1 + (1.0 - pct) * other.1,
            pct * cur.2 + (1.0 - pct) * other.2,
        );
        k += 2;
    }
    Some(cur)
}

/// The packages' colour shortcut: `blue` -> `blue!20`; mixes, white and none unchanged.
pub fn tint(expr: &str) -> String {
    let expr = strip(expr);
    if expr.contains('!') || expr == "white" || expr == "none" {
        expr.to_string()
    } else {
        format!("{expr}!20")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixes() {
        assert_eq!(parse_color(Some("blue!20"), None), Some((0.8, 0.8, 1.0)));
        let c = parse_color(Some("red!70!black"), None).unwrap();
        assert!((c.0 - 0.7).abs() < 1e-12 && c.1 == 0.0);
        assert_eq!(parse_color(Some("none"), Some((0.0, 0.0, 0.0))), None);
        assert_eq!(parse_color(Some("foo"), Some((0.0, 0.0, 0.0))), Some((0.0, 0.0, 0.0)));
        assert_eq!(tint("blue"), "blue!20");
        assert_eq!(tint("red!5"), "red!5");
    }
}
