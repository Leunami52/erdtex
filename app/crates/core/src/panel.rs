//! The properties panel, as data: [`form`] describes the fields for what is selected, and
//! [`set`] applies what one of them changed.  The page only draws the form and reports its
//! fields, so the panel works (and is tested) the same in the desktop app and a browser.
//!
//! A port of `properties.py`, `cardpicker.py` and `colorpicker.py`.  Each field has an id:
//!
//! * `opt:<key>`: an option of the element (empty, `false` or the key's default removes it)
//! * `setup:<key>`: an option of the diagram's `\setup`
//! * `attr.<what>.<row>`: the attribute table (`name`, `key`, `unique`, `type`, `kind`,
//!   `formula`, and the buttons `add`, `remove`, `up`, `down`, `indent`, `outdent`, `next`)
//! * `fix.<n>`: the n-th problem's fix
//! * the rest name what they do (`label`, `id`, `kind`, `card`, `routing`, ...)

use crate::cardinality::{
    self, Card, MIN_MAX, N_CHOICES, N_NOTATION, UNKNOWN_MIN, apply_changes, diagram_notation, label_notation,
    meaning, notation_name, parse_card, plan_diagram, plan_relationship, relationship_notation,
    relationship_of, relationship_sides,
};
use crate::colors::{Rgb, parse_color, tint};
use crate::document::{Document, Kind};
use crate::editor::{Editor, edge_tools, is_default_id, slug};
use crate::generalisation::{
    WORDS, explain_coverage, explain_overlap_after, is_letters, letters, parse_constraint,
};
use crate::latex_text::plain_text;
use crate::model::{
    AttrItem, Body, Coord, Element, ElementKey, Options, Setup, endpoint_id, flatten_items, nest_items,
    parse_coord_list,
};
use crate::relational::names::{TYPE_PRESETS, guess_type};
use crate::rules::{self, Severity, can_connect, dependent_link, explain_dependency, other_end, weak_links};
use crate::text::{fmt_g, parse_float, py_round, strip};

/// The eight directions of TikZ anchors.
pub const COMPASS: &[&str] =
    &["east", "north east", "north", "north west", "west", "south west", "south", "south east"];
const SIDES: &[&str] = &["auto", "above", "below", "left", "right"];

pub const FLIP_WARNING: &str = "The two notations count from opposite ends: a label in n-notation is written at the \
far end from the entity it counts, (min,max) at that entity itself. So converting moves each value to the other \
end of the relationship.";

/// The colours offered: (xcolor value, name); "" is the package's default.
pub const NODE_FILLS: &[(&str, &str)] = &[
    ("", "Default"),
    ("white", "White"),
    ("red", "Red"),
    ("green", "Green"),
    ("blue", "Blue"),
    ("yellow", "Yellow"),
    ("cyan", "Cyan"),
    ("magenta", "Magenta"),
    ("orange", "Orange"),
    ("gray", "Grey"),
    ("purple", "Purple"),
    ("brown", "Brown"),
];
pub const TEXT_COLORS: &[(&str, &str)] = &[
    ("", "Default"),
    ("black", "Black"),
    ("red!70!black", "Dark red"),
    ("red", "Red"),
    ("blue!70!black", "Dark blue"),
    ("blue", "Blue"),
    ("green!50!black", "Dark green"),
    ("magenta", "Magenta"),
    ("orange", "Orange"),
    ("gray", "Grey"),
];
pub const OUTLINE_COLORS: &[(&str, &str)] = &[
    ("", "Default"),
    ("blue!60", "Blue"),
    ("red!60", "Red"),
    ("green!50!black", "Green"),
    ("orange", "Orange"),
    ("magenta!60", "Magenta"),
    ("purple!60", "Purple"),
    ("gray", "Grey"),
    ("black", "Black"),
];
pub const AREA_FILLS: &[(&str, &str)] = &[
    ("", "None"),
    ("blue!5", "Light blue"),
    ("red!5", "Light red"),
    ("green!5", "Light green"),
    ("yellow!10", "Light yellow"),
    ("orange!10", "Light orange"),
    ("gray!10", "Light grey"),
];

/// The options the panel has fields for; the rest go to "More options".
fn covered(group: &str) -> &'static [&'static str] {
    match group {
        "node" => &[
            "weak",
            "dependency",
            "color",
            "colour",
            "key",
            "derived",
            "multivalued",
            "composite",
            "of",
            "formula",
            "unique",
            "constraint pos",
            "super card",
            "sub card",
            "label pos",
            "type",
            "rm",
            "rm name",
            "rm key",
            "rm category",
        ],
        "edge" => &[
            "card placement",
            "inline",
            "beside",
            "card side",
            "role",
            "role side",
            "bend",
            "bend left",
            "bend right",
            "via",
            "ortho",
            "arrow",
            "label side",
            "sloped",
            "horizontal",
            "label color",
            "double",
            "double distance",
        ],
        "tree" => &["anchor", "dir", "side", "align"],
        "note" => &["color", "anchor"],
        "subdomain" => &["color", "colour", "dotted", "fill"],
        _ => &[],
    }
}

/// The value of an option that means "not set": choosing it removes the option.
fn default_of(key: &str) -> &'static str {
    match key {
        "attribute style" => "compact",
        "isa shape" => "hexagon",
        "constraint style" => "letters",
        "card side" | "role side" | "label side" => "auto",
        "anchor" => "east", // of attribute lists; a note's is "center", see `set`
        _ => "",
    }
}

// -- the form ---------------------------------------------------------------------------------

/// One entry of a drop-down.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub value: String,
    pub text: String,
    pub tip: String,
}

/// One entry of a colour drop-down.
#[derive(Clone, Debug, PartialEq)]
pub struct Swatch {
    pub value: String,
    pub name: String,
    /// what it looks like; `None` for "none" or an unknown colour
    pub rgb: Option<Rgb>,
    pub tip: String,
}

/// One row of the attribute table.
#[derive(Clone, Debug, PartialEq)]
pub struct AttrRow {
    pub depth: usize,
    pub name: String,
    pub key: bool,
    pub unique: bool,
    pub r#type: String,
    /// what the schema guesses without a type
    pub guess: String,
    /// composite: its type is that of its parts
    pub composite: bool,
    /// `normal`, `multi-valued` or `derived`
    pub kind: &'static str,
    pub formula: String,
}

/// One row of an element's list of all its attributes (`lineage`): its own and the inherited ones.
#[derive(Clone, Debug, PartialEq)]
pub struct LineageRow {
    pub depth: usize,
    /// on one line, without LaTeX
    pub name: String,
    pub key: bool,
    /// the key of a weak entity type (dashed)
    pub weak_key: bool,
    pub unique: bool,
    /// `normal`, `multi-valued` or `derived`
    pub kind: &'static str,
    /// ER: the data type; GSM: the type the attribute line leads to
    pub r#type: String,
    /// inherited: the element it comes from (empty: its own)
    pub from: String,
    /// where it is defined and how it is inherited, one chain per line (the tooltip)
    pub trace: String,
    /// where to edit it: the element's key, and the row in an attribute list (`12` or `12:3`)
    pub target: String,
}

/// The cardinality of an edge: the choices of the notation in use, and what the value means.
#[derive(Clone, Debug, PartialEq)]
pub struct CardField {
    /// `n` or `minmax`
    pub notation: &'static str,
    /// n-notation: the four choices, each with its meaning as tooltip
    pub choices: Vec<Item>,
    pub current: Option<String>,
    /// min-max: the two fields
    pub lo: String,
    pub hi: String,
    /// an old or invalid label, as written
    pub note: Option<String>,
    /// what the value means, with the real names
    pub sentence: String,
    /// the button that changes the notation of the relationship: (text, tooltip)
    pub deviation: Option<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Field {
    /// text, with a button beside it if `button` is set (its id is the row's)
    Text {
        text: String,
        button: Option<String>,
        tip: String,
    },
    Problem {
        severity: Severity,
        message: String,
        fix: Option<String>,
    },
    Check {
        text: String,
        value: bool,
        enabled: bool,
        tip: String,
    },
    Choice {
        items: Vec<Item>,
        current: String,
        editable: bool,
        tip: String,
    },
    Line {
        value: String,
        placeholder: String,
        tip: String,
    },
    /// a label: several lines, which become `\\`
    Label {
        value: String,
        tip: String,
    },
    Number {
        value: f64,
        min: f64,
        max: f64,
        step: f64,
        decimals: u32,
        suffix: String,
    },
    /// x and y in cm (`<id>.x`, `<id>.y`)
    Position {
        x: f64,
        y: f64,
        step: f64,
    },
    Button {
        text: String,
        tip: String,
    },
    Color {
        swatches: Vec<Swatch>,
        current: String,
        tinted: bool,
    },
    /// a data type: free text with the usual types offered
    Type {
        value: String,
        guess: String,
    },
    Cardinality(CardField),
    Attributes(Vec<AttrRow>),
    /// every attribute of an entity or object type, inherited ones with where they come from
    Lineage(Vec<LineageRow>),
    /// read-only source text
    Code(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: String,
    pub label: String,
    pub field: Field,
}

/// The panel for the current selection.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Form {
    pub title: String,
    pub rows: Vec<Row>,
    /// the field that placing or a double click puts the cursor in
    pub focus: Option<String>,
}

impl Form {
    fn row(&mut self, id: &str, label: &str, field: Field) {
        self.rows.push(Row { id: id.into(), label: label.into(), field });
    }

