//! What new diagrams start with.
//!
//! A setting is stored per diagram, so it travels with the file and affects LaTeX; these are
//! only the app's defaults for new ones.  A value the packages have as default isn't written.
//! Where the app keeps its settings is not the core's business: functions that read them take
//! a `get` function.

use crate::cardinality::{MIN_MAX, apply_changes, plan_diagram};
use crate::colors::parse_color;
use crate::document::{Document, Kind};
use crate::model::{Body, Options, Setup};
use crate::style::defaults;
use crate::text::{is_space, py_decimal, strip};

/// A main choice, offered as a switch.  The first choice is the packages' default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Toggle {
    pub key: &'static str,
    pub title: &'static str,
    /// (value, text)
    pub choices: &'static [(&'static str, &'static str)],
    pub kinds: &'static [Kind],
}

const ER: &[Kind] = &[Kind::Er];

/// The main choices.
pub const TOGGLES: &[Toggle] = &[
    Toggle {
        key: "sans serif",
        title: "Font",
        choices: &[("false", "serif"), ("true", "sans-serif")],
        kinds: &[Kind::Er, Kind::Gsm],
    },
    Toggle {
        key: "isa shape",
        title: "IS_A shape",
        choices: &[("hexagon", "hexagon"), ("diamond", "diamond")],
        kinds: ER,
    },
    Toggle {
        key: "constraint style",
        title: "IS_A type shown as",
        choices: &[("letters", "letters: cd"), ("words", "words: complete, disjoint")],
        kinds: ER,
    },
    Toggle {
        key: "attribute style",
        title: "Attributes",
        choices: &[("compact", "compact lists"), ("chen", "ovals (Chen)")],
        kinds: ER,
    },
    Toggle {
        key: "show types",
        title: "Data types",
        choices: &[("false", "hidden"), ("true", "shown")],
        kinds: ER,
    },
    Toggle {
        key: "card notation",
        title: "Cardinality notation",
        choices: &[("n", "n-notation: 1, c, n, cn"), ("minmax", "min-max: (0,*), (1,1), …")],
        kinds: ER,
    },
    Toggle {
        key: "card symbols",
        title: "Cardinalities drawn as",
        choices: &[("labels", "labels"), ("crowsfoot", "crow's foot symbols")],
        kinds: ER,
    },
];

/// Not defaults of a diagram: the language is a setting of the app.
pub const NOT_OFFERED: &[&str] = &["label language"];

/// The other style keys, by what they belong to: the first group whose prefix fits.
fn groups(kind: Kind) -> &'static [(&'static str, &'static [&'static str])] {
    match kind {
        Kind::Er => &[
            ("Entities", &["entity ", "weak "]),
            ("Relationships", &["rel ", "dependency "]),
            ("Generalisation and aggregation", &["isa ", "canbe ", "partof ", "constraint "]),
            ("Attributes", &["attr ", "attribute ", "chen ", "type "]),
            ("Cardinalities and roles", &["card ", "crow ", "role "]),
            ("Lines", &["edge ", "link "]),
            ("Notes and subdomains", &["note ", "subdomain "]),
            ("Other", &[""]),
        ],
        Kind::Gsm => &[
            ("Abstract types", &["abstract "]),
            ("Representable types", &["repr "]),
            ("Subtypes", &["sub "]),
            ("Aggregations", &["aggr "]),
            ("Lines and arrows", &["edge ", "arrow ", "generalization ", "specialization "]),
            ("Edge labels", &["label "]),
            ("Notes and subdomains", &["note ", "subdomain "]),
            ("Other", &[""]),
        ],
    }
}

/// Keys with a fixed set of values.
pub const ENUMS: &[(&str, &[&str])] = &[
    ("card placement", &["inline", "beside"]),
    ("card side", &["auto", "left", "right"]),
    ("label side", &["auto", "left", "right"]),
    ("rule warnings", &["true", "false"]),
    ("label sloped", &["true", "false"]),
];

const LENGTH_WORDS: &[&str] = &[
    "width", "height", "sep", "distance", "spread", "gap", "tick", "stem", "indent", "dist", "length",
    "size", "tuck", "vpad", "hpad", "radius",
];

