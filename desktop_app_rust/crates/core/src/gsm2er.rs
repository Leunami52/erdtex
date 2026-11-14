//! GSM diagram -> ER diagram.  The GSM document is only read; the result is a *new* ER
//! document.
//!
//! ```text
//! abstract type, subtype          -> entity type
//! single-valued attribute
//!    to a representable type      -> attribute (key if it names an identifier)
//!    to an object type            -> relationship  (target side 1, source side cn)
//! multi-valued attribute
//!    to a representable type      -> entity + relationship (cn:cn)
//!                                    (choice: or a multi-valued attribute: \{value\})
//!    to an object type            -> relationship (cn:cn)
//! grouping {C}                    -> multi-valued reference to C (the grouping's attribute
//!                                    edge names the relationship)
//! aggregation
//!    of object types, not referenced     -> relationship (choice: entity)
//!    of representables, referenced once  -> entity (choice: composite attribute: a{b, c})
//!    otherwise                           -> entity + relationships to its object components
//!                                           (component side 1)
//! attribute pairs in both directions between two object types (inverses, e.g. lives in /
//! home of)                        -> one relationship                                (choice)
//! generalization (disjoint)       -> IS_A, cd  (the supertype is generated)
//! specialization (overlapping)    -> IS_A, io
//! derived attributes ([...])      -> derived attributes (/age)
//! ```
//!
//! GSM has no keys: an attribute that names an identifier (has Id, ...Id, ...No) becomes the
//! key, otherwise a key `<Name> ID` is added.  GSM has no cardinalities
//! either; minimum cardinalities are assumed to be 0 (cn), single-valued references give 1.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::document::{Document, Kind};
use crate::latex_text::plain_text;
use crate::model::{AttrItem, AttributeTree, Body, Coord, Edge, ElementKey, Node, Options};
use crate::texscan::ParseError;
use crate::text::{is_space, py_mod, py_sum, round_half_even, strip};

const OBJECT: &[&str] = &["abstract", "subtype"];
const GENERIC_TYPES: &[&str] = &[
    "number", "string", "text", "integer", "int", "real", "float", "decimal", "boolean", "bool", "date",
    "time", "datetime", "amount", "value", "count",
];

/// A question the conversion asks: its options are (value, title, explanation).
#[derive(Clone, Debug, PartialEq)]
pub struct ConversionChoice {
    pub key: String,
    pub title: String,
    pub question: String,
    pub options: Vec<(String, String, String)>,
    pub default: String,
    pub current: String,
    /// the rule in the rule books that describes it
    pub rule: &'static str,
}

/// Where attribute names come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AttrNames {
    /// the edge label where the type is generic or used several times, else the type
    #[default]
    Auto,
    /// the representable type's name
    Type,
    /// the edge label
    Edge,
}

/// What the conversion is asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversionSettings {
    pub attr_names: AttrNames,
    pub add_keys: bool,
    /// answers to the questions: key -> value
    pub choices: HashMap<String, String>,
}

impl Default for ConversionSettings {
    fn default() -> Self {
        ConversionSettings { attr_names: AttrNames::Auto, add_keys: true, choices: HashMap::new() }
    }
}

/// The ER diagram, the questions asked, and what happened.
#[derive(Clone, Debug)]
pub struct Conversion {
    pub doc: Document,
    pub choices: Vec<ConversionChoice>,
    pub report: Vec<String>,
}

/// `re.sub(r"[^A-Za-z0-9]+", "_", plain_text(text)).strip("_").lower() or "r"`
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in plain_text(text).chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            gap = false;
        } else if !gap {
            out.push('_');
            gap = true;
        }
    }
    let s = out.trim_matches('_').to_lowercase();
    if s.is_empty() { "r".into() } else { s }
}

/// Python's `\w`, roughly: letters, digits and the underscore.
fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `word` occurs in `text` as a whole word, ignoring case.
fn has_word(text: &str, word: &str) -> bool {
    let chars: Vec<char> = text.to_lowercase().chars().collect();
    let w: Vec<char> = word.chars().collect();
    (0..chars.len()).any(|i| {
        chars.get(i..i + w.len()) == Some(w.as_slice())
            && (i == 0 || chars.get(i - 1).is_none_or(|c| !word_char(*c)))
            && chars.get(i + w.len()).is_none_or(|c| !word_char(*c))
    })
}

/// `Person Id`, `has Id`, `orderNo` ... -> true
pub fn names_key(name: &str, edge_label: &str) -> bool {
    let (n, lab) = (plain_text(name), plain_text(edge_label));
    if has_word(&lab, "id") || lab.to_lowercase().contains("identif") || has_word(&lab, "key") {
        return true;
    }
    if has_word(&n, "id") || has_word(&n, "key") {
        return true;
    }
    let chars: Vec<char> = n.to_lowercase().chars().collect();
    let ends = ["id", "no", "nr"].iter().any(|e| {
        let e: Vec<char> = e.chars().collect();
        chars.len() > 2
            && chars.ends_with(&e)
            && chars.get(chars.len() - 3).is_some_and(char::is_ascii_lowercase)
    });
    ends || matches!(n.to_lowercase().as_str(), "id" | "no" | "nr")
}

/// `Num.\\languages\\{}[\textbar speaks\textbar]` -> (`Num. languages`, true)
pub fn strip_derivation(label: &str) -> (String, bool) {
    let text = plain_text(label);
    let chars: Vec<char> = text.chars().collect();
    // re.search(r"\[.*\]"): a [ with a ] after it on the same line
    let derived = chars
        .iter()
        .enumerate()
        .any(|(i, c)| *c == '[' && chars.iter().skip(i + 1).take_while(|d| **d != '\n').any(|d| *d == ']'));
    // re.sub(r"\s*\[.*?\]\s*", " ", text)
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let ws = chars.iter().skip(i).take_while(|c| is_space(**c)).count();
        let open = i + ws;
        let close = if chars.get(open) == Some(&'[') {
            chars
                .iter()
                .enumerate()
                .skip(open + 1)
                .take_while(|(_, c)| **c != '\n')
                .find(|(_, c)| **c == ']')
                .map(|(j, _)| j)
        } else {
            None
        };
        match close {
            Some(j) => {
                let after = chars.iter().skip(j + 1).take_while(|c| is_space(**c)).count();
                out.push(' ');
                i = j + 1 + after;
            }
            None => {
                if let Some(c) = chars.get(i) {
                    out.push(*c);
                }
                i += 1;
            }
        }
    }
    (strip(&out).to_string(), derived)
}