    fn text(&mut self, label: &str, text: impl Into<String>) {
        self.row("", label, Field::Text { text: text.into(), button: None, tip: String::new() });
    }

    fn check(&mut self, id: &str, text: &str, value: bool) {
        self.row(id, "", Field::Check { text: text.into(), value, enabled: true, tip: String::new() });
    }

    fn line(&mut self, id: &str, label: &str, value: &str, placeholder: &str) {
        self.row(
            id,
            label,
            Field::Line { value: value.into(), placeholder: placeholder.into(), tip: String::new() },
        );
    }

    fn button(&mut self, id: &str, label: &str, text: &str, tip: &str) {
        self.row(id, label, Field::Button { text: text.into(), tip: tip.into() });
    }

    /// A drop-down as the Python app's `combo`: "" reads "(default)"; a current value that
    /// isn't offered is added.
    fn combo(&mut self, id: &str, label: &str, values: &[&str], current: &str) {
        let mut items: Vec<Item> = values
            .iter()
            .map(|v| Item {
                value: v.to_string(),
                text: if v.is_empty() { "(default)".into() } else { v.to_string() },
                tip: String::new(),
            })
            .collect();
        if !current.is_empty() && !values.contains(&current) {
            items.push(Item { value: current.into(), text: current.into(), tip: String::new() });
        }
        let current = if items.iter().any(|i| i.value == current) {
            current.to_string()
        } else {
            items.first().map(|i| i.value.clone()).unwrap_or_default()
        };
        self.row(id, label, Field::Choice { items, current, editable: false, tip: String::new() });
    }

    /// The tooltip of an entry of the last drop-down.
    fn item_tip(&mut self, value: &str, tip: String) {
        if let Some(Field::Choice { items, .. }) = self.rows.last_mut().map(|r| &mut r.field)
            && let Some(it) = items.iter_mut().find(|i| i.value == value)
        {
            it.tip = tip;
        }
    }

    /// The tooltip of the last row.
    fn tip(&mut self, text: &str) {
        if let Some(row) = self.rows.last_mut() {
            match &mut row.field {
                Field::Text { tip, .. }
                | Field::Check { tip, .. }
                | Field::Choice { tip, .. }
                | Field::Line { tip, .. }
                | Field::Label { tip, .. }
                | Field::Button { tip, .. } => *tip = text.into(),
                _ => {}
            }
        }
    }

    /// Renames the entries of the last drop-down: (value, text).
    fn texts(&mut self, texts: &[(&str, &str)]) {
        if let Some(Field::Choice { items, .. }) = self.rows.last_mut().map(|r| &mut r.field) {
            for (value, text) in texts {
                if let Some(it) = items.iter_mut().find(|i| i.value == *value) {
                    it.text = text.to_string();
                }
            }
        }
    }

    /// The field with this id.
    pub fn field(&self, id: &str) -> Option<&Field> {
        self.rows.iter().find(|r| r.id == id).map(|r| &r.field)
    }

    /// All the text the panel shows (labels, texts, problems), for tests.
    pub fn all_text(&self) -> String {
        let mut out = vec![self.title.clone()];
        for r in &self.rows {
            out.push(r.label.clone());
            match &r.field {
                Field::Text { text, .. } => out.push(text.clone()),
                Field::Problem { message, .. } => out.push(message.clone()),
                Field::Check { text, .. } => out.push(text.clone()),
                Field::Cardinality(c) => {
                    out.push(c.sentence.clone());
                    out.extend(c.note.clone());
                }
                _ => {}
            }
        }
        out.join(" ")
    }
}

fn tex_to_lines(s: &str) -> String {
    s.replace("\\\\", "\n")
}

fn lines_to_tex(s: &str) -> String {
    s.trim_matches('\n').split('\n').map(str::trim_end).collect::<Vec<_>>().join("\\\\")
}

fn swatches(entries: &[(&str, &str)], current: &str, tinted: bool, default: Option<&str>) -> Vec<Swatch> {
    let shown = |v: &str| {
        if v.is_empty() {
            None
        } else {
            parse_color(Some(&if tinted { tint(v) } else { v.to_string() }), None)
        }
    };
    let make = |value: &str, name: &str| {
        let rgb =
            if value.is_empty() { default.and_then(|d| parse_color(Some(d), None)) } else { shown(value) };
        let mut tip =
            if value.is_empty() { "the package default".to_string() } else { format!("LaTeX: {value}") };
        if !value.is_empty() && tinted && !value.contains('!') && value != "white" && value != "none" {
            tip.push_str(&format!("  (used as {})", tint(value)));
        }
        Swatch { value: value.into(), name: name.into(), rgb, tip }
    };
    let mut out: Vec<Swatch> = entries.iter().map(|(v, n)| make(v, n)).collect();
    if !current.is_empty() && !entries.iter().any(|(v, _)| *v == current) {
        out.push(make(current, &format!("Custom ({current})")));
    }
    out
}

/// What a colour expression looks like (for the "Other…" dialog): `None` if unknown.
pub fn color_rgb(expr: &str, tinted: bool) -> Option<Rgb> {
    let e = strip(expr);
    if e.is_empty() {
        return None;
    }
    parse_color(Some(&if tinted { tint(e) } else { e.to_string() }), None)
}

fn opts_of(body: &Body) -> Option<&Options> {
    match body {
        Body::Node(n) => Some(&n.opts),
        Body::Edge(e) => Some(&e.opts),
        Body::Attributes(t) => Some(&t.opts),
        Body::Note(n) => Some(&n.opts),
        Body::Subdomain(s) => Some(&s.opts),
        Body::Setup(s) => Some(&s.opts),
        _ => None,
    }
}

fn opts_mut(body: &mut Body) -> Option<&mut Options> {
    match body {
        Body::Node(n) => Some(&mut n.opts),
        Body::Edge(e) => Some(&mut e.opts),
        Body::Attributes(t) => Some(&mut t.opts),
        Body::Note(n) => Some(&mut n.opts),
        Body::Subdomain(s) => Some(&mut s.opts),
        Body::Setup(s) => Some(&mut s.opts),
        _ => None,
    }
}

fn setup_of(doc: &Document) -> Option<&Element> {
    doc.elements.iter().find(|e| matches!(e.body, Body::Setup(_)))
}

fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
        None => String::new(),
    }
}

fn label_of(doc: &Document, ident: &str) -> String {
    let label = doc.by_id(ident).and_then(Element::as_node).and_then(|n| n.label.as_deref()).unwrap_or("");
    let p = plain_text(label);
    if p.is_empty() { ident.to_string() } else { p }
}

/// The single selected element.
fn single(ed: &Editor) -> Option<ElementKey> {
    let keys: Vec<ElementKey> = ed.selection.iter().copied().filter(|k| ed.doc().get(*k).is_some()).collect();
    match keys.as_slice() {
        [k] => Some(*k),
        _ => None,
    }
}

/// The form for the current selection.
pub fn form(ed: &mut Editor) -> Form {
    let mut f = Form::default();
    let keys: Vec<ElementKey> = ed.selection.iter().copied().filter(|k| ed.doc().get(*k).is_some()).collect();
    match keys.as_slice() {
        [] => {
            f.title = "Diagram".into();
            diagram_form(ed, &mut f);
        }
        [key] => element_form(ed, *key, &mut f),
        many => {
            f.title = format!("{} elements selected", many.len());
            f.button("delete", "", "Delete selected", "");
        }
    }
    f
}

fn element_form(ed: &mut Editor, key: ElementKey, f: &mut Form) {
    let Some(el) = ed.doc().get(key).cloned() else { return };
    let mut tail = Form::default(); // below the problems (the attribute table)
    match &el.body {
        Body::Node(n) if n.generalisation.is_some() => {
            f.title =
                if n.command == "canbe" { "Specialisation (CAN_BE)" } else { "Generalisation (IS_A)" }.into();
            isa_form(ed, key, &el, f);
        }
        Body::Node(n) => {
            f.title = type_name(ed.doc().kind, &n.command, &n.opts);
            node_form(ed, key, &el, f);
            if crate::lineage::eligible(ed.doc(), &n.id) {
                lineage_rows(ed.doc(), &n.id, &mut tail);
            }
        }
        Body::Edge(e) => {
            f.title = type_name(ed.doc().kind, &e.command, &e.opts);
            edge_form(ed, key, &el, f);
        }
        Body::Attributes(_) => {
            f.title = "Attribute list".into();
            tree_form(ed, &el, f, &mut tail);
        }
        Body::Note(n) => {
            f.title = "Note".into();
            label_row(f, &n.content, "Text");
            pos_rows(ed, &el, f);
            f.row(
                "opt:color",
                "Text colour",
                Field::Color {
                    swatches: swatches(
                        TEXT_COLORS,
                        n.opts.get("color").unwrap_or(""),
                        false,
                        Some("red!70!black"),
                    ),
                    current: n.opts.get("color").unwrap_or("").into(),
                    tinted: false,
                },
            );
            let mut values = vec!["center"];
            values.extend(COMPASS);
            f.combo("note-anchor", "Anchor", &values, n.opts.get("anchor").unwrap_or("center"));
            extra_row(f, &el, "note");
        }
        Body::Subdomain(s) => {
            f.title = "Subdomain".into();
            subdomain_form(ed, key, s, &el, f);
        }
        Body::Point(_) => {
            f.title = "Junction point".into();
            id_row(f, el.id().unwrap_or(""));
            pos_rows(ed, &el, f);
        }
        Body::Raw(r) => {
            f.title = "Plain TeX (not editable here)".into();
            f.row("", "", Field::Code(r.raw.clone()));
        }
        Body::Setup(_) => f.title = "Setup".into(),
        Body::Grid(_) => f.title = "Grid".into(),
    }
    let problems: Vec<rules::Problem> =
        rules::check(ed.doc()).into_iter().filter(|p| p.element == key).collect();
    for (n, p) in problems.iter().enumerate() {
        f.row(
            &format!("fix.{n}"),
            "",
            Field::Problem {
                severity: p.severity,
                message: p.message.clone(),
                fix: p.fix.as_ref().map(|x| x.title.clone()),
            },
        );
    }
    f.rows.append(&mut tail.rows);
}

