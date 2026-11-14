//! TikZ coordinates.

use crate::texscan::split_top;
use crate::text::{fmt_num, is_space, parse_float, strip};

/// A coordinate.  Numeric `(x,y)` pairs are editable; anything else (node names, calc
/// expressions, polar coordinates) is kept as written.
#[derive(Clone, Debug, PartialEq)]
pub enum Coord {
    Numeric { x: f64, y: f64 },
    Raw(String),
}

/// `^\s*[-+]?(\d+(\.\d*)?|\.\d+)\s*$`
fn is_number(s: &str) -> bool {
    let t = s.trim_matches(is_space);
    let t = t.strip_prefix(['+', '-']).unwrap_or(t);
    let (int, frac) = match t.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (t, None),
    };
    let digits = |p: &str| p.bytes().all(|b| b.is_ascii_digit());
    match frac {
        // \d+
        None => !int.is_empty() && digits(int),
        // \d+\.\d*  or  \.\d+
        Some(f) => digits(int) && digits(f) && (!int.is_empty() || !f.is_empty()),
    }
}

impl Coord {
    pub fn numeric(x: f64, y: f64) -> Self {
        Coord::Numeric { x, y }
    }

    pub fn parse(text: &str) -> Self {
        let parts = split_top(text, ',');
        if let [a, b] = parts.as_slice()
            && is_number(a)
            && is_number(b)
            && let (Some(x), Some(y)) = (parse_float(a), parse_float(b))
        {
            return Coord::Numeric { x, y };
        }
        Coord::Raw(strip(text).to_string())
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Coord::Numeric { .. })
    }

    pub fn to_tex(&self) -> String {
        match self {
            Coord::Raw(raw) => format!("({raw})"),
            Coord::Numeric { x, y } => format!("({},{})", fmt_num(*x), fmt_num(*y)),
        }
    }
}

/// `(0,1) (2,3) (a.north)` -> three coordinates.
pub fn parse_coord_list(text: &str) -> Vec<Coord> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(&c) = b.get(i) {
        if c == b'(' {
            let mut depth = 0i64;
            let mut j = i;
            while let Some(&d) = b.get(j) {
                if d == b'(' {
                    depth += 1;
                } else if d == b')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                j += 1;
            }
            out.push(Coord::parse(text.get(i + 1..j.min(text.len())).unwrap_or("")));
            i = j + 1;
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_raw() {
        assert_eq!(Coord::parse(" 1.5 , -2 "), Coord::numeric(1.5, -2.0));
        assert_eq!(Coord::parse(".5,3."), Coord::numeric(0.5, 3.0));
        assert_eq!(Coord::parse("1e2,3"), Coord::Raw("1e2,3".into()));
        assert_eq!(Coord::parse("a.north").to_tex(), "(a.north)");
        assert_eq!(Coord::numeric(1.25, -0.5).to_tex(), "(1.25,-0.5)");
    }
}
