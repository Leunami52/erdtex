//! The attributes of an attribute list: `*key`, `+candidate key`, `/derived = {formula}`,
//! `\{multi-valued\}`, `composite{part, part}`, `name : type`, `name@x/y`.

use crate::texscan::{split_tex, strip_braces};
use crate::text::{fmt_g, is_space, parse_float, strip};

/// One attribute of a list.  `pos` is the place of its oval relative to its owner, in cm
/// (only the Chen style uses it).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AttrItem {
    pub text: String,
    pub key: bool,
    /// data type (`name : type`); `None` = not set
    pub r#type: Option<String>,
    pub multivalued: bool,
    pub derived: bool,
    /// how a derived attribute is computed (SQL)
    pub formula: Option<String>,
    pub parts: Vec<AttrItem>,
    pub pos: Option<(f64, f64)>,
    /// a further candidate key
    pub unique: bool,
}

/// An item that can't be read (only a position that isn't a number).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemError(pub String);

impl std::fmt::Display for ItemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// `PID : integer` -> (`PID`, `integer`): split at the last colon outside braces, as the
/// package does.  No colon -> (text, None).
pub fn split_type(text: &str) -> (String, Option<String>) {
    let parts = split_tex(text, ':');
    match parts.split_last() {
        Some((last, rest)) if !rest.is_empty() => {
            let ty = strip_braces(strip(last));
            (strip(&rest.join(":")).to_string(), (!ty.is_empty()).then(|| ty.to_string()))
        }
        _ => (strip(text).to_string(), None),
    }
}

fn needs_braces(text: &str, sep: char) -> bool {
    split_tex(text, sep).len() > 1
}

/// `\+[^\d\s]` at the start: `+email`, but not `+49 phone`.
fn unique_mark(p: &str) -> bool {
    let mut c = p.chars();
    c.next() == Some('+') && c.next().is_some_and(|d| !d.is_numeric() && !is_space(d))
}

/// `address{street, town}` -> (`address`, `street, town`).  Not a command's argument
/// (`\textit{x}`), a subscript (`x_{i}`), or a group in the middle (`ratio 1{:}2`).
fn split_composite(label: &str) -> Option<(String, String)> {
    let chars: Vec<char> = label.chars().collect();
    let mut i = 0;
    while i < chars.len() && chars.get(i) != Some(&'{') {
        i += if chars.get(i) == Some(&'\\') { 2 } else { 1 };
    }
    if i >= chars.len() || !label.ends_with('}') {
        return None;
    }
    let name: String = chars.get(..i)?.iter().collect();
    let name = strip(&name).to_string();
    if name.is_empty() || name.contains('\\') || name.ends_with(['_', '^']) {
        return None;
    }
    let mut depth = 0i64;
    for k in i..chars.len() {
        match chars.get(k) {
            Some('{') => depth += 1,
            Some('}') => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            let inner: String = chars.get(i + 1..k)?.iter().collect();
            if k != chars.len() - 1 || strip(&inner).is_empty() {
                return None;
            }
            return Some((name, inner));
        }
    }
    None
}

/// `@([-+]?[\d.]+)/([-+]?[\d.]+)\s*$`, found as Python's `re.search` finds it: the leftmost
/// `@` from which the rest matches.  -> (byte offset of the `@`, x text, y text).
fn item_position(label: &str) -> Option<(usize, &str, &str)> {
    fn number(s: &str) -> usize {
        let sign = usize::from(s.starts_with(['+', '-']));
        let body = s.get(sign..).unwrap_or("");
        let n = body.bytes().take_while(|b| b.is_ascii_digit() || *b == b'.').count();
        if n == 0 { 0 } else { sign + n }
    }
    for (at, _) in label.match_indices('@') {
        let rest = label.get(at + 1..).unwrap_or("");
        let nx = number(rest);
        if nx == 0 || rest.as_bytes().get(nx) != Some(&b'/') {
            continue;
        }
        let after = rest.get(nx + 1..).unwrap_or("");
        let ny = number(after);
        if ny == 0 || !after.get(ny..).unwrap_or("").chars().all(is_space) {
            continue;
        }
        return Some((at, rest.get(..nx).unwrap_or(""), after.get(..ny).unwrap_or("")));
    }
    None
}

fn float_or_error(s: &str) -> Result<f64, ItemError> {
    parse_float(s).ok_or_else(|| ItemError(format!("could not convert string to float: '{s}'")))
}

impl AttrItem {
    pub fn new(text: &str) -> Self {
        AttrItem { text: text.to_string(), ..Default::default() }
    }