/// What an element is, in full: "Entity type", "Representable type", "Attribute line" (the palette's
/// names).
fn type_name(kind: Kind, command: &str, opts: &Options) -> String {
    let weak = opts.is_on("weak") || opts.get("dependency").is_some_and(|d| !d.is_empty());
    match (kind, command) {
        (Kind::Er, "entity") if weak => "Weak entity type",
        (Kind::Er, "entity") => "Entity type",
        (Kind::Er, "relationship") if weak => "Weak relationship type",
        (Kind::Er, "relationship") => "Relationship type",
        (Kind::Er, "attribute") => "Attribute (Chen)",
        (Kind::Er, "partof") => "Aggregation (PART_OF)",
        (Kind::Er, "link") => "Line",
        (Kind::Gsm, "abstract") => "Abstract type",
        (Kind::Gsm, "representable") => "Representable type",
        (Kind::Gsm, "subtype") => "Subtype",
        (Kind::Gsm, "aggregation") => "Aggregation",
        (Kind::Gsm, "grouping") => "Grouping",
        (Kind::Gsm, "attribute") => "Single-valued attribute",
        (Kind::Gsm, "multiattribute") => "Multi-valued attribute",
        (Kind::Gsm, "component") => "Component of a constructed type",
        (Kind::Gsm, "generalization") => "Generalization (disjoint IS-A)",
        (Kind::Gsm, "specialization") => "Specialization (overlapping IS-A)",
        (Kind::Gsm, "link") => "Plain line",
        _ => return cap_first(command),
    }
    .into()
}

/// All attributes of an entity or object type, its own and the inherited ones (at the bottom).
fn lineage_rows(doc: &Document, id: &str, tail: &mut Form) {
    let attrs = crate::lineage::all_attributes(doc, id);
    let one_line =
        |s: &str| plain_text(&s.replace("\\\\", " ")).split_whitespace().collect::<Vec<_>>().join(" ");
    let rows: Vec<LineageRow> = attrs
        .iter()
        .map(|a| {
            let trace = if a.inherited() {
                let mut lines = vec![format!("Inherited from {}:", a.owner_label)];
                lines.extend(a.chains.iter().map(|c| crate::lineage::chain_text(doc, id, c)));
                lines.join("\n")
            } else {
                "Its own attribute.".to_string()
            };
            LineageRow {
                depth: a.depth,
                name: one_line(&a.name),
                key: a.key,
                weak_key: a.weak_key,
                unique: a.unique,
                kind: if a.derived {
                    "derived"
                } else if a.multivalued {
                    "multi-valued"
                } else {
                    "normal"
                },
                r#type: one_line(&a.r#type),
                from: if a.inherited() { a.owner_label.clone() } else { String::new() },
                trace,
                target: match a.source.row {
                    Some(r) => format!("{}:{r}", a.source.element.0),
                    None => a.source.element.0.to_string(),
                },
            }
        })
        .collect();
    let what = if doc.kind == Kind::Er { "attributes" } else { "attribute lines" };
    tail.row("lineage", &format!("All {what}\n(own and inherited):"), Field::Lineage(rows));
}

// -- the diagram (\setup) ---------------------------------------------------------------------

fn diagram_form(ed: &Editor, f: &mut Form) {
    let doc = ed.doc();
    let setup = setup_of(doc).and_then(|e| opts_of(&e.body)).cloned();
    let get = |k: &str| setup.as_ref().and_then(|o| o.get(k)).unwrap_or("").to_string();
    let has = |k: &str| setup.as_ref().is_some_and(|o| o.contains(k));
    f.text("Type", if doc.kind == Kind::Er { "ER diagram" } else { "GSM diagram" });
    let sans = has("sans serif") && matches!(get("sans serif").as_str(), "" | "true");
    f.check("sans serif", "Sans-serif", sans);
    if doc.kind == Kind::Er {
        let notation = diagram_notation(doc);
        f.combo("notation", "Cardinality notation", &[N_NOTATION, MIN_MAX], notation);
        f.texts(&[(N_NOTATION, "n-notation: 1, c, n, cn"), (MIN_MAX, "min-max: (0,*), (1,1), …")]);
        f.tip(FLIP_WARNING);
        let or = |k: &str, d: &str| {
            let v = get(k);
            if v.is_empty() { d.to_string() } else { v }
        };
        f.combo(
            "setup:attribute style",
            "Attributes",
            &["compact", "chen"],
            &or("attribute style", "compact"),
        );
        f.texts(&[("compact", "compact lists"), ("chen", "ovals (Chen)")]);
        f.check("types", "Show data types", doc.types_shown());
        f.row(
            "crowsfoot",
            "",
            Field::Check {
                text: "Crow's foot symbols".into(),
                value: get("card symbols") == "crowsfoot",
                enabled: notation == N_NOTATION,
                tip:
                    "Draw 1, c, n and cn as symbols at the end of the edge. Only for n-notation, which they \
                      correspond to exactly."
                        .into(),
            },
        );
        f.combo(
            "setup:card placement",
            "Cardinality position",
            &["", "inline", "beside"],
            &get("card placement"),
        );
        f.combo("setup:isa shape", "IS_A shape", &["hexagon", "diamond"], &or("isa shape", "hexagon"));
        f.combo(
            "setup:constraint style",
            "IS_A type shown as",
            &["letters", "words"],
            &or("constraint style", "letters"),
        );
        f.texts(&[("letters", "letters: cd"), ("words", "words: complete, disjoint")]);
        let mut values = vec!["", "inside", "auto"];
        values.extend(COMPASS);
        f.combo("setup:constraint pos", "IS_A type position", &values, &get("constraint pos"));
    } else {
        f.combo("setup:label sloped", "Edge labels", &["", "true", "false"], &get("label sloped"));
        let mut values = vec!["", "auto"];
        values.extend(COMPASS);
        f.combo("setup:aggr label pos", "Aggregation labels", &values, &get("aggr label pos"));
    }
    f.text("", "These settings are stored in a \\setup{...} line of the file. Select an element to edit it.");
}

// -- nodes ------------------------------------------------------------------------------------

fn id_row(f: &mut Form, id: &str) {
    f.line("id", "Id", id, "");
    f.tip("Internal name used by edges; not printed");
}

fn label_row(f: &mut Form, value: &str, title: &str) {
    f.row(
        "label",
        title,
        Field::Label {
            value: tex_to_lines(value),
            tip: "LaTeX allowed. A new line (Shift+Enter) becomes \\\\.".into(),
        },
    );
    f.focus = Some("label".into());
}

fn pos_rows(ed: &Editor, el: &Element, f: &mut Form) {
    let (pos, opts) = match &el.body {
        Body::Node(n) => (n.pos.as_ref(), Some(&n.opts)),
        Body::Note(n) => (n.pos.as_ref(), Some(&n.opts)),
        Body::Point(p) => (p.pos.as_ref(), None),
        _ => return,
    };
    let positioned = opts.is_some_and(|o| {
        ["right", "left", "above", "below", "above right", "above left", "below right", "below left"]
            .iter()
            .any(|k| o.contains(k))
    });
    match pos {
        Some(Coord::Numeric { x, y }) if !positioned => {
            let step = if ed.snap_cm > 0.0 { ed.snap_cm } else { 0.25 };
            f.row("pos", "Position (cm)", Field::Position { x: *x, y: *y, step });
        }
        _ => f.button(
            "absolute",
            "Position",
            "Make position absolute",
            "Replace the relative/computed position by (x,y)",
        ),
    }
}

fn color_row(ed: &mut Editor, el: &Element, f: &mut Form) {
    let Body::Node(n) = &el.body else { return };
    let prefix = match n.command.as_str() {
        "entity" => Some("entity"),
        "relationship" | "partof" | "isa" | "canbe" => Some("rel"),
        "attribute" => Some("attribute"),
        "abstract" => Some("abstract"),
        "representable" => Some("repr"),
        "subtype" => Some("sub"),
        _ => None,
    };
    let default = prefix
        .map(|p| ed.session_mut().with_layout(|_, lay| lay.style.get(&format!("{p} fill")).to_string()));
    let current = n.opts.get("color").or(n.opts.get("colour")).unwrap_or("").to_string();
    f.row(
        "opt:color",
        "Colour",
        Field::Color {
            swatches: swatches(NODE_FILLS, &current, true, default.as_deref().filter(|d| !d.is_empty())),
            current,
            tinted: true,
        },
    );
}