/// A side of a relationship to be: (entity id, cardinality, link options).
type Side = (String, String, Vec<(String, String)>);

/// A relationship to be.
#[derive(Clone, Debug)]
struct Rel {
    id: String,
    label: String,
    sides: Vec<Side>,
    attrs: Vec<AttrItem>,
    pos: Option<(f64, f64)>,
}

impl Rel {
    fn new(id: String, label: String, sides: Vec<Side>) -> Rel {
        Rel { id, label, sides, attrs: vec![], pos: None }
    }

    fn entity_set(&self) -> BTreeSet<String> {
        self.sides.iter().map(|s| s.0.clone()).collect()
    }
}

/// An object reference: (source, target, multi-valued, edge).
type Reference<'a> = (String, String, bool, &'a Edge);

struct Converter<'a> {
    g: &'a Document,
    st: &'a ConversionSettings,
    choices: Vec<ConversionChoice>,
    report: Vec<String>,
    nodes: HashMap<&'a str, &'a Node>,
    out: HashMap<&'a str, Vec<(ElementKey, &'a Edge)>>,
    inc: HashMap<&'a str, Vec<(ElementKey, &'a Edge)>>,
    ids: HashSet<String>,
    entities: Vec<String>,
    ent_label: HashMap<String, String>,
    ent_pos: HashMap<String, (f64, f64)>,
    ent_of: HashMap<String, String>,
    from_repr: HashSet<String>,
    attrs: HashMap<String, Vec<(AttrItem, String)>>,
    rels: Vec<Rel>,
    isas: Vec<(String, String, Vec<String>, String)>,
    points: HashSet<&'a str>,
    pair_attrs: Vec<(BTreeSet<String>, AttrItem)>,
    junction_links: HashSet<ElementKey>,
    agg_mode: HashMap<String, String>,
    anchor: HashMap<String, &'static str>,
}

fn item(text: &str) -> AttrItem {
    AttrItem { text: text.to_string(), ..AttrItem::default() }
}

impl<'a> Converter<'a> {
    fn new(g: &'a Document, st: &'a ConversionSettings) -> Self {
        let mut c = Converter {
            g,
            st,
            choices: vec![],
            report: vec![],
            nodes: HashMap::new(),
            out: HashMap::new(),
            inc: HashMap::new(),
            ids: HashSet::new(),
            entities: vec![],
            ent_label: HashMap::new(),
            ent_pos: HashMap::new(),
            ent_of: HashMap::new(),
            from_repr: HashSet::new(),
            attrs: HashMap::new(),
            rels: vec![],
            isas: vec![],
            points: g.points().map(|p| p.id.as_str()).collect(),
            pair_attrs: vec![],
            junction_links: HashSet::new(),
            agg_mode: HashMap::new(),
            anchor: HashMap::new(),
        };
        for n in g.nodes() {
            c.nodes.insert(&n.id, n);
        }
        for el in &g.elements {
            if let Body::Edge(e) = &el.body {
                c.out.entry(e.src_id()).or_default().push((el.key, e));
                c.inc.entry(e.dst_id()).or_default().push((el.key, e));
            }
        }
        c
    }

    // -- helpers ------------------------------------------------------------------------

    fn outgoing(&self, ident: &str) -> Vec<(ElementKey, &'a Edge)> {
        self.out.get(ident).cloned().unwrap_or_default()
    }

    fn incoming(&self, ident: &str) -> Vec<(ElementKey, &'a Edge)> {
        self.inc.get(ident).cloned().unwrap_or_default()
    }