fn enum_values(key: &str) -> Option<&'static [&'static str]> {
    ENUMS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// What sort of value a style key takes, for checking it and to show a colour: `choice`,
/// `text`, `number`, `color`, `font` or `length`.
pub fn key_type(key: &str) -> &'static str {
    let last = key.split(is_space).rfind(|w| !w.is_empty()).unwrap_or("");
    if enum_values(key).is_some() {
        "choice"
    } else if key == "type sep" {
        "text"
    } else if matches!(key, "rel aspect" | "isa slant" | "abstract apex angle" | "label pos") {
        "number"
    } else if matches!(last, "fill" | "draw" | "text" | "color") {
        "color"
    } else if last == "font" {
        "font"
    } else if LENGTH_WORDS.contains(&last) {
        "length"
    } else {
        "text"
    }
}

/// `-?(\d+\.?\d*|\.\d+)` at the start: its length in chars.
fn number_at(chars: &[char]) -> Option<usize> {
    let mut i = usize::from(chars.first() == Some(&'-'));
    let digits = |from: usize| chars.iter().skip(from).take_while(|c| py_decimal(**c).is_some()).count();
    let int = digits(i);
    if int > 0 {
        i += int;
        if chars.get(i) == Some(&'.') {
            i += 1 + digits(i + 1);
        }
        Some(i)
    } else if chars.get(i) == Some(&'.') && digits(i + 1) > 0 {
        Some(i + 1 + digits(i + 1))
    } else {
        None
    }
}

fn is_number(value: &str) -> bool {
    let chars: Vec<char> = value.chars().collect();
    let skip = chars.iter().take_while(|c| is_space(**c)).count();
    let rest = chars.get(skip..).unwrap_or(&[]);
    number_at(rest).is_some_and(|n| rest.iter().skip(n).all(|c| is_space(*c)))
}

fn is_length(value: &str) -> bool {
    let chars: Vec<char> = value.chars().collect();
    let skip = chars.iter().take_while(|c| is_space(**c)).count();
    let rest = chars.get(skip..).unwrap_or(&[]);
    let Some(n) = number_at(rest) else { return false };
    let rest = rest.get(n..).unwrap_or(&[]);
    let skip = rest.iter().take_while(|c| is_space(**c)).count();
    let rest: String = rest.get(skip..).unwrap_or(&[]).iter().collect();
    ["pt", "cm", "mm", "em", "ex", "in", "bp"]
        .iter()
        .any(|u| rest.strip_prefix(u).is_some_and(|after| after.chars().all(is_space)))
}

/// What's wrong with this value, or `None`.
pub fn check_value(key: &str, value: &str) -> Option<String> {
    let value = strip(value);
    if value.is_empty() {
        return None;
    }
    match key_type(key) {
        "choice" => {
            let values = enum_values(key).unwrap_or(&[]);
            (!values.contains(&value)).then(|| format!("{key}: one of {}", values.join(", ")))
        }
        "number" => (!is_number(value)).then(|| format!("{key}: a number, such as 1.5")),
        "length" => (!is_length(value)).then(|| format!("{key}: a length, such as 0.5cm or 2pt")),
        "color" => (value != "none" && parse_color(Some(value), None).is_none())
            .then(|| format!("{key}: a colour, such as blue!20 or red!70!black")),
        "font" => {
            (!value.starts_with('\\')).then(|| format!("{key}: font commands, such as \\small\\bfseries"))
        }
        _ => None,
    }
}

/// The main choices for a kind of diagram.
pub fn toggles(kind: Kind) -> Vec<&'static Toggle> {
    TOGGLES.iter().filter(|t| t.kinds.contains(&kind)).collect()
}

/// The packages' default of a key (`None` for keys the packages don't have).
pub fn default_value(kind: Kind, key: &str) -> Option<&'static str> {
    if let Some(t) = TOGGLES.iter().find(|t| t.key == key) {
        return t.choices.first().map(|c| c.0);
    }
    defaults(kind).iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// All style keys that aren't main choices, by group.
pub fn style_groups(kind: Kind) -> Vec<(&'static str, Vec<&'static str>)> {
    let shown: Vec<&str> = toggles(kind).iter().map(|t| t.key).chain(NOT_OFFERED.iter().copied()).collect();
    let mut out: Vec<(&'static str, Vec<&'static str>)> =
        groups(kind).iter().map(|(t, _)| (*t, vec![])).collect();
    for (key, _) in defaults(kind) {
        if shown.contains(key) {
            continue;
        }
        let title = groups(kind)
            .iter()
            .find(|(_, prefixes)| prefixes.iter().any(|p| key.starts_with(p)))
            .map(|g| g.0);
        if let Some(slot) = out.iter_mut().find(|(t, _)| Some(*t) == title) {
            slot.1.push(key);
        }
    }
    out.retain(|(_, keys)| !keys.is_empty());
    out
}