fn extra_row(f: &mut Form, el: &Element, group: &str) {
    let Some(opts) = opts_of(&el.body) else { return };
    let cov = covered(group);
    let extra =
        Options { items: opts.items.iter().filter(|(k, _)| !cov.contains(&k.as_str())).cloned().collect() };
    let tex = extra.to_tex();
    let inner = tex.strip_prefix('[').and_then(|t| t.strip_suffix(']')).unwrap_or("");
    f.line("more", "More options", inner, "TikZ options, e.g. minimum width=3cm");
    f.tip("Further options, passed on to TikZ (key=value, comma separated)");
}

fn node_form(ed: &mut Editor, _key: ElementKey, el: &Element, f: &mut Form) {
    let Body::Node(n) = &el.body else { return };
    let doc = ed.doc().clone();
    let er = doc.kind == Kind::Er;
    if er && n.command == "entity" {
        f.check("weak", "Weak entity type", n.opts.is_on("weak"));
    }
    if er && n.command == "relationship" {
        let cur = match n.opts.get("dependency") {
            Some("E") => "E",
            Some("Id") => "Id",
            _ if n.opts.is_on("weak") => "weak",
            _ => "",
        };
        f.combo("kind", "Kind", &["", "weak", "E", "Id"], cur);
        for v in ["weak", "E", "Id"] {
            f.item_tip(v, explain_dependency(&doc, &n.id, v));
        }
        f.texts(&[
            ("", "normal"),
            ("weak", "weak (double border)"),
            ("E", "existence dependency (E)"),
            ("Id", "identification dependency (Id)"),
        ]);
        if !cur.is_empty() {
            f.text("", explain_dependency(&doc, &n.id, cur));
            dependent_row(&doc, &n.id, f);
        }
    }
    id_row(f, &n.id);
    if let Some(label) = &n.label {
        label_row(f, label, "Label");
    }
    pos_rows(ed, el, f);
    if !(er && matches!(n.command.as_str(), "aggregation" | "grouping")) {
        color_row(ed, el, f);
    }
    if er && n.command == "attribute" {
        for (key, text) in [
            ("key", "Key attribute (underlined)"),
            ("unique", "Further candidate key (not drawn)"),
            ("derived", "Derived (dashed)"),
            ("multivalued", "Multi-valued (double)"),
            ("composite", "Composite (rounded box)"),
        ] {
            f.check(&format!("opt:{key}"), text, n.opts.contains(key));
        }
        let mut owners = vec![String::new()];
        owners.extend(
            doc.nodes()
                .filter(|o| {
                    matches!(o.command.as_str(), "entity" | "relationship" | "attribute") && o.id != n.id
                })
                .map(|o| o.id.clone()),
        );
        let owners: Vec<&str> = owners.iter().map(String::as_str).collect();
        f.combo("opt:of", "Belongs to", &owners, n.opts.get("of").unwrap_or(""));
        let shown = plain_text(n.label.as_deref().unwrap_or(""));
        f.row(
            "opt:type",
            "Data type",
            Field::Type {
                value: n.opts.get("type").unwrap_or("").into(),
                guess: guess_type(if shown.is_empty() { &n.id } else { &shown }).into(),
            },
        );
        if n.opts.contains("derived") {
            f.line(
                "opt:formula",
                "Formula",
                n.opts.get("formula").unwrap_or(""),
                "SQL expression (optional)",
            );
        }
    }
    if matches!(n.command.as_str(), "aggregation" | "grouping") {
        let mut values = vec!["", "auto"];
        values.extend(COMPASS);
        f.combo("opt:label pos", "Label position", &values, n.opts.get("label pos").unwrap_or(""));
    }
    if er && matches!(n.command.as_str(), "entity" | "relationship" | "partof") {
        f.line(
            "opt:rm name",
            "Relation name",
            n.opts.get("rm name").unwrap_or(""),
            "automatic (File → Relational Schema / SQL)",
        );
    }
    if matches!(n.command.as_str(), "entity" | "relationship" | "partof") {
        f.button("add-tree", "", "Add attribute list", "");
    }
    extra_row(f, el, "node");
}

/// A weak relationship between two weak entities: which one depends on the other?
fn dependent_row(doc: &Document, rel: &str, f: &mut Form) {
    let mut ends: Vec<String> = Vec::new();
    for (_, link) in weak_links(doc, rel) {
        let end = other_end(link, rel).to_string();
        if !ends.contains(&end) {
            ends.push(end);
        }
    }
    if ends.len() < 2 {
        return;
    }
    let chosen = dependent_link(doc, rel).map(|(_, l)| other_end(l, rel).to_string()).unwrap_or_default();
    let mut values = vec![""];
    values.extend(ends.iter().map(String::as_str));
    f.combo("dependent", "Dependent", &values, &chosen);
    let mut texts = vec![(String::new(), "(not specified)".to_string())];
    texts.extend(ends.iter().map(|e| (e.clone(), label_of(doc, e))));
    let texts: Vec<(&str, &str)> = texts.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    f.texts(&texts);
    f.tip("The double line leads to this entity. It takes part in the schema as the one that depends on the other.");
}

fn isa_form(ed: &mut Editor, _key: ElementKey, el: &Element, f: &mut Form) {
    let Body::Node(n) = &el.body else { return };
    let Some(g) = &n.generalisation else { return };
    let doc = ed.doc().clone();
    f.combo("kind", "Kind", &["isa", "canbe"], &n.command);
    f.texts(&[("isa", "IS_A (generalisation)"), ("canbe", "CAN_BE (specialisation)")]);
    id_row(f, &n.id);
    let ents: Vec<&str> = doc.nodes().filter(|o| o.command == "entity").map(|o| o.id.as_str()).collect();
    f.combo("supertype", "Supertype", &ents, &g.supertype);
    // the type: coverage, then overlap (in the order of the letters, `cd`), explained with the real
    // names; the overlap's explanation goes on from the coverage's
    let (complete, disjoint) = parse_constraint(g.constraint.as_deref());
    let sup = endpoint_id(&g.supertype);
    let sup_name = label_of(&doc, sup);
    let subs: Vec<String> =
        relationship_sides(&doc, &n.id).iter().map(|s| label_of(&doc, &s.entity)).collect();
    let word = |c: char| WORDS.iter().find(|(k, _)| *k == c).map_or("", |(_, w)| *w);
    type Explain<'a> = Box<dyn Fn(Option<bool>, &str, &[String]) -> String + 'a>;
    type Selector<'a> = (&'static str, &'static str, Option<bool>, [(char, bool); 2], Explain<'a>);
    let selectors: [Selector; 2] = [
        ("coverage", "Coverage", complete, [('c', true), ('i', false)], Box::new(explain_coverage)),
        (
            "overlap",
            "Overlap",
            disjoint,
            [('d', true), ('o', false)],
            Box::new(move |v, sup: &str, subs: &[String]| explain_overlap_after(v, complete, sup, subs)),
        ),
    ];
    for (id, title, value, choices, explain) in selectors {
        let current =
            choices.iter().find(|(_, v)| Some(*v) == value).map(|(k, _)| k.to_string()).unwrap_or_default();
        let [(a, _), (b, _)] = choices;
        let (a, b) = (a.to_string(), b.to_string());
        f.combo(id, title, &["", a.as_str(), b.as_str()], &current);
        f.texts(&[("", "(not specified)")]);
        for (k, v) in choices {
            let value = k.to_string();
            f.texts(&[(value.as_str(), format!("{} ({k})", word(k)).as_str())]);
            f.item_tip(&value, explain(Some(v), &sup_name, &subs));
        }
        f.text("", explain(value, &sup_name, &subs));
    }
    if let Some(c) = g.constraint.as_deref().filter(|c| !c.is_empty() && !is_letters(Some(c))) {
        f.text("", format!("Written as “{c}”. Choosing above replaces it with letters."));
    }
    let mut values = vec!["", "inside", "auto"];
    values.extend(COMPASS);
    f.combo("opt:constraint pos", "Type position", &values, n.opts.get("constraint pos").unwrap_or(""));
    f.line("opt:super card", "Supertype cardinality", n.opts.get("super card").unwrap_or(""), "");
    f.line("opt:sub card", "Subtype cardinality", n.opts.get("sub card").unwrap_or(""), "");
    f.line(
        "opt:rm name",
        "Category column",
        n.opts.get("rm name").unwrap_or(""),
        "category (relational schema)",
    );
    pos_rows(ed, el, f);
    color_row(ed, el, f);
    extra_row(f, el, "node");
}

// -- edges ------------------------------------------------------------------------------------

