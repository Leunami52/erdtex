//! The panels' data as JSON for the page (written by hand: a JSON library would add more to
//! the download than these few lines).

use diagram_core::colors::Rgb;
use diagram_core::gsm2er::Conversion;
use diagram_core::panel::{AttrRow, CardField, Field, Form, Item, LineageRow, Outcome};
use diagram_core::problems::Listed;
use diagram_core::rmwindow;
use diagram_core::rules::Severity;
use std::fmt::Write;

/// A JSON string.
pub fn s(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn num(v: f64) -> String {
    if v.is_finite() { format!("{v}") } else { "null".into() }
}

fn opt(v: Option<&str>) -> String {
    v.map_or("null".into(), s)
}

fn rgb(c: Option<Rgb>) -> String {
    match c {
        Some((r, g, b)) => {
            let ch = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            s(&format!("#{:02x}{:02x}{:02x}", ch(r), ch(g), ch(b)))
        }
        None => "null".into(),
    }
}

/// `{"key": value, ...}` from pairs of a key and a JSON value.
fn obj(pairs: &[(&str, String)]) -> String {
    let inner: Vec<String> = pairs.iter().map(|(k, v)| format!("{}:{v}", s(k))).collect();
    format!("{{{}}}", inner.join(","))
}

fn arr(items: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", items.into_iter().collect::<Vec<_>>().join(","))
}

fn items(list: &[Item]) -> String {
    arr(list.iter().map(|i| obj(&[("value", s(&i.value)), ("text", s(&i.text)), ("tip", s(&i.tip))])))
}

fn severity(sv: Severity) -> String {
    s(sv.as_str())
}

fn attr_row(r: &AttrRow) -> String {
    obj(&[
        ("depth", r.depth.to_string()),
        ("name", s(&r.name)),
        ("key", r.key.to_string()),
        ("unique", r.unique.to_string()),
        ("type", s(&r.r#type)),
        ("guess", s(&r.guess)),
        ("composite", r.composite.to_string()),
        ("kind", s(r.kind)),
        ("formula", s(&r.formula)),
    ])
}

fn lineage_row(r: &LineageRow) -> String {
    obj(&[
        ("depth", r.depth.to_string()),
        ("name", s(&r.name)),
        ("key", r.key.to_string()),
        ("weakKey", r.weak_key.to_string()),
        ("unique", r.unique.to_string()),
        ("kind", s(r.kind)),
        ("type", s(&r.r#type)),
        ("from", s(&r.from)),
        ("trace", s(&r.trace)),
        ("target", s(&r.target)),
    ])
}

fn card(c: &CardField) -> Vec<(&'static str, String)> {
    vec![
        ("notation", s(c.notation)),
        ("choices", items(&c.choices)),
        ("current", opt(c.current.as_deref())),
        ("lo", s(&c.lo)),
        ("hi", s(&c.hi)),
        ("note", opt(c.note.as_deref())),
        ("sentence", s(&c.sentence)),
        ("deviation", c.deviation.as_ref().map_or("null".into(), |(t, tip)| arr([s(t), s(tip)]))),
    ]
}

fn field(f: &Field) -> Vec<(&'static str, String)> {
    let b = |v: bool| v.to_string();
    match f {
        Field::Text { text, button, tip } => {
            vec![("type", s("text")), ("text", s(text)), ("button", opt(button.as_deref())), ("tip", s(tip))]
        }
        Field::Problem { severity: sv, message, fix } => {
            vec![
                ("type", s("problem")),
                ("severity", severity(*sv)),
                ("message", s(message)),
                ("fix", opt(fix.as_deref())),
            ]
        }
        Field::Check { text, value, enabled, tip } => vec![
            ("type", s("check")),
            ("text", s(text)),
            ("value", b(*value)),
            ("enabled", b(*enabled)),
            ("tip", s(tip)),
        ],
        Field::Choice { items: list, current, editable, tip } => vec![
            ("type", s("choice")),
            ("items", items(list)),
            ("current", s(current)),
            ("editable", b(*editable)),
            ("tip", s(tip)),
        ],
        Field::Line { value, placeholder, tip } => {
            vec![("type", s("line")), ("value", s(value)), ("placeholder", s(placeholder)), ("tip", s(tip))]
        }
        Field::Label { value, tip } => vec![("type", s("label")), ("value", s(value)), ("tip", s(tip))],
        Field::Number { value, min, max, step, decimals, suffix } => vec![
            ("type", s("number")),
            ("value", num(*value)),
            ("min", num(*min)),
            ("max", num(*max)),
            ("step", num(*step)),
            ("decimals", decimals.to_string()),
            ("suffix", s(suffix)),
        ],
        Field::Position { x, y, step } => {
            vec![("type", s("position")), ("x", num(*x)), ("y", num(*y)), ("step", num(*step))]
        }
        Field::Button { text, tip } => vec![("type", s("button")), ("text", s(text)), ("tip", s(tip))],
        Field::Color { swatches, current, tinted } => vec![
            ("type", s("color")),
            (
                "swatches",
                arr(swatches.iter().map(|w| {
                    obj(&[
                        ("value", s(&w.value)),
                        ("name", s(&w.name)),
                        ("rgb", rgb(w.rgb)),
                        ("tip", s(&w.tip)),
                    ])
                })),
            ),
            ("current", s(current)),
            ("tinted", b(*tinted)),
        ],
        Field::Type { value, guess } => {
            vec![("type", s("datatype")), ("value", s(value)), ("guess", s(guess))]
        }
        Field::Cardinality(c) => {
            let mut v = vec![("type", s("card"))];
            v.extend(card(c));
            v
        }
        Field::Attributes(rows) => vec![("type", s("attributes")), ("rows", arr(rows.iter().map(attr_row)))],
        Field::Lineage(rows) => vec![("type", s("lineage")), ("rows", arr(rows.iter().map(lineage_row)))],
        Field::Code(text) => vec![("type", s("code")), ("text", s(text))],
    }
}

pub fn form(f: &Form) -> String {
    let rows = f.rows.iter().map(|r| {
        let mut pairs = vec![("id", s(&r.id)), ("label", s(&r.label))];
        pairs.extend(field(&r.field));
        obj(&pairs)
    });
    obj(&[("title", s(&f.title)), ("focus", opt(f.focus.as_deref())), ("rows", arr(rows))])
}

pub fn outcome(o: &Outcome) -> String {
    match o {
        Outcome::Done => obj(&[("kind", s("done"))]),
        Outcome::Row { row, edit } => {
            obj(&[("kind", s("row")), ("row", row.to_string()), ("edit", edit.to_string())])
        }
        Outcome::Ask { title, text } => obj(&[("kind", s("ask")), ("title", s(title)), ("text", s(text))]),
        Outcome::Refused(text) => obj(&[("kind", s("refused")), ("text", s(text))]),
        Outcome::Inform { title, text } => {
            obj(&[("kind", s("inform")), ("title", s(title)), ("text", s(text))])
        }
        Outcome::Invalid(text) => obj(&[("kind", s("invalid")), ("text", s(text))]),
    }
}

pub fn problems(list: &[Listed], summary: &str, count: usize) -> String {
    obj(&[
        ("summary", s(summary)),
        ("count", count.to_string()),
        (
            "rows",
            arr(list.iter().map(|p| {
                obj(&[
                    ("severity", severity(p.severity)),
                    ("text", s(&p.text)),
                    ("fix", opt(p.fix.as_deref())),
                    ("rule", s(p.rule)),
                ])
            })),
        ),
    ])
}

pub fn relational(v: &rmwindow::View) -> String {
    let decisions = v.decisions.iter().map(|d| {
        let options = d.options.iter().map(|o| {
            obj(&[
                ("title", s(&o.title)),
                ("description", s(&o.description)),
                ("reason", s(&o.reason)),
                ("valid", o.valid.to_string()),
                ("result", s(&o.result)),
            ])
        });
        obj(&[
            ("element", s(&d.element_id)),
            ("title", s(&d.title)),
            ("question", s(&d.question)),
            ("current", d.current.to_string()),
            ("default", d.default.to_string()),
            ("minor", d.minor.to_string()),
            ("rule", s(d.rule)),
            ("options", arr(options)),
        ])
    });
    let types = v.types.iter().map(|r| {
        obj(&[
            ("owner", s(&r.owner)),
            ("name", s(&r.name)),
            ("type", s(&r.r#type)),
            ("guessed", r.guessed.to_string()),
        ])
    });
    let problems = v.problems.iter().map(|p| {
        obj(&[
            ("severity", severity(p.severity)),
            ("text", s(&p.text)),
            ("element", opt(p.element_id.as_deref())),
        ])
    });
    obj(&[
        ("er", v.er.to_string()),
        ("output", s(&v.output)),
        ("decisions", arr(decisions)),
        ("types", arr(types)),
        ("problems", arr(problems)),
        ("issues", v.issues.to_string()),
        ("summary", s(&v.summary())),
        ("typesShown", v.types_shown.to_string()),
    ])
}

pub fn conversion(c: &Conversion) -> String {
    let choices = c.choices.iter().map(|ch| {
        obj(&[
            ("key", s(&ch.key)),
            ("title", s(&ch.title)),
            ("question", s(&ch.question)),
            ("options", arr(ch.options.iter().map(|(v, t, x)| arr([s(v), s(t), s(x)])))),
            ("default", s(&ch.default)),
            ("current", s(&ch.current)),
            ("rule", s(ch.rule)),
        ])
    });
    obj(&[
        ("text", s(&c.doc.to_text())),
        ("choices", arr(choices)),
        ("report", arr(c.report.iter().map(|r| s(r)))),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_are_escaped() {
        assert_eq!(s("a\"b\\c\nd\u{1}"), r#""a\"b\\c\nd\u0001""#);
        assert_eq!(num(f64::NAN), "null");
        assert_eq!(num(1.0), "1");
        assert_eq!(num(-0.25), "-0.25");
        assert_eq!(rgb(Some((1.0, 0.5, 0.0))), r##""#ff8000""##);
    }
}