    pub fn parse(part: &str) -> Result<Self, ItemError> {
        let mut p = strip_braces(strip(part)).to_string(); // a clist strips one brace layer
        let key = p.starts_with('*');
        let unique = unique_mark(&p);
        if key || unique {
            let mut c = p.chars();
            c.next();
            p = strip(c.as_str()).to_string();
        }
        let (label, ty) = split_type(&p);
        let mut label = strip_braces(&label).to_string();
        let mut item = AttrItem { key, r#type: ty, unique, ..Default::default() };
        if label.starts_with('/') && label.chars().count() > 1 {
            item.derived = true;
            let pieces = split_tex(label.get(1..).unwrap_or(""), '=');
            let first = pieces.first().map_or("", String::as_str);
            if pieces.len() > 1 {
                let formula = pieces.get(1..).map_or(String::new(), |p| p.join("="));
                let formula = strip_braces(&formula);
                item.formula = (!formula.is_empty()).then(|| formula.to_string());
            }
            label = strip(first).to_string();
        }
        if label.starts_with("\\{") && label.ends_with("\\}") && label.chars().count() > 4 {
            item.multivalued = true;
            label = strip(label.get(2..label.len() - 2).unwrap_or("")).to_string();
        }
        if let Some((name, inner)) = split_composite(&label) {
            label = name;
            item.parts = split_tex(&inner, ',')
                .iter()
                .filter(|x| !strip(x).is_empty())
                .map(|x| AttrItem::parse(x))
                .collect::<Result<_, _>>()?;
        }
        if let Some((at, x, y)) = item_position(&label) {
            item.pos = Some((float_or_error(x)?, float_or_error(y)?));
            label = strip(label.get(..at).unwrap_or("")).to_string();
        }
        item.text = label;
        Ok(item)
    }

    /// The name as a compact list prints it.
    pub fn shown(&self) -> String {
        if self.derived {
            format!("/{}", self.text)
        } else if self.multivalued {
            format!("\\{{{}\\}}", self.text)
        } else {
            self.text.clone()
        }
    }

    pub fn to_tex(&self) -> String {
        let mut t = self.text.clone();
        if let Some((x, y)) = self.pos {
            t += &format!("@{}/{}", fmt_g(x), fmt_g(y));
        }
        if !self.parts.is_empty() {
            let parts: Vec<String> = self.parts.iter().map(AttrItem::to_tex).collect();
            t += &format!("{{{}}}", parts.join(", "));
        }
        if self.multivalued {
            t = format!("\\{{{t}\\}}");
        }
        if self.derived {
            t = format!("/{t}");
            if let Some(f) = &self.formula {
                t += &format!(" = {{{f}}}");
            }
        }
        if let Some(ty) = &self.r#type {
            if needs_braces(&t, ',') || needs_braces(&t, ':') || t.starts_with('*') {
                t = format!("{{{t}}}");
            }
            let ty =
                if needs_braces(ty, ',') || needs_braces(ty, ':') { format!("{{{ty}}}") } else { ty.clone() };
            t = format!("{t} : {ty}");
        } else if needs_braces(&t, ',') || t.starts_with(['*', '{']) {
            t = format!("{{{t}}}");
        }
        let mark = if self.key {
            "*"
        } else if self.unique {
            "+"
        } else {
            ""
        };
        format!("{mark}{t}")
    }
}

/// Top to bottom, as a list draws them: (item, depth).
pub fn flatten_items(items: &[AttrItem]) -> Vec<(&AttrItem, usize)> {
    fn walk<'a>(items: &'a [AttrItem], depth: usize, out: &mut Vec<(&'a AttrItem, usize)>) {
        for it in items {
            out.push((it, depth));
            walk(&it.parts, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    walk(items, 0, &mut out);
    out
}

/// The opposite of `flatten_items`; the items' own parts are replaced.
pub fn nest_items(flat: Vec<(AttrItem, usize)>) -> Vec<AttrItem> {
    // a stack of open items; closing one hands it to its parent
    let mut top: Vec<AttrItem> = Vec::new();
    let mut stack: Vec<AttrItem> = Vec::new();
    fn close(stack: &mut Vec<AttrItem>, top: &mut Vec<AttrItem>) {
        if let Some(done) = stack.pop() {
            match stack.last_mut() {
                Some(parent) => parent.parts.push(done),
                None => top.push(done),
            }
        }
    }
    for (mut it, depth) in flat {
        let depth = depth.min(stack.len());
        while stack.len() > depth {
            close(&mut stack, &mut top);
        }
        it.parts.clear();
        stack.push(it);
    }
    while !stack.is_empty() {
        close(&mut stack, &mut top);
    }
    top
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_of_items() {
        let it = AttrItem::parse("*PID : integer").unwrap();
        assert!(it.key && it.text == "PID" && it.r#type.as_deref() == Some("integer"));
        assert!(AttrItem::parse("+email").unwrap().unique);
        assert!(!AttrItem::parse("+49 phone").unwrap().unique);
        let d = AttrItem::parse("/age = {2026 - born}").unwrap();
        assert!(d.derived && d.formula.as_deref() == Some("2026 - born"));
        let c = AttrItem::parse("address{street, town}").unwrap();
        assert_eq!(c.parts.len(), 2);
        assert_eq!(c.to_tex(), "address{street, town}");
        assert_eq!(AttrItem::parse("n@1/2@3/4").unwrap().pos, Some((3.0, 4.0)));
        assert_eq!(AttrItem::parse("\\textit{c}").unwrap().parts.len(), 0);
    }

    #[test]
    fn flatten_and_nest_are_opposites() {
        let items = vec![AttrItem::parse("a{b, c{d}}").unwrap(), AttrItem::parse("e").unwrap()];
        let flat: Vec<(AttrItem, usize)> =
            flatten_items(&items).into_iter().map(|(i, d)| (i.clone(), d)).collect();
        assert_eq!(nest_items(flat), items);
    }
}