fn edge_form(ed: &mut Editor, key: ElementKey, el: &Element, f: &mut Form) {
    let Body::Edge(e) = &el.body else { return };
    let doc = ed.doc().clone();
    let er = doc.kind == Kind::Er;
    if !er {
        let kinds: Vec<&str> = edge_tools(Kind::Gsm).iter().map(|(_, c)| *c).collect();
        f.combo("kind", "Kind", &kinds, &e.command);
    }
    f.row(
        "reverse",
        "Connects",
        Field::Text {
            text: format!("{}  →  {}", e.src, e.dst),
            button: Some("Reverse".into()),
            tip: String::new(),
        },
    );
    if er {
        f.row("card", "Cardinality", Field::Cardinality(card_field(&doc, key, e.label.as_deref())));
        f.focus = Some("card".into());
        let cur = if e.opts.contains("beside") {
            "beside"
        } else if e.opts.contains("inline") {
            "inline"
        } else {
            ""
        };
        f.combo("placement", "Placement", &["", "inline", "beside"], cur);
        f.combo("opt:card side", "Side", SIDES, e.opts.get("card side").unwrap_or("auto"));
        f.line("opt:role", "Role", e.opts.get("role").unwrap_or(""), "");
        f.combo("opt:role side", "Role side", SIDES, e.opts.get("role side").unwrap_or("auto"));
        f.check("opt:arrow", "Arrow head", e.opts.contains("arrow"));
    } else {
        f.line("label", "Label", e.label.as_deref().unwrap_or(""), "");
        f.focus = Some("label".into());
        let cur = if e.opts.contains("horizontal") {
            "horizontal"
        } else if e.opts.contains("sloped") {
            "sloped"
        } else {
            ""
        };
        f.combo("orientation", "Orientation", &["", "sloped", "horizontal"], cur);
        f.texts(&[("", "(diagram default)")]);
        f.combo("opt:label side", "Label side", SIDES, e.opts.get("label side").unwrap_or("auto"));
        let current = e.opts.get("label color").unwrap_or("").to_string();
        f.row(
            "opt:label color",
            "Label colour",
            Field::Color {
                swatches: swatches(TEXT_COLORS, &current, false, Some("black")),
                current,
                tinted: false,
            },
        );
    }
    // routing
    let (route, bend) = route_of(&e.opts);
    f.combo("routing", "Routing", &["straight", "curved", "-|", "|-", "via"], route);
    f.texts(&[("-|", "right angle  ─┐"), ("|-", "right angle  │─"), ("via", "way points")]);
    if route == "curved" {
        let value = bend.as_deref().and_then(parse_float).map_or(30.0, f64::trunc);
        f.row(
            "bend",
            "Bend",
            Field::Number { value, min: -85.0, max: 85.0, step: 1.0, decimals: 0, suffix: " °".into() },
        );
    }
    if route == "via" {
        let n = parse_coord_list(e.opts.get("via").unwrap_or("")).len();
        f.text(
            "Way points",
            format!("{n} — drag the squares; double-click the edge to add one, a square to remove it"),
        );
    }
    extra_row(f, el, "edge");
}

/// The routing of an edge, and its bend in degrees if curved.
fn route_of(opts: &Options) -> (&'static str, Option<String>) {
    let mut bend = opts.get("bend").map(str::to_string);
    if opts.contains("bend left") {
        bend = Some(opts.get("bend left").filter(|v| !v.is_empty()).unwrap_or("30").to_string());
    }
    if opts.contains("bend right") {
        bend = Some(format!("-{}", opts.get("bend right").filter(|v| !v.is_empty()).unwrap_or("30")));
    }
    let route = if opts.get("via").is_some_and(|v| !v.is_empty()) {
        "via"
    } else if opts.get("ortho") == Some("-|") {
        "-|"
    } else if opts.get("ortho") == Some("|-") {
        "|-"
    } else if bend.as_deref().is_some_and(|b| !b.is_empty()) {
        "curved"
    } else {
        "straight"
    };
    (route, bend)
}

const SHORT_NAMES: [(&str, &str); 2] = [(N_NOTATION, "n-notation"), (MIN_MAX, "min-max")];

fn short_name(notation: &str) -> &'static str {
    SHORT_NAMES.iter().find(|(k, _)| *k == notation).map_or("", |(_, v)| *v)
}

fn card_field(doc: &Document, key: ElementKey, label: Option<&str>) -> CardField {
    let rid = relationship_of(doc, key);
    let side = rid.as_ref().and_then(|r| relationship_sides(doc, r).into_iter().find(|s| s.edge == key));
    let card = parse_card(label);
    let mut notation = match &rid {
        Some(r) => relationship_notation(doc, r),
        None => diagram_notation(doc),
    };
    if notation == "mixed" {
        notation = if card.here { MIN_MAX } else { N_NOTATION };
    }
    let mut out = CardField {
        notation: if notation == MIN_MAX { MIN_MAX } else { N_NOTATION },
        choices: vec![],
        current: None,
        lo: String::new(),
        hi: String::new(),
        note: None,
        sentence: String::new(),
        deviation: None,
    };
    if out.notation == MIN_MAX {
        let here = card.here && card.known;
        if here {
            out.lo = if card.unknown_min { UNKNOWN_MIN.to_string() } else { card.lo.to_string() };
            out.hi = card.hi.as_ref().map_or("*".to_string(), ToString::to_string);
        }
    } else {
        out.current = label.filter(|_| label_notation(&card) == N_NOTATION).map(str::to_string);
        out.choices = N_CHOICES
            .iter()
            .map(|c| Item {
                value: c.to_string(),
                text: c.to_string(),
                tip: match (&rid, &side) {
                    (Some(r), Some(s)) => meaning(doc, r, s, &parse_card(Some(c))),
                    _ => String::new(),
                },
            })
            .collect();
    }
    let form = label_notation(&card);
    if form == "old" || form == "invalid" {
        let l = label.unwrap_or("");
        out.note = Some(if form == "invalid" {
            format!("Written as “{l}”, which is not valid.")
        } else {
            format!("Written as “{l}”, an older form.")
        });
    }
    out.sentence = sentence(doc, rid.as_deref(), side.as_ref(), &card);
    if let Some(r) = &rid {
        let wanted = diagram_notation(doc);
        let current = relationship_notation(doc, r);
        out.deviation = Some(if current == wanted {
            let other = if wanted == N_NOTATION { MIN_MAX } else { N_NOTATION };
            (
                format!("Use {} here (discouraged)…", short_name(other)),
                format!(
                    "Use {} for all edges of this relationship, although the diagram uses {}. Not recommended: \
                     a diagram is easier to read with one notation.",
                    notation_name(other),
                    notation_name(wanted)
                ),
            )
        } else {
            (
                format!("Back to {}…", short_name(wanted)),
                format!("Use the diagram's {} for this relationship again.", notation_name(wanted)),
            )
        });
    }
    out
}

fn sentence(doc: &Document, rid: Option<&str>, side: Option<&cardinality::Side>, card: &Card) -> String {
    match (rid, side) {
        (Some(r), Some(s)) if card.known => meaning(doc, r, s, card),
        (Some(_), Some(_)) if card.text.is_empty() => "No cardinality set.".into(),
        _ => String::new(),
    }
}

// -- attribute lists, notes, subdomains -------------------------------------------------------

fn attribute_style(ed: &mut Editor) -> String {
    if ed.doc().kind != Kind::Er {
        return "compact".into();
    }
    ed.session_mut().with_layout(|_, lay| lay.style.get("attribute style").to_string())
}

fn tree_form(ed: &mut Editor, el: &Element, f: &mut Form, tail: &mut Form) {
    let Body::Attributes(t) = &el.body else { return };
    f.text("Belongs to", t.node.clone());
    let chen = attribute_style(ed) == "chen";
    if !chen {
        f.combo("opt:anchor", "Position", COMPASS, t.opts.get("anchor").unwrap_or("east"));
        if let Some(Field::Choice { editable, .. }) = f.rows.last_mut().map(|r| &mut r.field) {
            *editable = true;
        }
        f.combo(
            "opt:dir",
            "Direction",
            &["", "up", "down", "left", "right"],
            t.opts.get("dir").unwrap_or(""),
        );
        f.combo("opt:side", "Labels", &["", "left", "right"], t.opts.get("side").unwrap_or(""));
        f.combo("opt:align", "Bar alignment", &["", "top", "bottom"], t.opts.get("align").unwrap_or(""));
    }
    extra_row(f, el, "tree");
    let flat = flatten_items(&t.items);
    let rows = flat
        .iter()
        .map(|(it, depth)| {
            let shown = plain_text(&it.text);
            AttrRow {
                depth: *depth,
                name: it.text.clone(),
                key: it.key,
                unique: it.unique,
                r#type: it.r#type.clone().unwrap_or_default(),
                guess: guess_type(if shown.is_empty() { &it.text } else { &shown }).into(),
                composite: !it.parts.is_empty(),
                kind: if it.derived {
                    "derived"
                } else if it.multivalued {
                    "multi-valued"
                } else {
                    "normal"
                },
                formula: it.formula.clone().unwrap_or_default(),
            }
        })
        .collect();
    tail.row("attributes", "Attributes\n(top to bottom):", Field::Attributes(rows));
    f.focus = Some("attributes".into());
    if chen && flat.iter().any(|(it, _)| it.pos.is_some()) {
        tail.button("ovals-auto", "", "Place the ovals automatically", "");
    }
}