/// Every key the preferences offer, main choices first.
pub fn all_keys(kind: Kind) -> Vec<&'static str> {
    toggles(kind)
        .iter()
        .map(|t| t.key)
        .chain(style_groups(kind).into_iter().flat_map(|(_, keys)| keys))
        .collect()
}

/// The name under which the app keeps a default.
pub fn setting_name(kind: Kind, key: &str) -> String {
    format!("new diagram/{}/{key}", kind.name())
}

/// What the app keeps; the packages' default if nothing, or nothing valid, is kept.
pub fn stored_value(get: &dyn Fn(&str) -> Option<String>, kind: Kind, key: &str) -> String {
    let default = default_value(kind, key).unwrap_or("").to_string();
    let value = get(&setting_name(kind, key)).unwrap_or_else(|| default.clone());
    if let Some(t) = toggles(kind).iter().find(|t| t.key == key)
        && !t.choices.iter().any(|(v, _)| *v == value)
    {
        return default;
    }
    if check_value(key, &value).is_none() { value } else { default }
}

/// Into the diagram's `\setup`: what differs from the packages, the rest removed.
pub fn write_values(doc: &mut Document, values: &[(String, String)]) {
    let existing = doc.elements.iter().find(|e| matches!(e.body, Body::Setup(_))).map(|e| e.key);
    let mut opts = match existing.and_then(|k| doc.get(k)).map(|e| &e.body) {
        Some(Body::Setup(s)) => s.opts.clone(),
        _ => Options::new(),
    };
    for (key, value) in values {
        let value = if key_type(key) != "text" { strip(value) } else { value.as_str() }; // a text may end in a space on purpose
        if strip(value).is_empty() || Some(value) == default_value(doc.kind, key) {
            opts.remove(key);
        } else {
            opts.set(key, if value == "true" { None } else { Some(value) });
        }
    }
    match existing {
        Some(key) => {
            let empty = opts.is_empty();
            if let Some(el) = doc.get_mut(key) {
                if let Body::Setup(s) = &mut el.body {
                    s.opts = opts;
                }
                el.touch();
            }
            if empty {
                doc.remove(key);
            }
        }
        None if !opts.is_empty() => {
            doc.insert(0, Body::Setup(Setup { opts }));
        }
        None => {}
    }
}

/// A new diagram starts with what the app keeps for its kind.
pub fn apply_defaults(doc: &mut Document, get: &dyn Fn(&str) -> Option<String>) {
    let kind = doc.kind;
    let mut values: Vec<(String, String)> =
        all_keys(kind).into_iter().map(|k| (k.to_string(), stored_value(get, kind, k))).collect();
    if values.iter().any(|(k, v)| k == "card notation" && v == MIN_MAX) {
        // crow's foot is for n-notation only
        for (k, v) in &mut values {
            if k == "card symbols" {
                v.clear();
            }
        }
    }
    write_values(doc, &values);
}

/// The same for a diagram that already has content (GSM to ER).  Its labels have to go with
/// the notation.  Returns what couldn't be converted.
pub fn apply_defaults_to_generated(doc: &mut Document, get: &dyn Fn(&str) -> Option<String>) -> Vec<String> {
    let mut notes = Vec::new();
    if stored_value(get, doc.kind, "card notation") == MIN_MAX {
        let (changes, skipped) = plan_diagram(doc, MIN_MAX);
        apply_changes(doc, &changes);
        notes = skipped.into_iter().map(|(name, why)| format!("{name} stays in n-notation: {why}")).collect();
    }
    let first = doc.elements.first().map(|e| e.key);
    apply_defaults(doc, get);
    if let Some(first) = first
        && doc.elements.first().map(|e| e.key) != Some(first)
    {
        // the comment at the top stays on top
        let leading = doc.get_mut(first).map(|e| std::mem::take(&mut e.meta.leading)).unwrap_or_default();
        if let Some(top) = doc.elements.first_mut() {
            top.meta.leading = leading;
        }
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checking_values() {
        assert_eq!(key_type("entity fill"), "color");
        assert_eq!(key_type("rel aspect"), "number");
        assert_eq!(key_type("card placement"), "choice");
        assert_eq!(key_type("entity min width"), "length");
        assert_eq!(check_value("entity min width", "2 cm"), None);
        assert!(check_value("entity min width", "2").is_some());
        assert_eq!(check_value("rel aspect", " -.5 "), None);
        assert!(check_value("card placement", "above").is_some());
        assert_eq!(check_value("entity fill", "blue!20"), None);
    }
}
