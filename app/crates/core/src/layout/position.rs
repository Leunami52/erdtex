//! Coordinates, anchors and TikZ's positioning keys (`right=1cm of a`).

use super::prims::Gfx;
use super::{Fallible, Layout, length};
use crate::geometry::{CM, COMPASS, Pt, Shape};
use crate::model::{Coord, Options};
use crate::text::{is_space, py_float, strip};

/// `[-+]?\d*\.?\d+` at the start of `s`: its length, or 0.
fn number_prefix(s: &str) -> usize {
    let b = s.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    let int = b.iter().skip(i).take_while(|c| c.is_ascii_digit()).count();
    i += int;
    if b.get(i) == Some(&b'.') {
        let frac = b.iter().skip(i + 1).take_while(|c| c.is_ascii_digit()).count();
        if frac > 0 {
            return i + 1 + frac;
        }
    }
    // without a fraction, \d+ has to be the digits (backtracking gives the same)
    if int > 0 { i } else { 0 }
}

/// The `(sign, inner)` pairs of `([+-]?)\s*\(([^()]*)\)` in `s`, found as `re.findall` does.
fn calc_terms(s: &str) -> Vec<(char, String)> {
    let chars: Vec<char> = s.chars().collect();
    let try_at = |start: usize| -> Option<(char, String, usize)> {
        let attempt = |mut j: usize, sign: char| -> Option<(char, String, usize)> {
            while chars.get(j).is_some_and(|c| is_space(*c)) {
                j += 1;
            }
            if chars.get(j) != Some(&'(') {
                return None;
            }
            let mut k = j + 1;
            let mut inner = String::new();
            loop {
                match chars.get(k) {
                    Some(')') => return Some((sign, inner, k + 1)),
                    Some('(') | None => return None,
                    Some(c) => inner.push(*c),
                }
                k += 1;
            }
        };
        match chars.get(start) {
            Some(&c @ ('+' | '-')) => attempt(start + 1, c).or_else(|| attempt(start, ' ')),
            _ => attempt(start, ' '),
        }
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match try_at(i) {
            Some((sign, inner, end)) => {
                out.push((sign, inner));
                i = end.max(i + 1);
            }
            None => i += 1,
        }
    }
    out
}

/// `\s*(.*?)\s*of\s+(.+?)\s*` as `re.fullmatch` reads it: (distance, reference).
fn split_of(value: &str) -> Option<(String, String)> {
    let v = value.trim_start_matches(is_space);
    let ws = |s: &str| s.chars().take_while(|c| is_space(*c)).map(char::len_utf8).sum::<usize>();
    for (at, _) in v.match_indices("of") {
        let dist = v.get(..at).unwrap_or("").trim_end_matches(is_space);
        let after = v.get(at + 2..).unwrap_or("");
        let gap = ws(after);
        if gap == 0 || dist.contains('\n') {
            continue;
        }
        let rest = after.get(gap..).unwrap_or("");
        let reference = if rest.is_empty() {
            // the reference takes one of the spaces, if there are at least two
            let spaces: Vec<char> = after.chars().collect();
            if spaces.len() < 2 {
                continue;
            }
            spaces.last().map(|c| c.to_string()).unwrap_or_default()
        } else {
            rest.trim_end_matches(is_space).to_string()
        };
        if reference.contains('\n') || reference.is_empty() {
            continue;
        }
        return Some((dist.to_string(), reference));
    }
    None
}