fn subdomain_form(ed: &mut Editor, key: ElementKey, s: &crate::model::Subdomain, el: &Element, f: &mut Form) {
    label_row(f, &s.label, "Label");
    let current = s.opts.get("color").or(s.opts.get("colour")).unwrap_or("").to_string();
    f.row(
        "opt:color",
        "Colour",
        Field::Color {
            swatches: swatches(OUTLINE_COLORS, &current, false, Some("blue!60")),
            current,
            tinted: false,
        },
    );
    f.check("opt:dotted", "Dotted background", s.opts.contains("dotted"));
    let fill = s.opts.get("fill").unwrap_or("").to_string();
    f.row(
        "opt:fill",
        "Fill",
        Field::Color { swatches: swatches(AREA_FILLS, &fill, false, None), current: fill, tinted: false },
    );
    let contents = ed.session_mut().with_layout(|doc, lay| {
        let boxes = crate::subdomains::boxes_of(doc, lay);
        let Some(b) = boxes.iter().find(|b| b.element == key) else { return String::new() };
        crate::subdomains::members(b, doc, lay, &boxes)
            .into_iter()
            .filter_map(|k| doc.get(k)?.as_node().cloned())
            .filter_map(|n| {
                let label = n.label.as_deref().filter(|l| !l.is_empty())?;
                let p = plain_text(label);
                Some(if p.is_empty() { n.id.clone() } else { p })
            })
            .collect::<Vec<_>>()
            .join(", ")
    });
    f.text("Contains", if contents.is_empty() { "nothing yet".to_string() } else { contents });
    f.tip("Whatever has its centre inside belongs to the box and moves with it.");
    let n = if s.points.len() == 2 { 4 } else { s.points.len() };
    f.text(
        "Corners",
        format!(
            "{n} — drag a square to move a corner, a circle to add one; double-click a corner to remove it"
        ),
    );
    if s.points.len() > 2 {
        f.button("make-rectangle", "", "Make rectangle", "Replace the outline by its bounding rectangle");
    }
    extra_row(f, el, "subdomain");
}

// -- changes ----------------------------------------------------------------------------------

/// What a change did.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// done (or nothing to do)
    Done,
    /// done; the attribute table's row to show, and whether to start typing its name
    Row { row: usize, edit: bool },
    /// the user must agree first: ask, then send the same change again with `confirmed`
    Ask { title: String, text: String },
    /// not done: for the status bar
    Refused(String),
    /// not done: for a message box
    Inform { title: String, text: String },
    /// the value can't be used: shown at the field
    Invalid(String),
}

/// Changes the selected element (or the diagram) as the field `id` says.
pub fn set(ed: &mut Editor, id: &str, value: &str, confirmed: bool) -> Outcome {
    if let Some(rest) = id.strip_prefix("setup:") {
        let v = if value == default_of(rest) { "" } else { value };
        set_setup(ed, rest, v);
        return Outcome::Done;
    }
    match id {
        "sans serif" => {
            set_setup(ed, "sans serif", if value == "true" { "true" } else { "" });
            return Outcome::Done;
        }
        "crowsfoot" => {
            set_setup(ed, "card symbols", if value == "true" { "crowsfoot" } else { "" });
            return Outcome::Done;
        }
        "types" => {
            let on = value == "true";
            ed.apply(if on { "Show data types" } else { "Hide data types" }, |ed| {
                ed.session_mut().doc_mut().set_types_shown(on);
            });
            return Outcome::Done;
        }
        "notation" => return set_notation(ed, value, confirmed),
        "delete" => {
            ed.delete_selection();
            return Outcome::Done;
        }
        _ => {}
    }
    let Some(key) = single(ed) else { return Outcome::Done };
    let Some(el) = ed.doc().get(key).cloned() else { return Outcome::Done };
    if let Some(n) = id.strip_prefix("fix.").and_then(|n| n.parse::<usize>().ok()) {
        let fix = rules::check(ed.doc()).into_iter().filter(|p| p.element == key).nth(n).and_then(|p| p.fix);
        if let Some(fix) = fix {
            ed.apply(&fix.title.clone(), |ed| fix.apply(ed.session_mut().doc_mut(), key));
        }
        return Outcome::Done;
    }
    if let Some(rest) = id.strip_prefix("attr.") {
        return set_attribute(ed, key, rest, value);
    }
    if let Some(k) = id.strip_prefix("opt:") {
        let label = match k {
            "color" | "fill" => "Change colour",
            "type" => "Set data type",
            "rm name" if matches!(&el.body, Body::Node(n) if n.generalisation.is_some()) => {
                "Set category column"
            }
            "rm name" => "Set relation name",
            _ => "Change option",
        };
        let v = strip(value);
        let v = if v == default_of(k) { "" } else { v };
        set_opt(ed, key, k, v, label);
        return Outcome::Done;
    }
    match id {
        "note-anchor" => {
            set_opt(ed, key, "anchor", if value == "center" { "" } else { value }, "Change option")
        }
        "id" => {
            let v = strip(value);
            if v.is_empty() || Some(v) == el.id() {
                return Outcome::Done;
            }
            if ed.doc().ids().contains(v) {
                return Outcome::Refused(format!("The id '{v}' is already used."));
            }
            let v = v.to_string();
            ed.apply("Rename", |ed| {
                let _ = ed.session_mut().doc_mut().rename(key, &v);
            });
        }
        "label" => set_label(ed, key, value),
        "pos.x" | "pos.y" => {
            let Some(v) = parse_float(value) else { return Outcome::Invalid("Not a number".into()) };
            let v = py_round(v, 3);
            change(ed, key, "Move", |body| {
                let pos = match body {
                    Body::Node(n) => &mut n.pos,
                    Body::Note(n) => &mut n.pos,
                    Body::Point(p) => &mut p.pos,
                    _ => return,
                };
                if let Some(Coord::Numeric { x, y }) = pos {
                    *pos = Some(if id == "pos.x" { Coord::numeric(v, *y) } else { Coord::numeric(*x, v) });
                }
            });
        }
        "absolute" => {
            ed.apply("Make absolute", |ed| ed.make_absolute(key));
        }
        "weak" => {
            let on = value == "true";
            let ident = el.id().unwrap_or("").to_string();
            ed.apply("Weak entity", |ed| {
                if let Some(o) = ed.session_mut().doc_mut().get_mut(key).and_then(|e| opts_mut(&mut e.body)) {
                    if on { o.set("weak", None) } else { o.remove("weak") }
                }
                touch(ed, key);
                if !on {
                    ed.drop_invalid_doubles(&ident);
                }
            });
        }
        "kind" => return set_kind(ed, key, &el, value),
        "dependent" => {
            let rel = el.id().unwrap_or("").to_string();
            let who = (!value.is_empty()).then(|| value.to_string());
            ed.apply("Dependent entity", |ed| {
                rules::set_dependent(ed.session_mut().doc_mut(), &rel, who.as_deref())
            });
        }
        "supertype" => change(ed, key, "Change supertype", |body| {
            if let Body::Node(n) = body
                && let Some(g) = &mut n.generalisation
            {
                g.supertype = value.to_string();
            }
        }),
        "overlap" | "coverage" => {
            let Body::Node(n) = &el.body else { return Outcome::Done };
            let (complete, disjoint) =
                parse_constraint(n.generalisation.as_ref().and_then(|g| g.constraint.as_deref()));
            let chosen = match value {
                "d" | "c" => Some(true),
                "o" | "i" => Some(false),
                _ => None,
            };
            let new = if id == "overlap" { letters(complete, chosen) } else { letters(chosen, disjoint) };
            change(ed, key, "Generalisation type", |body| {
                if let Body::Node(n) = body
                    && let Some(g) = &mut n.generalisation
                {
                    g.constraint = new;
                }
            });
        }
        "reverse" => {
            let Body::Edge(e) = &el.body else { return Outcome::Done };
            let (ok, why) = can_connect(ed.doc(), &e.command, &e.dst, &e.src);
            if !ok {
                return Outcome::Refused(format!("Not allowed: {why}"));
            }
            change(ed, key, "Reverse edge", |body| {
                if let Body::Edge(e) = body {
                    std::mem::swap(&mut e.src, &mut e.dst);
                }
            });
        }
        "card" => {
            let label = (!value.is_empty()).then(|| value.to_string());
            if matches!(&el.body, Body::Edge(e) if e.label == label) {
                return Outcome::Done;
            }
            change(ed, key, "Edit cardinality", |body| {
                if let Body::Edge(e) = body {
                    e.label = label;
                }
            });
        }
        "card.minmax" => return set_min_max(ed, key, &el, value),
        "card.deviate" => return deviate(ed, key, confirmed),
        "placement" => change(ed, key, "Cardinality placement", |body| {
            if let Some(o) = opts_mut(body) {
                o.remove("beside");
                o.remove("inline");
                if !value.is_empty() {
                    o.set(value, None);
                }
            }
        }),
        "orientation" => change(ed, key, "Label orientation", |body| {
            if let Some(o) = opts_mut(body) {
                o.remove("horizontal");
                o.remove("sloped");
                if !value.is_empty() {
                    o.set(value, None);
                }
            }
        }),
        "routing" => set_routing(ed, key, &el, value),
        "bend" => {
            let v = parse_float(value).unwrap_or(0.0).trunc().clamp(-85.0, 85.0);
            change(ed, key, "Change bend", |body| {
                if let Some(o) = opts_mut(body) {
                    for k in ["bend left", "bend right", "out", "in"] {
                        o.remove(k);
                    }
                    if v != 0.0 { o.set("bend", Some(&fmt_g(v))) } else { o.remove("bend") }
                }
            });
        }
        "more" => {
            let group = match &el.body {
                Body::Node(_) => "node",
                Body::Edge(_) => "edge",
                Body::Attributes(_) => "tree",
                Body::Note(_) => "note",
                Body::Subdomain(_) => "subdomain",
                _ => return Outcome::Done,
            };
            let new = Options::parse(Some(value));
            let cov = covered(group);
            change(ed, key, "Change options", |body| {
                if let Some(o) = opts_mut(body) {
                    o.items.retain(|(k, _)| cov.contains(&k.as_str()));
                    o.items.extend(new.items);
                }
            });
        }
        "add-tree" => {
            ed.apply("Add attributes", |ed| ed.add_attribute_list());
            return Outcome::Row { row: 0, edit: true };
        }
        "ovals-auto" => change(ed, key, "Place the ovals automatically", |body| {
            if let Body::Attributes(t) = body {
                fn clear(items: &mut [AttrItem]) {
                    for it in items {
                        it.pos = None;
                        clear(&mut it.parts);
                    }
                }
                clear(&mut t.items);
            }
        }),
        "make-rectangle" => {
            ed.make_rectangle(key);
        }
        _ => {}
    }
    Outcome::Done
}