    fn kind(&self, ident: &str) -> Option<&'a str> {
        self.nodes.get(ident).map(|n| n.command.as_str())
    }

    fn label(&self, ident: &str) -> String {
        match self.nodes.get(ident).and_then(|n| n.label.as_deref()) {
            Some(l) if !l.is_empty() => l.to_string(),
            _ => ident.to_string(),
        }
    }

    fn name(&self, ident: &str) -> String {
        let n = plain_text(&self.label(ident));
        if n.is_empty() { ident.to_string() } else { n }
    }

    fn pos(&self, ident: &str) -> (f64, f64) {
        match self.nodes.get(ident).and_then(|n| n.pos.as_ref()) {
            Some(Coord::Numeric { x, y }) => (*x, *y),
            _ => (0.0, 0.0),
        }
    }

    fn new_id(&mut self, base: &str) -> String {
        let base: String = base
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();
        let base = if base.is_empty() { "x".to_string() } else { base };
        let (mut ident, mut k) = (base.clone(), 2);
        while self.ids.contains(&ident) {
            ident = format!("{base}{k}");
            k += 1;
        }
        self.ids.insert(ident.clone());
        ident
    }

    fn choose(
        &mut self,
        rule: &'static str,
        key: String,
        title: String,
        question: String,
        options: Vec<(&str, &str, String)>,
        default: &str,
    ) -> String {
        let options: Vec<(String, String, String)> =
            options.into_iter().map(|(v, t, e)| (v.to_string(), t.to_string(), e)).collect();
        let mut current = self.st.choices.get(&key).cloned().unwrap_or_else(|| default.to_string());
        if !options.iter().any(|o| o.0 == current) {
            current = default.to_string();
        }
        self.choices.push(ConversionChoice {
            key,
            title,
            question,
            options,
            default: default.into(),
            current: current.clone(),
            rule,
        });
        current
    }

    fn entity(&mut self, gsm_id: &str, label: Option<String>, pos: Option<(f64, f64)>) -> String {
        if let Some(e) = self.ent_of.get(gsm_id) {
            return e.clone();
        }
        let eid = self.new_id(gsm_id);
        self.ent_of.insert(gsm_id.to_string(), eid.clone());
        self.entities.push(eid.clone());
        let label = label.unwrap_or_else(|| self.label(gsm_id));
        self.ent_label.insert(eid.clone(), label);
        let pos = pos.unwrap_or_else(|| self.pos(gsm_id));
        self.ent_pos.insert(eid.clone(), pos);
        eid
    }

    fn ent(&mut self, gsm_id: &str) -> String {
        self.entity(gsm_id, None, None)
    }

    fn attrs_of(&mut self, eid: &str) -> &mut Vec<(AttrItem, String)> {
        self.attrs.entry(eid.to_string()).or_default()
    }

    /// An entity for a representable type (e.g. Language), key: Name.
    fn repr_entity(&mut self, rid: &str) -> String {
        if !self.ent_of.contains_key(rid) {
            let eid = self.ent(rid);
            self.from_repr.insert(eid.clone());
            self.attrs_of(&eid).push((AttrItem { key: true, ..item("Name") }, String::new()));
        }
        self.ent_of.get(rid).cloned().unwrap_or_default()
    }

    fn attr_name(&self, target: &str, edge: &Edge) -> String {
        let (edge_label, derived) = strip_derivation(edge.label.as_deref().unwrap_or(""));
        let has = !edge_label.is_empty();
        if derived && has {
            return edge_label;
        }
        match self.st.attr_names {
            AttrNames::Edge if has => return edge_label,
            AttrNames::Auto if has => {
                if GENERIC_TYPES.contains(&self.name(target).to_lowercase().as_str()) {
                    return edge_label;
                }
                let uses = self
                    .incoming(target)
                    .iter()
                    .filter(|(_, e)| {
                        matches!(e.command.as_str(), "attribute" | "multiattribute" | "component")
                    })
                    .count();
                if uses > 1 {
                    return edge_label; // a generic type used several times
                }
            }
            _ => {}
        }
        self.name(target)
    }

    /// The ER entity that gets the attributes of GSM node `src`.
    fn owner(&mut self, src: &str) -> Option<String> {
        let k = self.kind(src);
        let object = k.is_some_and(|k| OBJECT.contains(&k));
        let entity_agg =
            k == Some("aggregation") && self.agg_mode.get(src).map(String::as_str) == Some("entity");
        (object || entity_agg).then(|| self.ent(src))
    }

    // -- main -----------------------------------------------------------------------------

    fn run(mut self, source: Option<&str>) -> Result<Conversion, ParseError> {
        let g = self.g;
        for n in g.nodes() {
            if OBJECT.contains(&n.command.as_str()) {
                self.ent(&n.id);
            }
        }
        self.classify_aggregations();
        for n in g.nodes() {
            if n.command == "aggregation" && self.agg_mode.get(&n.id).map(String::as_str) == Some("entity") {
                self.ent(&n.id);
            }
        }
        let mut refs: Vec<Reference<'a>> = Vec::new();
        for el in &g.elements {
            if let Body::Edge(e) = &el.body
                && matches!(e.command.as_str(), "attribute" | "multiattribute")
                && let Some(r) = self.attribute_edge(e)
            {
                refs.push(r);
            }
        }
        for el in &g.elements {
            if let Body::Edge(e) = &el.body
                && e.command == "link"
                && !self.junction_links.contains(&el.key)
            {
                let msg = format!(
                    "the plain link {} – {} has no GSM meaning and was not converted",
                    self.name(e.src_id()),
                    self.name(e.dst_id())
                );
                self.report.push(msg);
            }
        }
        self.object_references(&refs);
        self.attach_pair_attributes();
        for n in g.nodes() {
            if n.command == "aggregation" {
                self.aggregation(n);
            } else if n.command == "grouping"
                && !self
                    .incoming(&n.id)
                    .iter()
                    .any(|(_, e)| matches!(e.command.as_str(), "attribute" | "multiattribute" | "component"))
            {
                self.lone_grouping(n);
            }
        }
        self.generalisations();
        self.keys();
        let doc = self.build(source)?;
        Ok(Conversion { doc, choices: self.choices, report: self.report })
    }

    // -- aggregations -----------------------------------------------------------------------

    fn classify_aggregations(&mut self) {
        for n in self.g.nodes() {
            if n.command != "aggregation" {
                continue;
            }
            let comps: Vec<&Edge> =
                self.outgoing(&n.id).into_iter().map(|x| x.1).filter(|e| e.command == "component").collect();
            let objs: Vec<&Edge> = comps
                .iter()
                .copied()
                .filter(|e| {
                    self.kind(e.dst_id())
                        .is_some_and(|k| OBJECT.contains(&k) || k == "aggregation" || k == "grouping")
                })
                .collect();
            let refs: Vec<&Edge> = self
                .incoming(&n.id)
                .into_iter()
                .map(|x| x.1)
                .filter(|e| matches!(e.command.as_str(), "attribute" | "multiattribute" | "component"))
                .collect();
            let name = self.name(&n.id);
            let mode = if refs.is_empty() && objs.len() >= 2 {
                let parts = objs.iter().map(|e| self.name(e.dst_id())).collect::<Vec<_>>().join(", ");
                self.choose(
                    "G2E-9",
                    format!("agg:{}", n.id),
                    format!("Aggregation {name} ({parts})"),
                    format!("{name} combines object types and nothing refers to it."),
                    vec![
                        (
                            "relationship",
                            "Relationship type",
                            format!(
                                "A relationship {name} between {parts}; its attributes become attributes of the relationship \
                                 (ER: a relationship type is the aggregation of entity types)."
                            ),
                        ),
                        ("entity", "Entity type", format!("An entity {name} with one relationship to each of {parts}.")),
                    ],
                    "relationship",
                )
            } else if let ([], [r]) = (objs.as_slice(), refs.as_slice())
                && r.command == "attribute"
                && self.kind(r.src_id()).is_some_and(|k| OBJECT.contains(&k))
            {
                let owner = self.name(r.src_id());
                self.choose(
                    "G2E-10",
                    format!("agg:{}", n.id),
                    format!("Aggregation {name}"),
                    format!("{name} consists of values only and is used once (by {owner})."),
                    vec![
                        (
                            "entity",
                            "Entity type",
                            format!("An entity {name} with these attributes and a key, linked to {owner}."),
                        ),
                        (
                            "composite",
                            "Composite attribute",
                            format!("A composite attribute {name} of {owner}, with its parts below it."),
                        ),
                    ],
                    "entity",
                )
            } else {
                "entity".to_string()
            };
            self.agg_mode.insert(n.id.clone(), mode);
        }
    }

    fn aggregation(&mut self, n: &Node) {
        let mode = self.agg_mode.get(&n.id).cloned().unwrap_or_default();
        let comps: Vec<&Edge> =
            self.outgoing(&n.id).into_iter().map(|x| x.1).filter(|e| e.command == "component").collect();
        let label_or_id = n.label.clone().filter(|l| !l.is_empty()).unwrap_or_else(|| n.id.clone());
        match mode.as_str() {
            "composite" => {} // handled at its attribute edge
            "relationship" => {
                let id = self.new_id(&slug(&label_or_id));
                let mut rel = Rel::new(id, label_or_id, vec![]);
                rel.pos = Some(self.pos(&n.id));
                for e in &comps {
                    let k = self.kind(e.dst_id());
                    if k.is_some_and(|k| OBJECT.contains(&k) || k == "aggregation") {
                        let role = e
                            .label
                            .clone()
                            .filter(|l| !l.is_empty())
                            .map(|l| vec![("role".to_string(), l)])
                            .unwrap_or_default();
                        rel.sides.push((self.ent(e.dst_id()), "cn".into(), role));
                    } else if k == Some("grouping") {
                        if let Some(member) = self.group_member(e.dst_id()) {
                            rel.sides.push((self.member_entity(&member), "cn".into(), vec![]));
                        }
                    } else if k == Some("representable") {
                        rel.attrs.push(item(&self.attr_name(e.dst_id(), e)));
                    }
                }
                for (_, e) in self.outgoing(&n.id) {
                    if e.command == "attribute" && self.kind(e.dst_id()) == Some("representable") {
                        rel.attrs.push(item(&self.attr_name(e.dst_id(), e)));
                    }
                }
                self.rels.push(rel);
            }
            _ => {
                let eid = self.ent(&n.id);
                for e in comps {
                    let k = self.kind(e.dst_id());
                    let edge_label = e.label.clone().unwrap_or_default();
                    if k == Some("representable") {
                        let nm = self.attr_name(e.dst_id(), e);
                        let key = names_key(&nm, &edge_label);
                        self.attrs_of(&eid).push((AttrItem { key, ..item(&nm) }, edge_label));
                    } else if k.is_some_and(|k| OBJECT.contains(&k) || k == "aggregation") {
                        let label = if edge_label.is_empty() { "has".to_string() } else { edge_label };
                        let id = self.new_id(&slug(&label));
                        let target = self.ent(e.dst_id());
                        self.rels.push(Rel::new(
                            id,
                            label,
                            vec![(eid.clone(), "cn".into(), vec![]), (target, "1".into(), vec![])],
                        ));
                    } else if k == Some("grouping")
                        && let Some(member) = self.group_member(e.dst_id())
                    {
                        let label = if edge_label.is_empty() { self.label(e.dst_id()) } else { edge_label };
                        self.multi_ref(&eid, &member, &label, Some(e.dst_id()));
                    }
                }
            }
        }
    }

    // -- groupings ----------------------------------------------------------------------------

    fn group_member(&mut self, gid: &str) -> Option<String> {
        let comps: Vec<&Edge> =
            self.outgoing(gid).into_iter().map(|x| x.1).filter(|e| e.command == "component").collect();
        let Some(first) = comps.first() else {
            let msg = format!("grouping {} has no member type and was not converted", self.name(gid));
            self.report.push(msg);
            return None;
        };
        if comps.len() > 1 {
            let msg = format!(
                "grouping {} has several components; {} is used",
                self.name(gid),
                self.name(first.dst_id())
            );
            self.report.push(msg);
        }
        Some(first.dst_id().to_string())
    }

    fn member_entity(&mut self, member: &str) -> String {
        if self.kind(member) == Some("representable") { self.repr_entity(member) } else { self.ent(member) }
    }

    fn lone_grouping(&mut self, n: &Node) {
        let Some(member) = self.group_member(&n.id) else { return };
        let eid = self.ent(&n.id);
        let msg = format!(
            "grouping {} is not referenced; it became an entity that contains {} objects",
            self.name(&n.id),
            self.name(&member)
        );
        self.report.push(msg);
        self.multi_ref(&eid, &member, "contains", None);
    }

    // -- attribute edges ----------------------------------------------------------------------

    fn attribute_edge(&mut self, e: &'a Edge) -> Option<Reference<'a>> {
        let (s, t) = (e.src_id(), e.dst_id());
        let multi = e.command == "multiattribute";
        let (sk, tk) = (self.kind(s), self.kind(t));
        let edge_label = e.label.clone().unwrap_or_default();
        if self.points.contains(s) {
            // an attribute of several objects (junction)
            let mut links = self.outgoing(s);
            links.extend(self.incoming(s));
            links.retain(|(_, x)| x.command == "link");
            let objs: BTreeSet<String> = links
                .iter()
                .map(|(_, x)| if x.dst_id() == s { x.src_id() } else { x.dst_id() }.to_string())
                .collect();
            let ents: BTreeSet<String> = objs.iter().filter_map(|o| self.owner(o)).collect();
            if ents.len() >= 2 && tk == Some("representable") {
                self.junction_links.extend(links.iter().map(|(k, _)| *k));
                let a = item(&self.attr_name(t, e));
                self.pair_attrs.push((ents, a));
                return None;
            }
        }
        if sk == Some("aggregation") && self.agg_mode.get(s).map(String::as_str) == Some("relationship") {
            return None; // relationship attributes, see aggregation()
        }
        if sk == Some("representable") {
            let mut owners: Vec<String> = Vec::new();
            for (_, x) in self.incoming(s) {
                if x.command == "attribute"
                    && let Some(o) = self.owner(x.src_id())
                    && !owners.contains(&o)
                {
                    owners.push(o);
                }
            }
            if owners.is_empty() {
                let (ns, nt) = (self.name(s), self.name(t));
                let msg = format!(
                    "the line from {ns} to {nt} was left out: no object type has {ns} as an attribute, so \
                     there is nothing to carry {nt} as a derived attribute"
                );
                self.report.push(msg);
                return None;
            }
            let (nm, _) = strip_derivation(&edge_label);
            let text = if nm.is_empty() { self.name(t) } else { nm };
            for o in owners {
                self.attrs_of(&o).push((AttrItem { derived: true, ..item(&text) }, String::new()));
            }
            let msg =
                format!("{} is derived from {}: it became a derived attribute", self.name(t), self.name(s));
            self.report.push(msg);
            return None;
        }
        let Some(owner) = self.owner(s) else {
            let msg =
                format!("the attribute {} of {} was not converted", plain_text(&edge_label), self.name(s));
            self.report.push(msg);
            return None;
        };
        match tk {
            Some("representable") => {
                let nm = self.attr_name(t, e);
                let (_, derived) = strip_derivation(&edge_label);
                if derived {
                    self.attrs_of(&owner).push((AttrItem { derived: true, ..item(&nm) }, String::new()));
                    let msg = format!("{}.{nm} is derived ({})", self.name(s), plain_text(&edge_label));
                    self.report.push(msg);
                } else if multi {
                    let label = if edge_label.is_empty() { self.label(t) } else { edge_label };
                    self.multi_ref(&owner, t, &label, None);
                } else {
                    self.attrs_of(&owner).push((item(&nm), edge_label));
                }
                None
            }
            Some("grouping") => {
                if let Some(member) = self.group_member(t) {
                    let label = if edge_label.is_empty() { self.label(t) } else { edge_label };
                    self.multi_ref(&owner, &member, &label, Some(t));
                }
                None
            }
            Some("aggregation") if self.agg_mode.get(t).map(String::as_str) == Some("composite") => {
                let parts: Vec<AttrItem> = self
                    .outgoing(t)
                    .into_iter()
                    .filter(|(_, x)| x.command == "component")
                    .map(|(_, x)| item(&self.attr_name(x.dst_id(), x)))
                    .collect();
                let a = AttrItem { parts, ..item(&self.label(t)) };
                self.attrs_of(&owner).push((a, String::new()));
                None
            }
            Some(k) if OBJECT.contains(&k) || k == "aggregation" => {
                Some((s.to_string(), t.to_string(), multi, e))
            }
            _ => {
                let msg = format!(
                    "the attribute {} of {} was not converted",
                    plain_text(&edge_label),
                    self.name(s)
                );
                self.report.push(msg);
                None
            }
        }
    }

    fn ent_label_plain(&self, eid: &str) -> String {
        plain_text(self.ent_label.get(eid).map_or(eid, String::as_str))
    }

    /// A multi-valued reference owner ->> target (representable or object).
    fn multi_ref(&mut self, owner: &str, target: &str, label: &str, via: Option<&str>) {
        let tid = if self.kind(target) == Some("representable") {
            let key = format!("multi:{owner}:{target}:{}", slug(label));
            let (o, t, l) = (self.ent_label_plain(owner), self.name(target), plain_text(label));
            let mode = self.choose(
                "G2E-5",
                key,
                format!("Multi-valued: {o} {l} {t}"),
                format!("{t} is a value type used multi-valued."),
                vec![
                    (
                        "entity",
                        "Entity type + relationship",
                        format!("An entity {t} (key: Name) and an n:n relationship {l}."),
                    ),
                    (
                        "attribute",
                        "Multi-valued attribute",
                        format!("A multi-valued attribute {t} of {o}: {{{t}}}."),
                    ),
                ],
                "entity",
            );
            if mode == "attribute" {
                let a = AttrItem { multivalued: true, ..item(&self.label(target)) };
                self.attrs_of(owner).push((a, String::new()));
                return;
            }
            self.repr_entity(target)
        } else {
            self.ent(target)
        };
        let id = self.new_id(&slug(label));
        let mut rel = Rel::new(
            id,
            label.to_string(),
            vec![(owner.to_string(), "cn".into(), vec![]), (tid, "cn".into(), vec![])],
        );
        rel.pos = via.map(|v| self.pos(v));
        self.rels.push(rel);
    }

    // -- references between object types ------------------------------------------------------

    fn object_references(&mut self, refs: &[Reference<'a>]) {
        let pair_of =
            |r: &Reference<'_>| -> BTreeSet<String> { [r.0.clone(), r.1.clone()].into_iter().collect() };
        let mut done: HashSet<usize> = HashSet::new();
        // the lines back of a pair already answered with "two relationships": no question again
        let mut separate: HashSet<usize> = HashSet::new();
        for (i, r) in refs.iter().enumerate() {
            if done.contains(&i) {
                continue;
            }
            let (s, t, multi, e) = r;
            let pair = pair_of(r);
            let in_pair: Vec<usize> =
                (0..refs.len()).filter(|j| refs.get(*j).is_some_and(|x| pair_of(x) == pair)).collect();
            let back: Vec<usize> = in_pair
                .iter()
                .copied()
                .filter(|j| refs.get(*j).is_some_and(|x| x.0 == *t && x.1 == *s))
                .collect();
            let forth =
                in_pair.iter().filter(|j| refs.get(**j).is_some_and(|x| x.0 == *s && x.1 == *t)).count();
            if s != t
                && !separate.contains(&i)
                && forth == 1
                && let [b] = back.as_slice()
                && let Some(bref) = refs.get(*b)
            {
                let l1 = plain_text(e.label.as_deref().unwrap_or(""));
                let l2 = plain_text(bref.3.label.as_deref().unwrap_or(""));
                let (ns, nt) = (self.name(s), self.name(t));
                let mode = self.choose(
                    "G2E-7",
                    format!("inv:{s}:{t}"),
                    format!("{ns} {l1} / {nt} {l2}"),
                    format!("'{l1}' and '{l2}' connect {ns} and {nt} in opposite directions."),
                    vec![
                        (
                            "merge",
                            "One relationship (inverse attributes)",
                            format!("'{l2}' is the inverse of '{l1}': one relationship {l1} whose cardinalities come from both attributes."),
                        ),
                        ("separate", "Two relationships", format!("'{l1}' and '{l2}' are different facts: two relationships.")),
                    ],
                    "merge",
                );
                if mode == "merge" {
                    done.insert(*b);
                    done.insert(i);
                    let ct = if *multi { "cn" } else { "1" };
                    let cs = if bref.2 { "cn" } else { "1" };
                    let label = e.label.clone().filter(|l| !l.is_empty()).unwrap_or_else(|| "has".into());
                    let id = self.new_id(&slug(&label));
                    let src = self.owner(s).unwrap_or_default();
                    let dst = self.ent(t);
                    self.rels.push(Rel::new(
                        id,
                        label,
                        vec![(src, cs.into(), vec![]), (dst, ct.into(), vec![])],
                    ));
                    continue;
                }
                separate.insert(*b);
            }
            done.insert(i);
            self.reference(s, t, *multi, e);
        }
    }

    /// Attributes of a pair of objects become attributes of their relationship (the years of
    /// employment of *works for*).
    fn attach_pair_attributes(&mut self) {
        for (ents, item) in self.pair_attrs.clone() {
            let index = match self.rels.iter().position(|r| r.entity_set() == ents) {
                Some(i) => i,
                None => {
                    let id = self.new_id("has");
                    let sides = ents.iter().map(|e| (e.clone(), "cn".to_string(), vec![])).collect();
                    self.rels.push(Rel::new(id, "has".into(), sides));
                    self.rels.len() - 1
                }
            };
            let Some(rel) = self.rels.get_mut(index) else { continue };
            rel.attrs.push(item.clone());
            let rel_label = plain_text(&rel.label);
            let names: Vec<String> = ents.iter().map(|e| self.ent_label_plain(e)).collect();
            self.report.push(format!(
                "{} depends on {}: attribute of the relationship {rel_label}",
                item.text,
                names.join(" and ")
            ));
        }
    }

    fn reference(&mut self, s: &str, t: &str, multi: bool, e: &Edge) {
        let src = self.owner(s).unwrap_or_default();
        let dst = self.ent(t);
        let label = e.label.clone().filter(|l| !l.is_empty()).unwrap_or_else(|| "has".into());
        let rid = self.new_id(&slug(&label));
        let card = if multi { "cn" } else { "1" };
        let sides = if src == dst {
            // recursive
            vec![
                (src, "cn".to_string(), vec![("bend".to_string(), "25".to_string())]),
                (dst, card.to_string(), vec![("bend".to_string(), "-25".to_string())]),
            ]
        } else {
            vec![(src, "cn".to_string(), vec![]), (dst, card.to_string(), vec![])]
        };
        self.rels.push(Rel::new(rid, label, sides));
    }

    // -- IS-A ----------------------------------------------------------------------------------

    fn generalisations(&mut self) {
        let mut groups: Vec<((String, String), Vec<&Edge>)> = Vec::new();
        for e in self.g.edges() {
            if matches!(e.command.as_str(), "generalization" | "specialization") {
                let key = (e.dst_id().to_string(), e.command.clone());
                match groups.iter_mut().find(|(k, _)| *k == key) {
                    Some((_, v)) => v.push(e),
                    None => groups.push((key, vec![e])),
                }
                if let Some(l) = e.label.as_deref().filter(|l| !l.is_empty()) {
                    let msg = format!(
                        "{} IS-A {} is marked {} (not expressible in ER)",
                        self.name(e.src_id()),
                        self.name(e.dst_id()),
                        plain_text(l)
                    );
                    self.report.push(msg);
                }
            }
        }
        for ((sup, cmd), edges) in groups {
            if !self.ent_of.contains_key(&sup) && !self.kind(&sup).is_some_and(|k| OBJECT.contains(&k)) {
                continue;
            }
            let subs: Vec<String> = edges.iter().map(|e| self.ent(e.src_id())).collect();
            let iid = self.new_id(&format!("isa_{sup}"));
            let sup_ent = self.ent(&sup);
            let cons = if cmd == "generalization" { "cd" } else { "io" };
            self.isas.push((iid, sup_ent, subs, cons.into()));
        }
    }

    // -- keys ----------------------------------------------------------------------------------

    fn keys(&mut self) {
        let subtypes: HashSet<String> =
            self.isas.iter().flat_map(|(_, _, subs, _)| subs.iter().cloned()).collect();
        for eid in self.entities.clone() {
            if subtypes.contains(&eid) || self.from_repr.contains(&eid) {
                continue;
            }
            let plain = self.ent_label_plain(&eid);
            let add_keys = self.st.add_keys;
            let items = self.attrs_of(&eid);
            if items.iter().any(|(it, _)| it.key) {
                continue;
            }
            let cand = items.iter().position(|(it, lab)| {
                names_key(&it.text, lab) && !(it.derived || it.multivalued || !it.parts.is_empty())
            });
            let msg = if let Some(i) = cand
                && let Some((it, _)) = items.get_mut(i)
            {
                it.key = true;
                format!("{plain}: {} is the key", it.text)
            } else if add_keys {
                let name = format!("{plain} ID");
                items.insert(0, (AttrItem { key: true, ..item(&name) }, String::new()));
                format!("{plain}: key {name} added (GSM has no keys)")
            } else {
                format!("{plain} has no key")
            };
            self.report.push(msg);
        }
    }

    // -- the ER document -------------------------------------------------------------------------

    fn build(&mut self, source: Option<&str>) -> Result<Document, ParseError> {
        let pos = self.layout();
        let mut lines = vec![
            format!("% ER diagram generated by ERDTeX from {}", source.unwrap_or("a GSM diagram")),
            "% (GSM has no keys or cardinalities: check them)".to_string(),
        ];
        let c = |id: &str| {
            let (x, y) = pos.get(id).copied().unwrap_or((0.0, 0.0));
            Coord::numeric(round_half_even(x * 10.0) / 10.0, round_half_even(y * 10.0) / 10.0).to_tex()
        };
        let tree = |node: &str, items: Vec<AttrItem>, anchor: &str| {
            Body::Attributes(AttributeTree {
                node: node.to_string(),
                items,
                opts: Options { items: vec![("anchor".into(), Some(anchor.into()))] },
            })
            .to_tex()
        };
        for eid in &self.entities {
            lines.push(format!(
                "\\entity{{{eid}}}{}{{{}}}",
                c(eid),
                self.ent_label.get(eid).cloned().unwrap_or_default()
            ));
            let items: Vec<AttrItem> =
                self.attrs.get(eid).map(|v| v.iter().map(|(it, _)| it.clone()).collect()).unwrap_or_default();
            if !items.is_empty() {
                lines.push(tree(eid, items, self.anchor.get(eid).copied().unwrap_or("north")));
            }
        }
        for r in &self.rels {
            lines.push(format!("\\relationship{{{}}}{}{{{}}}", r.id, c(&r.id), r.label));
            if !r.attrs.is_empty() {
                lines.push(tree(&r.id, r.attrs.clone(), self.anchor.get(&r.id).copied().unwrap_or("north")));
            }
            for (ent, card, link_opts) in &r.sides {
                let o =
                    Options { items: link_opts.iter().map(|(k, v)| (k.clone(), Some(v.clone()))).collect() };
                lines.push(format!("\\link{}{{{ent}}}{{{}}}{{{card}}}", o.to_tex(), r.id));
            }
        }
        for (iid, sup, subs, cons) in &self.isas {
            lines.push(format!("\\isa{{{iid}}}{}{{{sup}}}{{{cons}}}", c(iid)));
            for s in subs {
                lines.push(format!("\\link{{{s}}}{{{iid}}}"));
            }
        }
        Document::parse(&(lines.join("\n") + "\n"), Kind::Er)
    }

    /// Positions from the GSM drawing, spread so that entity boxes fit.
    fn layout(&mut self) -> HashMap<String, (f64, f64)> {
        let pts: Vec<(f64, f64)> =
            self.entities.iter().map(|e| self.ent_pos.get(e).copied().unwrap_or((0.0, 0.0))).collect();
        let mut s: f64 = 1.3;
        for (i, p) in pts.iter().enumerate() {
            for q in pts.iter().skip(i + 1) {
                let (dx, dy) = ((p.0 - q.0).abs(), (p.1 - q.1).abs());
                let need = (if dx > 1e-6 { 3.6 / dx } else { f64::INFINITY }).min(if dy > 1e-6 {
                    1.7 / dy
                } else {
                    f64::INFINITY
                });
                if need != f64::INFINITY {
                    s = s.max(need);
                }
            }
        }
        let s = s.min(3.0);
        let mut pos: HashMap<String, (f64, f64)> = HashMap::new();
        let mut sizes: Vec<(String, (f64, f64))> = Vec::new();
        for (e, p) in self.entities.iter().zip(&pts) {
            pos.insert(e.clone(), (p.0 * s, p.1 * s));
            sizes.push((e.clone(), (1.5, 0.55)));
        }
        let set_size = |sizes: &mut Vec<(String, (f64, f64))>, id: &str, size: (f64, f64)| match sizes
            .iter_mut()
            .find(|(k, _)| k == id)
        {
            Some(slot) => slot.1 = size,
            None => sizes.push((id.to_string(), size)),
        };
        for r in &self.rels {
            let ps: Vec<(f64, f64)> = r.sides.iter().filter_map(|(e, _, _)| pos.get(e).copied()).collect();
            let distinct: HashSet<&String> = r.sides.iter().map(|(e, _, _)| e).collect();
            let p = if let Some(rp) = r.pos {
                (rp.0 * s, rp.1 * s)
            } else if let (1, Some(first)) = (distinct.len(), ps.first()) {
                (first.0 + 2.6, first.1 + 1.6)
            } else if !ps.is_empty() {
                let n = ps.len() as f64;
                (py_sum(ps.iter().map(|p| p.0)) / n, py_sum(ps.iter().map(|p| p.1)) / n)
            } else {
                (0.0, 0.0)
            };
            pos.insert(r.id.clone(), p);
            set_size(&mut sizes, &r.id, (1.1, 0.8));
        }
        // several relationships between the same entities: side by side
        let mut par: Vec<(BTreeSet<String>, Vec<usize>)> = Vec::new();
        for (i, r) in self.rels.iter().enumerate() {
            if let (None, [a, b]) = (r.pos, r.sides.as_slice())
                && a.0 != b.0
            {
                let key = r.entity_set();
                match par.iter_mut().find(|(k, _)| *k == key) {
                    Some((_, v)) => v.push(i),
                    None => par.push((key, vec![i])),
                }
            }
        }
        for (_, group) in &par {
            if group.len() < 2 {
                continue;
            }
            let Some(first) = group.first().and_then(|i| self.rels.get(*i)) else { continue };
            let ends: Vec<(f64, f64)> =
                first.sides.iter().map(|(e, _, _)| pos.get(e).copied().unwrap_or((0.0, 0.0))).collect();
            let [(ax, ay), (bx, by)] = ends.as_slice() else { continue };
            let d = (bx - ax).hypot(by - ay);
            let d = if d == 0.0 { 1.0 } else { d };
            let (nx, ny) = (-(by - ay) / d, (bx - ax) / d);
            for (k, i) in group.iter().enumerate() {
                let Some(r) = self.rels.get(*i) else { continue };
                let off = (k as f64 - (group.len() as f64 - 1.0) / 2.0) * 1.8;
                if let Some(p) = pos.get_mut(&r.id) {
                    *p = (p.0 + nx * off, p.1 + ny * off);
                }
            }
        }
        for (iid, sup, subs, _) in &self.isas {
            let (sx, sy) = pos.get(sup).copied().unwrap_or((0.0, 0.0));
            let n = subs.len().max(1) as f64;
            let cx = py_sum(subs.iter().map(|x| pos.get(x).map_or(0.0, |p| p.0))) / n;
            let cy = py_sum(subs.iter().map(|x| pos.get(x).map_or(0.0, |p| p.1))) / n;
            pos.insert(iid.clone(), (sx + 0.45 * (cx - sx), sy + 0.45 * (cy - sy)));
            set_size(&mut sizes, iid, (1.1, 0.8));
        }
        let movable: HashSet<String> =
            self.rels.iter().map(|r| r.id.clone()).chain(self.isas.iter().map(|i| i.0.clone())).collect();
        relax(&mut pos, &sizes, &movable, 60);
        // neighbours (for the free side of attribute trees / ellipses)
        let mut nbr: HashMap<String, Vec<String>> = HashMap::new();
        for r in &self.rels {
            for (e, _, _) in &r.sides {
                nbr.entry(e.clone()).or_default().push(r.id.clone());
                nbr.entry(r.id.clone()).or_default().push(e.clone());
            }
        }
        for (iid, sup, subs, _) in &self.isas {
            for x in std::iter::once(sup).chain(subs) {
                nbr.entry(x.clone()).or_default().push(iid.clone());
                nbr.entry(iid.clone()).or_default().push(x.clone());
            }
        }
        let names: Vec<String> =
            self.entities.iter().cloned().chain(self.rels.iter().map(|r| r.id.clone())).collect();
        for n in names {
            let here = pos.get(&n).copied().unwrap_or((0.0, 0.0));
            let dirs: Vec<f64> = nbr
                .get(&n)
                .into_iter()
                .flatten()
                .filter_map(|m| pos.get(m).copied())
                .filter(|p| *p != here)
                .map(|p| py_mod((p.1 - here.1).atan2(p.0 - here.0).to_degrees(), 360.0))
                .collect();
            // the side farthest from every neighbour first (Python's stable sort keeps ties in order)
            let score = |angle: f64| {
                let nearest = if dirs.is_empty() {
                    180.0
                } else {
                    dirs.iter()
                        .map(|d| (angle - d).abs().min(360.0 - (angle - d).abs()))
                        .fold(f64::INFINITY, f64::min)
                };
                -nearest
            };
            let mut ranked = [(90.0, "north"), (270.0, "south"), (0.0, "east"), (180.0, "west")];
            ranked.sort_by(|a, b| score(a.0).total_cmp(&score(b.0)));
            self.anchor.insert(n, ranked.first().map_or("north", |r| r.1));
        }
        pos
    }
}

/// Pushes overlapping boxes apart (only relationships / IS_A move, unless two entities overlap).
fn relax(
    pos: &mut HashMap<String, (f64, f64)>,
    sizes: &[(String, (f64, f64))],
    movable: &HashSet<String>,
    rounds: usize,
) {
    for _ in 0..rounds {
        let mut moved = false;
        for (i, (a, (wa, ha))) in sizes.iter().enumerate() {
            for (b, (wb, hb)) in sizes.iter().skip(i + 1) {
                let ((ax, ay), (bx, by)) =
                    (pos.get(a).copied().unwrap_or((0.0, 0.0)), pos.get(b).copied().unwrap_or((0.0, 0.0)));
                let ox = wa + wb + 0.3 - (ax - bx).abs();
                let oy = ha + hb + 0.25 - (ay - by).abs();
                if ox <= 0.0 || oy <= 0.0 {
                    continue;
                }
                moved = true;
                let (mut da, mut db) = if ox < oy * 2.0 {
                    let d = (ox / 2.0 + 0.05) * if ax >= bx { 1.0 } else { -1.0 };
                    ((d, 0.0), (-d, 0.0))
                } else {
                    let d = (oy / 2.0 + 0.05) * if ay >= by { 1.0 } else { -1.0 };
                    ((0.0, d), (0.0, -d))
                };
                let (am, bm) = (movable.contains(a), movable.contains(b));
                if am && !bm {
                    da = (da.0 * 2.0, da.1 * 2.0);
                    db = (0.0, 0.0);
                } else if bm && !am {
                    da = (0.0, 0.0);
                    db = (db.0 * 2.0, db.1 * 2.0);
                }
                pos.insert(a.clone(), (ax + da.0, ay + da.1));
                pos.insert(b.clone(), (bx + db.0, by + db.1));
            }
        }
        if !moved {
            break;
        }
    }
}

/// The ER diagram for a GSM diagram.  `source`: the GSM file's name, for the comment on top.
pub fn convert(
    gsm: &Document,
    settings: &ConversionSettings,
    source: Option<&str>,
) -> Result<Conversion, ParseError> {
    Converter::new(gsm, settings).run(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert_eq!(slug("lives in"), "lives_in");
        assert_eq!(slug("--"), "r");
        assert!(names_key("Person Id", ""));
        assert!(names_key("orderNo", ""));
        assert!(names_key("x", "has Id"));
        assert!(!names_key("Name", ""));
        assert_eq!(strip_derivation("age [born]"), ("age".into(), true));
        assert_eq!(strip_derivation("a [b"), ("a [b".into(), false));
    }
}
