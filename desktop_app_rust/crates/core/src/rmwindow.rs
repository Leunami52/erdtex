//! The Relational Schema / SQL window, as data (a port of `sqlexport.py`): the output in the
//! chosen format, the decisions with the relations each option gives, the data types, and the
//! problems; and the changes the window writes into the diagram (each one undoable step).

use crate::document::{Document, Kind};
use crate::editor::Editor;
use crate::latex_text::plain_text;
use crate::model::{AttrItem, Body, ElementKey, Options as Opts, flatten_items};
use crate::relational::names::guess_type;
use crate::relational::transform;
use crate::relational::{FkNaming, Overrides, RelNaming, Settings};
use crate::relational::{Marks, Quote, render, to_text};
use crate::rules::Severity;

/// What the window shows the schema with (kept in the settings as `rm/...`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// `text`, `latex`, `postgresql`, `oracle` or `sqlite`
    pub format: String,
    /// `entities` or `relationship`
    pub rel_naming: String,
    /// `plain` or `relationship`
    pub fk_naming: String,
    /// `underscore` or `tags`
    pub marks: String,
    pub text_types: bool,
    pub quote_all: bool,
    pub drop: bool,
    /// the diagram file's name, for the comment on top
    pub source: String,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            format: "text".into(),
            rel_naming: "entities".into(),
            fk_naming: "plain".into(),
            marks: "underscore".into(),
            text_types: true,
            quote_all: false,
            drop: false,
            source: String::new(),
        }
    }
}

impl Options {
    fn settings(&self) -> Settings {
        Settings {
            rel_naming: if self.rel_naming == "relationship" {
                RelNaming::Relationship
            } else {
                RelNaming::Entities
            },
            fk_naming: if self.fk_naming == "relationship" {
                FkNaming::Relationship
            } else {
                FkNaming::Plain
            },
            ..Settings::default()
        }
    }

    fn marks(&self) -> Marks {
        if self.marks == "tags" { Marks::Tags } else { Marks::Underscore }
    }
}

/// One option of a decision, with the relations it gives.
#[derive(Clone, Debug, PartialEq)]
pub struct OptionView {
    pub title: String,
    pub description: String,
    pub reason: String,
    pub valid: bool,
    /// the relations of the decision's elements under this option (plain text)
    pub result: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecisionView {
    pub element_id: String,
    pub title: String,
    pub question: String,
    pub current: usize,
    pub default: usize,
    pub minor: bool,
    pub options: Vec<OptionView>,
    /// the rule in the rule books that describes it
    pub rule: &'static str,
}

/// An attribute in the data types table.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeRow {
    pub owner: String,
    pub name: String,
    /// the type stored, or else the one guessed from the name
    pub r#type: String,
    pub guessed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProblemRow {
    pub severity: Severity,
    pub text: String,
    pub element_id: Option<String>,
}

/// What the window shows.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct View {
    /// false for a GSM diagram: only `output` says so
    pub er: bool,
    pub output: String,
    pub decisions: Vec<DecisionView>,
    pub types: Vec<TypeRow>,
    pub problems: Vec<ProblemRow>,
    /// the number of problems of the transformation (the tab's title)
    pub issues: usize,
    pub relations: usize,
    pub types_shown: bool,
}

impl View {
    /// "3 relations, 1 decision(s)"
    pub fn summary(&self) -> String {
        let majors = self.decisions.iter().filter(|d| !d.minor).count();
        format!(
            "{} relation{}, {majors} decision(s)",
            self.relations,
            if self.relations == 1 { "" } else { "s" }
        )
    }
}

/// Where a type row's type is kept: a Chen attribute (`None`), or the n-th item of a list.
/// (element, the item of a list or `None` for a Chen attribute, owner, name, type kept)
type Target = (ElementKey, Option<usize>, String, String, Option<String>);