fn touch(ed: &mut Editor, key: ElementKey) {
    if let Some(el) = ed.session_mut().doc_mut().get_mut(key) {
        el.touch();
    }
}

/// One undoable change of an element's body.
fn change(ed: &mut Editor, key: ElementKey, label: &str, f: impl FnOnce(&mut Body)) {
    ed.apply(label, |ed| {
        if let Some(el) = ed.session_mut().doc_mut().get_mut(key) {
            f(&mut el.body);
            el.touch();
        }
    });
}

/// Empty removes the option, `true` sets it as a flag, anything else as its value.
fn set_opt(ed: &mut Editor, key: ElementKey, k: &str, value: &str, label: &str) {
    change(ed, key, label, |body| {
        if let Some(o) = opts_mut(body) {
            match value {
                "" | "false" => o.remove(k),
                "true" => o.set(k, None),
                v => o.set(k, Some(v)),
            }
        }
    });
}

fn set_setup(ed: &mut Editor, k: &str, value: &str) {
    ed.apply("Diagram style", |ed| {
        let doc = ed.session_mut().doc_mut();
        let key = match setup_of(doc) {
            Some(e) => e.key,
            None if value.is_empty() => return,
            None => doc.insert(0, Body::Setup(Setup::default())),
        };
        let mut empty = false;
        if let Some(el) = doc.get_mut(key) {
            if let Some(o) = opts_mut(&mut el.body) {
                if value.is_empty() {
                    o.remove(k)
                } else {
                    o.set(k, Some(value))
                }
                empty = o.is_empty();
            }
            el.touch();
        }
        if empty {
            doc.remove(key);
        }
    });
}

fn set_notation(ed: &mut Editor, target: &str, confirmed: bool) -> Outcome {
    if target == diagram_notation(ed.doc()) || (target != N_NOTATION && target != MIN_MAX) {
        return Outcome::Done;
    }
    let (changes, skipped) = plan_diagram(ed.doc(), target);
    if !confirmed {
        let mut text = format!("{FLIP_WARNING}\n\n{} relationship(s) will be converted.", changes.len() / 2);
        if !skipped.is_empty() {
            text.push_str("\n\nThese stay as they are, and are marked in the problem list:\n");
            text.push_str(
                &skipped.iter().map(|(name, why)| format!("  {name}: {why}")).collect::<Vec<_>>().join("\n"),
            );
        }
        text.push_str(&format!("\n\nSwitch this diagram to {}?", notation_name(target)));
        return Outcome::Ask { title: "Cardinality notation".into(), text };
    }
    let minmax = target == MIN_MAX;
    ed.apply("Cardinality notation", |ed| {
        let doc = ed.session_mut().doc_mut();
        apply_changes(doc, &changes);
        let key = match setup_of(doc) {
            Some(e) => e.key,
            None if !minmax => return,
            None => doc.insert(0, Body::Setup(Setup::default())),
        };
        let mut empty = false;
        if let Some(el) = doc.get_mut(key) {
            if let Some(o) = opts_mut(&mut el.body) {
                if minmax {
                    o.set("card notation", Some(MIN_MAX));
                    o.remove("card symbols"); // crow's foot is for n-notation only
                } else {
                    o.remove("card notation");
                }
                empty = o.is_empty();
            }
            el.touch();
        }
        if empty {
            doc.remove(key);
        }
    });
    Outcome::Done
}

fn set_label(ed: &mut Editor, key: ElementKey, text: &str) {
    let tex = lines_to_tex(text);
    ed.apply("Edit label", |ed| {
        let doc = ed.session_mut().doc_mut();
        let mut rename = None;
        if let Some(el) = doc.get_mut(key) {
            match &mut el.body {
                Body::Node(n) => {
                    n.label = Some(tex.clone());
                    if is_default_id(&n.id) {
                        rename = Some((n.id.clone(), slug(Some(&tex), &n.id)));
                    }
                }
                Body::Note(n) => n.content = tex.clone(),
                Body::Subdomain(s) => s.label = tex.clone(),
                Body::Edge(e) => e.label = Some(strip(&tex).to_string()).filter(|l| !l.is_empty()),
                _ => {}
            }
            el.touch();
        }
        // nodes with a generated id get a better one from their label
        if let Some((old, base)) = rename {
            let new = doc.unique_id(&base);
            if new != old && !is_default_id(&new) {
                let _ = doc.rename(key, &new);
            }
        }
    });
}

fn set_kind(ed: &mut Editor, key: ElementKey, el: &Element, value: &str) -> Outcome {
    let Body::Node(_) = &el.body else {
        // a GSM edge
        let Body::Edge(e) = &el.body else { return Outcome::Done };
        let (ok, why) = can_connect(ed.doc(), value, &e.src, &e.dst);
        if !ok {
            return Outcome::Refused(format!("Not allowed: {why}"));
        }
        change(ed, key, "Change edge type", |body| {
            if let Body::Edge(e) = body {
                e.command = value.to_string();
            }
        });
        return Outcome::Done;
    };
    if matches!(&el.body, Body::Node(n) if n.generalisation.is_some()) {
        if value == "isa" || value == "canbe" {
            change(ed, key, "Change generalisation", |body| {
                if let Body::Node(n) = body {
                    n.command = value.to_string();
                }
            });
        }
        return Outcome::Done;
    }
    let ident = el.id().unwrap_or("").to_string();
    ed.apply("Change relationship type", |ed| {
        if let Some(o) = ed.session_mut().doc_mut().get_mut(key).and_then(|e| opts_mut(&mut e.body)) {
            o.remove("weak");
            o.remove("dependency");
            match value {
                "weak" => o.set("weak", None),
                "E" | "Id" => o.set("dependency", Some(value)),
                _ => {}
            }
        }
        touch(ed, key);
        ed.drop_invalid_doubles(&ident);
    });
    Outcome::Done
}

fn set_min_max(ed: &mut Editor, key: ElementKey, el: &Element, value: &str) -> Outcome {
    let (lo, hi) = value.split_once(',').unwrap_or((value, ""));
    let (lo, hi) = (strip(lo), strip(hi));
    if lo.is_empty() || hi.is_empty() {
        return Outcome::Done;
    }
    let lo_ok = lo == UNKNOWN_MIN || lo.bytes().all(|b| b.is_ascii_digit());
    let hi_ok = hi == "*" || (hi.bytes().all(|b| b.is_ascii_digit()) && !hi.starts_with('0'));
    if !lo_ok {
        return Outcome::Invalid(format!(
            "The minimum: 0, 1, any positive number, or {UNKNOWN_MIN} if it is unknown"
        ));
    }
    if !hi_ok {
        return Outcome::Invalid("The maximum: any positive number, or * for no limit".into());
    }
    let label = format!("({lo},{hi})");
    if !parse_card(Some(&label)).known {
        return Outcome::Invalid("The maximum can't be smaller than the minimum.".into());
    }
    if matches!(&el.body, Body::Edge(e) if e.label.as_deref() == Some(label.as_str())) {
        return Outcome::Done;
    }
    change(ed, key, "Edit cardinality", |body| {
        if let Body::Edge(e) = body {
            e.label = Some(label);
        }
    });
    Outcome::Done
}