impl Layout<'_> {
    /// The absolute point of a coordinate (numeric in cm, or an anchor or simple calc).
    pub fn coord(&self, c: Option<&Coord>) -> Option<Pt> {
        match c? {
            Coord::Numeric { x, y } => Some((x * CM, y * CM)),
            Coord::Raw(raw) => self.eval_point(raw),
        }
    }

    pub(super) fn eval_point(&self, text: &str) -> Option<Pt> {
        let text = strip(text);
        if text.len() >= 2 && text.starts_with('$') && text.ends_with('$') {
            let inner = text.get(1..text.len() - 1).unwrap_or("");
            if !inner.contains('\n') {
                // calc: (a)+(1,2)-(b)
                let mut total = (0.0, 0.0);
                for (sign, term) in calc_terms(inner) {
                    let p = self.eval_point(&term)?;
                    let k = if sign == '-' { -1.0 } else { 1.0 };
                    total.0 += k * p.0;
                    total.1 += k * p.1;
                }
                return Some(total);
            }
        }
        let parts: Vec<&str> = text.split(',').map(strip).collect();
        if let [a, b] = parts.as_slice() {
            return Some((length(Some(a), 0.0), length(Some(b), 0.0)));
        }
        // polar: angle:radius
        let n = number_prefix(text);
        if n > 0
            && let Some(radius) = text.get(n..).and_then(|r| r.strip_prefix(':'))
            && !radius.is_empty()
            && !radius.contains('\n')
            && let Some(a) = text.get(..n).and_then(|a| a.parse::<f64>().ok())
        {
            let r = length(Some(radius), 0.0);
            let a = a.to_radians();
            return Some((r * a.cos(), r * a.sin()));
        }
        self.ref_point(text)
    }

    pub(super) fn ref_point(&self, reference: &str) -> Option<Pt> {
        let (ident, anchor) = reference.split_once('.').unwrap_or((reference, ""));
        let shape = self.shapes.get(strip(ident))?;
        Some(if anchor.is_empty() { shape.center() } else { shape.anchor(anchor) })
    }

    /// The centre of a node: its position, or TikZ's `right=... of x`.
    pub(super) fn node_position(
        &self,
        pos: Option<&Coord>,
        opts: &Options,
        size_shape: &Shape,
        g: &mut Gfx,
    ) -> Pt {
        let mut at = self.coord(pos);
        if at.is_none()
            && let Some(p) = pos
        {
            g.warnings.push(format!("cannot evaluate position {}", p.to_tex()));
        }
        let mut anchor = opts.get("anchor").map(str::to_string);
        for key in
            ["above right", "above left", "below right", "below left", "right", "left", "above", "below"]
        {
            if opts.contains(key) {
                if let Some(target) = self.positioning(key, opts.get(key).unwrap_or(""), size_shape, g) {
                    at = Some(target);
                    anchor = None;
                }
                break;
            }
        }
        let mut at = at.unwrap_or((0.0, 0.0));
        if let Some(a) = anchor.as_deref().filter(|a| !a.is_empty() && COMPASS.iter().any(|(n, _)| n == a)) {
            let (ax, ay) = size_shape.anchor(a);
            at = (at.0 - (ax - size_shape.cx), at.1 - (ay - size_shape.cy));
        }
        if opts.contains("xshift") {
            at.0 += length(opts.get("xshift"), 0.0);
        }
        if opts.contains("yshift") {
            at.1 += length(opts.get("yshift"), 0.0);
        }
        at
    }

    /// TikZ's positioning library: `right=1cm of a`, `above left=of b`, ...
    fn positioning(&self, key: &str, value: &str, shape: &Shape, g: &mut Gfx) -> Option<Pt> {
        let (dist, reference) = match split_of(value) {
            Some((d, r)) => (d, Some(r)),
            None => (value.to_string(), None),
        };
        let dists: Vec<&str> =
            if strip(&dist).is_empty() { vec![] } else { dist.split("and").map(strip).collect() };
        let dy_d = dists.first().map_or(CM, |d| length(Some(d), 0.0));
        let mut dx_d = dists.get(1).map_or(dy_d, |d| length(Some(d), 0.0));
        let vert: f64 = if key.contains("above") {
            1.0
        } else if key.contains("below") {
            -1.0
        } else {
            0.0
        };
        let horz: f64 = if key.contains("right") {
            1.0
        } else if key.contains("left") {
            -1.0
        } else {
            0.0
        };
        if horz != 0.0 && vert == 0.0 {
            dx_d = dy_d;
        }
        let my_anchor = match (horz as i32, vert as i32) {
            (0, 1) => "south",
            (0, -1) => "north",
            (1, 0) => "west",
            (-1, 0) => "east",
            (1, 1) => "south west",
            (-1, 1) => "south east",
            (1, -1) => "north west",
            _ => "north east",
        };
        let base = match reference {
            None => (0.0, 0.0),
            Some(r) => {
                let ident = strip(r.split('.').next().unwrap_or(""));
                let Some(other) = self.shapes.get(ident) else {
                    g.warnings.push(format!("unknown node '{ident}' in {key}="));
                    return None;
                };
                let opposite = match my_anchor {
                    "south" => "north",
                    "north" => "south",
                    "west" => "east",
                    "east" => "west",
                    "south west" => "north east",
                    "south east" => "north west",
                    "north west" => "south east",
                    _ => "south west",
                };
                other.anchor(opposite)
            }
        };
        let target = (base.0 + horz * dx_d, base.1 + vert * dy_d);
        let (ax, ay) = shape.anchor(my_anchor);
        Some((target.0 - (ax - shape.cx), target.1 - (ay - shape.cy)))
    }
}

/// Python's `float(x)` for an option value that is a number or missing.
pub(super) fn float_opt(v: Option<&str>, default: f64) -> Fallible<f64> {
    match v.filter(|s| !s.is_empty()) {
        Some(s) => py_float(s),
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_expressions_as_python_reads_them() {
        assert_eq!(
            calc_terms("(a)+(1,2)-(b)"),
            vec![(' ', "a".into()), ('+', "1,2".into()), ('-', "b".into())]
        );
        assert_eq!(calc_terms("+x(a)"), vec![(' ', "a".into())]);
        assert_eq!(split_of("2cm of a"), Some(("2cm".into(), "a".into())));
        assert_eq!(split_of("of b.north"), Some(("".into(), "b.north".into())));
        assert_eq!(split_of("2cm"), None);
        assert_eq!(number_prefix("30:2cm"), 2);
        assert_eq!(number_prefix("1."), 1);
    }
}