fn type_targets(doc: &Document) -> Vec<Target> {
    let mut rows = Vec::new();
    for el in &doc.elements {
        match &el.body {
            Body::Attributes(t) => {
                let owner = doc
                    .by_id(&t.node)
                    .and_then(|e| e.as_node())
                    .and_then(|n| n.label.as_deref())
                    .map(plain_text)
                    .filter(|l| !l.is_empty())
                    .unwrap_or_else(|| t.node.clone());
                for (k, (it, _)) in flatten_items(&t.items).into_iter().enumerate() {
                    if it.parts.is_empty() {
                        // a composite attribute is stored as its parts
                        let name = plain_text(&it.text);
                        let name = if name.is_empty() { it.text.clone() } else { name };
                        rows.push((el.key, Some(k), owner.clone(), name, it.r#type.clone()));
                    }
                }
            }
            Body::Node(n) if n.command == "attribute" => {
                let name = plain_text(n.label.as_deref().unwrap_or(""));
                let name = if name.is_empty() { n.id.clone() } else { name };
                let owner = n.opts.get("of").unwrap_or("").to_string();
                rows.push((el.key, None, owner, name, n.opts.get("type").map(str::to_string)));
            }
            _ => {}
        }
    }
    rows
}

/// The window's contents for a document.
pub fn view(doc: &Document, opts: &Options) -> View {
    if doc.kind != Kind::Er {
        return View {
            output: "The relational transformation is available for ER diagrams.".into(),
            ..View::default()
        };
    }
    let st = opts.settings();
    let schema = transform(doc, &st, &Overrides::new());
    let quote = if opts.quote_all { Quote::All } else { Quote::Needed };
    let output = render(&schema, &opts.format, &opts.source, opts.marks(), opts.text_types, quote, opts.drop);
    let decisions = schema
        .decisions
        .iter()
        .map(|d| DecisionView {
            element_id: d.element_id.clone(),
            title: d.title.clone(),
            question: d.question.clone(),
            current: d.current,
            default: d.default,
            minor: d.minor,
            rule: d.rule,
            options: d
                .options
                .iter()
                .map(|o| {
                    let result = if o.valid {
                        // the relations of the decision's elements under this option, as
                        // their lines of the plain text (`Name : {(...)}`)
                        let mut over = Overrides::new();
                        over.insert(d.element_id.clone(), o.assign.iter().cloned().collect());
                        let sch = transform(doc, &st, &over);
                        let text = to_text(&sch, opts.marks(), false, "");
                        let lines: Vec<&str> = text.lines().collect();
                        let found: Vec<String> = sch
                            .relations()
                            .filter(|r| !r.sources.is_disjoint(&d.involved))
                            .map(|r| {
                                lines
                                    .iter()
                                    .find(|ln| ln.split(" : ").next() == Some(r.name.as_str()))
                                    .map_or(r.name.clone(), |ln| ln.to_string())
                            })
                            .collect();
                        if found.is_empty() { "(no relations)".into() } else { found.join("\n") }
                    } else {
                        String::new()
                    };
                    OptionView {
                        title: o.title.clone(),
                        description: o.description.clone(),
                        reason: o.reason.clone(),
                        valid: o.valid,
                        result,
                    }
                })
                .collect(),
        })
        .collect();
    let types = type_targets(doc)
        .into_iter()
        .map(|(_, _, owner, name, t)| match t.filter(|t| !t.is_empty()) {
            Some(t) => TypeRow { owner, name, r#type: t, guessed: false },
            None => TypeRow { owner, r#type: guess_type(&name).into(), name, guessed: true },
        })
        .collect();
    let mut problems: Vec<ProblemRow> = schema
        .issues
        .iter()
        .map(|i| ProblemRow {
            severity: i.severity,
            text: i.message.clone(),
            element_id: i.element_id.clone(),
        })
        .collect();
    problems.extend(schema.notes.iter().map(|n| ProblemRow {
        severity: Severity::Hint,
        text: n.clone(),
        element_id: None,
    }));
    problems.extend(schema.unenforced.iter().map(|t| ProblemRow {
        severity: Severity::Hint,
        text: format!("Not enforced by the schema: {t}"),
        element_id: None,
    }));
    View {
        er: true,
        output,
        decisions,
        types,
        problems,
        issues: schema.issues.len(),
        relations: schema.relations.len(),
        types_shown: doc.types_shown(),
    }
}

fn opts_mut(body: &mut Body) -> Option<&mut Opts> {
    match body {
        Body::Node(n) => Some(&mut n.opts),
        Body::Edge(e) => Some(&mut e.opts),
        _ => None,
    }
}

/// Chooses option `index` of the decision about `element_id`.
pub fn choose(ed: &mut Editor, element_id: &str, index: usize, opts: &Options) -> bool {
    let schema = transform(ed.doc(), &opts.settings(), &Overrides::new());
    let Some(d) = schema.decisions.iter().find(|d| d.element_id == element_id) else { return false };
    if index == d.current {
        return false;
    }
    let Some(choice) = d.options.get(index).filter(|o| o.valid) else { return false };
    let Some(key) = ed.doc().by_id(element_id).map(|e| e.key) else { return false };
    let assign = choice.assign.clone();
    ed.apply("Relational mapping", |ed| {
        if let Some(el) = ed.session_mut().doc_mut().get_mut(key) {
            if let Some(o) = opts_mut(&mut el.body) {
                for (k, v) in &assign {
                    match v {
                        None => o.remove(k),
                        Some(v) => o.set(k, Some(v)),
                    }
                }
            }
            el.touch();
        }
    })
}

/// The n-th item of a list, top to bottom (as `flatten_items` counts).
fn nth_item(items: &mut [AttrItem], n: usize) -> Option<&mut AttrItem> {
    fn walk<'a>(items: &'a mut [AttrItem], n: &mut usize) -> Option<&'a mut AttrItem> {
        for it in items {
            if *n == 0 {
                return Some(it);
            }
            *n -= 1;
            if let Some(found) = walk(&mut it.parts, n) {
                return Some(found);
            }
        }
        None
    }
    let mut n = n;
    walk(items, &mut n)
}

fn set_type_at(doc: &mut Document, key: ElementKey, item: Option<usize>, value: &str) {
    let Some(el) = doc.get_mut(key) else { return };
    match (&mut el.body, item) {
        (Body::Attributes(t), Some(k)) => {
            if let Some(it) = nth_item(&mut t.items, k) {
                it.r#type = (!value.is_empty()).then(|| value.to_string());
            }
        }
        (Body::Node(n), None) => {
            if value.is_empty() {
                n.opts.remove("type")
            } else {
                n.opts.set("type", Some(value))
            }
        }
        _ => return,
    }
    el.touch();
}

/// The type of row `row` of the data types table.
pub fn set_type(ed: &mut Editor, row: usize, value: &str) -> bool {
    let value = crate::text::strip(value).to_string();
    let Some((key, item, _, _, old)) = type_targets(ed.doc()).into_iter().nth(row) else { return false };
    if value == old.clone().unwrap_or_default() {
        return false;
    }
    ed.apply("Set data type", |ed| set_type_at(ed.session_mut().doc_mut(), key, item, &value))
}

/// Writes every guessed type into the diagram.
pub fn store_guesses(ed: &mut Editor) -> bool {
    let rows: Vec<_> =
        type_targets(ed.doc()).into_iter().filter(|r| r.4.as_deref().is_none_or(str::is_empty)).collect();
    if rows.is_empty() {
        return false;
    }
    ed.apply("Store data types", |ed| {
        for (key, item, _, name, _) in &rows {
            set_type_at(ed.session_mut().doc_mut(), *key, *item, guess_type(name));
        }
    })
}