fn deviate(ed: &mut Editor, key: ElementKey, confirmed: bool) -> Outcome {
    let doc = ed.doc();
    let Some(rid) = relationship_of(doc, key) else { return Outcome::Done };
    let wanted = diagram_notation(doc);
    let target = if relationship_notation(doc, &rid) == wanted {
        if wanted == N_NOTATION { MIN_MAX } else { N_NOTATION }
    } else {
        wanted
    };
    let (mut changes, why) = plan_relationship(doc, &rid, target);
    let sides = relationship_sides(doc, &rid);
    let text = match why {
        Some(_) if sides.len() > 2 => {
            changes = sides.iter().map(|s| (s.edge, None)).collect();
            format!(
                "For an n-ary relationship the two notations say different things, so its cardinalities can't be \
                 converted. Remove them and choose new ones in {}?",
                notation_name(target)
            )
        }
        Some(why) => {
            return Outcome::Inform {
                title: "Cardinality".into(),
                text: format!("This relationship can't be converted: {why}."),
            };
        }
        None => format!("{FLIP_WARNING}\n\nUse {} for this relationship?", notation_name(target)),
    };
    if !confirmed {
        return Outcome::Ask { title: "Cardinality notation".into(), text };
    }
    if changes.is_empty() {
        // nothing labelled yet: start with the other notation
        if let Some(s) = sides.first() {
            changes = vec![(s.edge, Some(if target == MIN_MAX { "(0,*)" } else { "cn" }.to_string()))];
        }
    }
    ed.apply("Cardinality notation", |ed| apply_changes(ed.session_mut().doc_mut(), &changes));
    Outcome::Done
}

fn set_routing(ed: &mut Editor, key: ElementKey, el: &Element, value: &str) {
    let Body::Edge(e) = &el.body else { return };
    let via = e.opts.get("via").filter(|v| !v.is_empty()).map(str::to_string);
    let via = match (value, via) {
        ("via", None) => Some(default_waypoint(ed, e.src_id(), e.dst_id())),
        (_, v) => v,
    };
    change(ed, key, "Change routing", |body| {
        if let Some(o) = opts_mut(body) {
            for k in ["via", "ortho", "bend", "bend left", "bend right", "out", "in", "looseness"] {
                o.remove(k);
            }
            match value {
                "curved" => o.set("bend", Some("30")),
                "-|" | "|-" => o.set("ortho", Some(value)),
                "via" => o.set("via", via.as_deref()),
                _ => {}
            }
        }
    });
}

/// A way point at the corner of a right angle between the two ends.
fn default_waypoint(ed: &mut Editor, src: &str, dst: &str) -> String {
    let (src, dst) = (src.to_string(), dst.to_string());
    ed.session_mut().with_layout(|_, lay| {
        let (Some(a), Some(b)) = (lay.shapes.get(&src), lay.shapes.get(&dst)) else {
            return "(0,0)".to_string();
        };
        let cm = crate::geometry::CM;
        let x = (b.cx / cm * 4.0).round_ties_even() / 4.0;
        let y = (a.cy / cm * 4.0).round_ties_even() / 4.0;
        Coord::numeric(x, y).to_tex()
    })
}

// -- the attribute table ----------------------------------------------------------------------

/// `rest` is `<what>.<row>`.
fn set_attribute(ed: &mut Editor, key: ElementKey, rest: &str, value: &str) -> Outcome {
    let (what, row) = rest.rsplit_once('.').unwrap_or((rest, "-1"));
    let row: i64 = row.parse().unwrap_or(-1);
    let Some(Body::Attributes(t)) = ed.doc().get(key).map(|e| e.body.clone()) else { return Outcome::Done };
    let mut flat: Vec<(AttrItem, usize)> =
        flatten_items(&t.items).into_iter().map(|(i, d)| (i.clone(), d)).collect();
    let k = usize::try_from(row).ok().filter(|k| *k < flat.len());
    let save = |ed: &mut Editor, label: &str, flat: Vec<(AttrItem, usize)>| {
        let items = nest_items(flat);
        change(ed, key, label, |body| {
            if let Body::Attributes(t) = body {
                t.items = items;
            }
        });
    };
    match (what, k) {
        ("name", Some(k)) => {
            let name = strip(value).to_string();
            if name.is_empty() {
                flat.remove(k); // as the Python app: a row without a name goes
            } else if let Some((it, _)) = flat.get_mut(k) {
                if it.text == name {
                    return Outcome::Done;
                }
                it.text = name;
            }
            save(ed, "Edit attributes", flat);
            Outcome::Row { row: k.min(flat_len(ed, key).saturating_sub(1)), edit: false }
        }
        ("key" | "unique", Some(k)) => {
            let on = value == "true";
            if let Some((it, _)) = flat.get_mut(k) {
                // one or the other: what was just ticked counts
                if what == "key" {
                    it.key = on;
                    if on {
                        it.unique = false;
                    }
                } else {
                    it.unique = on;
                    if on {
                        it.key = false;
                    }
                }
            }
            save(ed, "Edit attributes", flat);
            Outcome::Row { row: k, edit: false }
        }
        ("type", Some(k)) => {
            let v = strip(value);
            if let Some((it, _)) = flat.get_mut(k) {
                let new = (!v.is_empty()).then(|| v.to_string());
                if !it.parts.is_empty() || it.r#type == new {
                    return Outcome::Done;
                }
                it.r#type = new;
            }
            save(ed, "Edit attributes", flat);
            Outcome::Row { row: k, edit: false }
        }
        ("kind", Some(k)) => {
            if let Some((it, _)) = flat.get_mut(k) {
                it.multivalued = value == "multi-valued";
                it.derived = value == "derived";
                if !it.derived {
                    it.formula = None;
                }
            }
            save(ed, "Attribute kind", flat);
            Outcome::Row { row: k, edit: false }
        }
        ("formula", Some(k)) => {
            let v = strip(value);
            if let Some((it, _)) = flat.get_mut(k) {
                if it.formula.as_deref().unwrap_or("") == v {
                    return Outcome::Done;
                }
                it.formula = (!v.is_empty()).then(|| v.to_string());
            }
            save(ed, "Attribute formula", flat);
            Outcome::Row { row: k, edit: false }
        }
        ("add" | "next", k) => {
            let (end, depth) = match k {
                Some(k) => (subtree_end(&flat, k), flat.get(k).map_or(0, |(_, d)| *d)),
                None => (flat.len(), 0),
            };
            flat.insert(end, (AttrItem::new("attribute"), depth));
            save(ed, "Add attribute", flat);
            Outcome::Row { row: end, edit: what == "next" }
        }
        ("remove", Some(k)) => {
            let end = subtree_end(&flat, k);
            flat.drain(k..end);
            save(ed, "Remove attribute", flat);
            Outcome::Row { row: k, edit: false }
        }
        ("up" | "down", Some(k)) => match reorder(&mut flat, k, what == "up") {
            Some(row) => {
                save(ed, "Reorder attributes", flat);
                Outcome::Row { row, edit: false }
            }
            None => Outcome::Done,
        },
        ("indent", Some(k)) => {
            let ok = k > 0 && flat.get(k - 1).zip(flat.get(k)).is_some_and(|(a, b)| a.1 >= b.1);
            if !ok {
                return Outcome::Done;
            }
            let end = subtree_end(&flat, k);
            for (_, d) in flat.iter_mut().take(end).skip(k) {
                *d += 1;
            }
            save(ed, "Make part of a composite attribute", flat);
            Outcome::Row { row: k, edit: false }
        }
        ("outdent", Some(k)) => {
            let depth = flat.get(k).map_or(0, |(_, d)| *d);
            if depth == 0 {
                return Outcome::Done;
            }
            let end = subtree_end(&flat, k);
            // behind the other parts, which would otherwise become its parts
            let mut stop = end;
            while flat.get(stop).is_some_and(|(_, d)| *d >= depth) {
                stop += 1;
            }
            let block: Vec<(AttrItem, usize)> = flat.drain(k..end).map(|(i, d)| (i, d - 1)).collect();
            let at = k + (stop - end);
            for (n, item) in block.into_iter().enumerate() {
                flat.insert(at + n, item);
            }
            save(ed, "Not a part of a composite attribute", flat);
            Outcome::Row { row: at, edit: false }
        }
        _ => Outcome::Done,
    }
}

fn flat_len(ed: &Editor, key: ElementKey) -> usize {
    match ed.doc().get(key).map(|e| &e.body) {
        Some(Body::Attributes(t)) => flatten_items(&t.items).len(),
        _ => 0,
    }
}

/// After the item `k` and its parts.
fn subtree_end(flat: &[(AttrItem, usize)], k: usize) -> usize {
    let depth = flat.get(k).map_or(0, |(_, d)| *d);
    let mut end = k + 1;
    while flat.get(end).is_some_and(|(_, d)| *d > depth) {
        end += 1;
    }
    end
}

/// Moves item `k` (with its parts) past its neighbour of the same depth; its new row.
fn reorder(flat: &mut Vec<(AttrItem, usize)>, k: usize, up: bool) -> Option<usize> {
    let depth = flat.get(k)?.1;
    let end = subtree_end(flat, k);
    if up {
        let mut j = k.checked_sub(1)?;
        while flat.get(j)?.1 > depth {
            j = j.checked_sub(1)?;
        }
        if flat.get(j)?.1 < depth {
            return None;
        }
        let moved: Vec<_> = flat.drain(k..end).collect();
        for (n, item) in moved.into_iter().enumerate() {
            flat.insert(j + n, item);
        }
        Some(j)
    } else {
        if flat.get(end)?.1 != depth {
            return None;
        }
        let end2 = subtree_end(flat, end);
        let moved: Vec<_> = flat.drain(end..end2).collect();
        for (n, item) in moved.into_iter().enumerate() {
            flat.insert(k + n, item);
        }
        Some(k + end2 - end)
    }
}

/// The usual data types, for the type fields.
pub fn type_presets() -> &'static [&'static str] {
    TYPE_PRESETS
}
