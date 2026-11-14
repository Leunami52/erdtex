//! Modelling rules.
//!
//! [`can_connect`] implements the *hard* rules: connections the editor refuses to create (the
//! target lights up red and the reason is shown).
//!
//! [`check`] reviews a whole document and reports problems with a severity:
//! error (the diagram violates a rule of the model), warning (incomplete or unusual, normal
//! while drafting) and hint (style suggestions, such as an entity without a key attribute).
//! It reports everything the LaTeX packages warn about, plus checks that need the whole
//! diagram (IS-A cycles, the GSM common-root rule, ...).

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::cardinality::{
    Card, LabelChanges, RELATIONSHIPS, Side, apply_changes, diagram_notation, label_notation, n_form,
    notation_name, parse_card, plan_relationship, relationship_notation, relationship_sides,
};
use crate::document::{Document, Kind};
use crate::generalisation::parse_constraint;
use crate::latex_text::plain_text;
use crate::model::{Body, Edge, Element, ElementKey, Node, endpoint_id, flatten_items};

const ER_RELATIONSHIPS: &[&str] = &["relationship", "partof"];
const GSM_ISA: &[&str] = &["generalization", "specialization"];

/// How bad a problem is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Error,
    Warning,
    Hint,
}

impl Severity {
    /// `error`, `warning` or `hint`.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Hint => "hint",
        }
    }
}

/// What a problem's button does.
#[derive(Clone, Debug, PartialEq)]
pub enum FixAction {
    /// the edge becomes single-lined
    MakeSingle,
    /// the edge becomes double-lined
    MakeDouble,
    /// m -> n, mc -> cn, [0,1] -> c, ...
    NormaliseLabel,
    /// this attribute becomes composite
    MakeComposite(ElementKey),
    /// new labels for a relationship's edges
    ConvertLabels(LabelChanges),
    /// this weak relationship becomes a normal one: no `weak` or `dependency=`, single lines
    MakeNormal,
}

/// A button that solves a problem.
#[derive(Clone, Debug, PartialEq)]
pub struct Fix {
    pub title: String,
    pub action: FixAction,
}

impl Fix {
    fn new(title: impl Into<String>, action: FixAction) -> Option<Fix> {
        Some(Fix { title: title.into(), action })
    }

    /// Changes the document; `element` is the problem's element.
    pub fn apply(&self, doc: &mut Document, element: ElementKey) {
        match &self.action {
            FixAction::MakeSingle => make_single(doc, element),
            FixAction::MakeDouble => make_double(doc, element),
            FixAction::NormaliseLabel => normalise_label(doc, element),
            FixAction::MakeComposite(parent) => {
                if let Some(el) = doc.get_mut(*parent) {
                    if let Body::Node(n) = &mut el.body {
                        n.opts.set("composite", None);
                    }
                    el.touch();
                }
            }
            FixAction::ConvertLabels(changes) => apply_changes(doc, changes),
            FixAction::MakeNormal => make_normal(doc, element),
        }
    }
}

/// Something `check` found.
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub element: ElementKey,
    pub severity: Severity,
    pub message: String,
    pub fix: Option<Fix>,
    /// the rule in the rule books (`ER-1`); layout warnings (problems.rs) have none
    pub rule: &'static str,
}

fn keyed_nodes(doc: &Document) -> impl Iterator<Item = (ElementKey, &Node)> {
    doc.elements.iter().filter_map(|e| e.as_node().map(|n| (e.key, n)))
}

fn keyed_edges(doc: &Document) -> impl Iterator<Item = (ElementKey, &Edge)> {
    doc.elements.iter().filter_map(|e| e.as_edge().map(|x| (e.key, x)))
}

fn node<'a>(doc: &'a Document, ident: &str) -> Option<&'a Node> {
    doc.by_id(ident).and_then(Element::as_node)
}

/// `entity`, `relationship`, `isa`, `point`, ... or `None` if unknown.
pub fn node_kind<'a>(doc: &'a Document, reference: &str) -> Option<&'a str> {
    let e = doc.by_id(endpoint_id(reference))?;
    match &e.body {
        Body::Point(_) => Some("point"),
        Body::Node(n) if n.generalisation.is_some() => Some("isa"),
        Body::Node(n) => Some(&n.command),
        _ => None,
    }
}

/// Is there a path of IS-A edges from `start` to `goal`?
fn isa_reaches(doc: &Document, start: &str, goal: &str) -> bool {
    let mut seen = HashSet::new();
    let mut todo = vec![start.to_string()];
    while let Some(n) = todo.pop() {
        if n == goal {
            return true;
        }
        if !seen.insert(n.clone()) {
            continue;
        }
        todo.extend(
            doc.edges()
                .filter(|e| GSM_ISA.contains(&e.command.as_str()) && e.src_id() == n)
                .map(|e| e.dst_id().to_string()),
        );
    }
    false
}

/// May an edge of type `command` go from `src` to `dst`?  (allowed, the reason if not)
pub fn can_connect(doc: &Document, command: &str, src: &str, dst: &str) -> (bool, String) {
    match refusal(doc, command, src, dst) {
        Some((_, why)) => (false, why),
        None => (true, String::new()),
    }
}

/// Why a connection is refused while drawing, with its rule in the rule books; nothing if it is
/// allowed.
pub fn refusal(doc: &Document, command: &str, src: &str, dst: &str) -> Option<(&'static str, String)> {
    match connect(doc, command, src, dst) {
        (true, _, _) => None,
        (false, rule, why) => Some((rule, why)),
    }
}

fn connect(doc: &Document, command: &str, src: &str, dst: &str) -> (bool, &'static str, String) {
    let no_by = |rule: &'static str, why: &str| (false, rule, why.to_string());
    let (Some(a), Some(b)) = (node_kind(doc, src), node_kind(doc, dst)) else {
        return no_by("DRAW-2", "edges must connect two elements");
    };
    if endpoint_id(src) == endpoint_id(dst) {
        return no_by("DRAW-3", "an element cannot be connected to itself");
    }
    if a == "point" || b == "point" {
        return (true, "", String::new());
    }
    if doc.kind == Kind::Er {
        let relationish = |k: &str| ER_RELATIONSHIPS.contains(&k) || k == "isa" || k == "canbe";
        if a == "entity" && b == "entity" {
            return no_by("DRAW-1", "two entities need a relationship in between");
        }
        if relationish(a) && relationish(b) {
            return no_by("DRAW-4", "relationships cannot be linked to each other");
        }
        if a == "attribute" || b == "attribute" {
            let other = if a == "attribute" { b } else { a };
            if !matches!(other, "entity" | "relationship" | "attribute") {
                return no_by("DRAW-5", "attributes belong to entities or relationships");
            }
            return (true, "", String::new());
        }
        if a == "isa" || b == "isa" {
            // the other end is an entity here: relationships, PART_OF, generalisations, Chen attributes and
            // junction points are decided above (DRAW-4, DRAW-5)
            let (isa_ref, other_ref) = if a == "isa" { (src, dst) } else { (dst, src) };
            if let Some(g) = node(doc, endpoint_id(isa_ref)).and_then(|n| n.generalisation.as_ref())
                && endpoint_id(&g.supertype) == endpoint_id(other_ref)
            {
                return no_by("DRAW-6", "the supertype cannot be its own subtype");
            }
        }
        return (true, "", String::new());
    }
    if GSM_ISA.contains(&command) {
        if a != "subtype" {
            return no_by("DRAW-7", "IS-A edges start at a subtype (circle)");
        }
        if b != "subtype" && b != "abstract" {
            return no_by("DRAW-8", "IS-A edges end at a subtype or an abstract type");
        }
        if isa_reaches(doc, endpoint_id(dst), endpoint_id(src)) {
            return no_by("DRAW-9", "this IS-A edge would create a cycle");
        }
        return (true, "", String::new());
    }
    if command == "component" {
        if a != "aggregation" && a != "grouping" {
            return no_by("DRAW-10", "component edges start at an aggregation or grouping");
        }
        if a == "grouping" && doc.edges().any(|e| e.command == "component" && e.src_id() == endpoint_id(src))
        {
            return no_by("DRAW-11", "a grouping has exactly one component");
        }
    }
    (true, "", String::new())
}

/// Is there such an edge already?
pub fn edge_exists(doc: &Document, command: &str, src: &str, dst: &str) -> bool {
    let (s, d) = (endpoint_id(src), endpoint_id(dst));
    doc.edges().any(|e| e.command == command && e.src_id() == s && e.dst_id() == d)
}

// ---------------------------------------------------------------------------------------------
//  double-lined links (ER): weak relationship <-> weak entity
// ---------------------------------------------------------------------------------------------

fn flag(n: &Node, key: &str) -> bool {
    n.opts.is_on(key)
}

/// Weak entities and weak / E / Id relationships.
pub fn double_bordered(n: Option<&Node>) -> bool {
    let Some(n) = n.filter(|n| n.generalisation.is_none()) else { return false };
    match n.command.as_str() {
        "entity" => flag(n, "weak"),
        "relationship" => flag(n, "weak") || n.opts.get("dependency").is_some_and(|d| !d.is_empty()),
        _ => false,
    }
}

/// A link between a weak relationship and a weak entity.
pub fn may_be_double(doc: &Document, e: &Edge) -> bool {
    if doc.kind != Kind::Er || e.command != "link" {
        return false;
    }
    let (a, b) = (node(doc, e.src_id()), node(doc, e.dst_id()));
    if !(double_bordered(a) && double_bordered(b)) {
        return false;
    }
    let (Some(a), Some(b)) = (a, b) else { return false };
    let pair = (a.command.as_str(), b.command.as_str());
    pair == ("entity", "relationship") || pair == ("relationship", "entity")
}

/// The other end of an edge.
pub fn other_end<'a>(e: &'a Edge, ident: &str) -> &'a str {
    if e.src_id() == ident { e.dst_id() } else { e.src_id() }
}

/// The links of weak relationship `rel` to weak entities.
pub fn weak_links<'a>(doc: &'a Document, rel: &str) -> Vec<(ElementKey, &'a Edge)> {
    keyed_edges(doc)
        .filter(|(_, e)| (e.src_id() == rel || e.dst_id() == rel) && may_be_double(doc, e))
        .collect()
}

fn weak_ends(doc: &Document, rel: &str) -> BTreeSet<String> {
    weak_links(doc, rel).iter().map(|(_, e)| other_end(e, rel).to_string()).collect()
}

/// The link leading to the entity that depends on the other side.  With two weak entities
/// only the double line can say which one that is; `None` if it doesn't.
pub fn dependent_link<'a>(doc: &'a Document, rel: &str) -> Option<(ElementKey, &'a Edge)> {
    let links = weak_links(doc, rel);
    if weak_ends(doc, rel).len() < 2 {
        return links.first().copied();
    }
    let doubled: Vec<_> = links.into_iter().filter(|(_, e)| is_double(e)).collect();
    match doubled.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// The double line goes to `entity`; `None`: not decided yet.
pub fn set_dependent(doc: &mut Document, rel: &str, entity: Option<&str>) {
    let links: Vec<(ElementKey, bool)> =
        weak_links(doc, rel).iter().map(|(k, e)| (*k, Some(other_end(e, rel)) == entity)).collect();
    for (key, double) in links {
        if double { make_double(doc, key) } else { make_single(doc, key) }
    }
}

/// The link to the only weak entity of a weak relationship.  With two of them, the owner's
/// link has to stay single, so nothing says that a given one should be double.
pub fn should_be_double(doc: &Document, e: &Edge) -> bool {
    if !may_be_double(doc, e) {
        return false;
    }
    let src_is_rel = node(doc, e.src_id()).is_some_and(|a| a.command == "relationship");
    weak_ends(doc, if src_is_rel { e.src_id() } else { e.dst_id() }).len() < 2
}

/// Drawn with a double line.
pub fn is_double(e: &Edge) -> bool {
    e.opts.contains("double") && e.opts.get("double") != Some("false")
}

fn edit_edge(doc: &mut Document, key: ElementKey, f: impl FnOnce(&mut Edge)) {
    if let Some(el) = doc.get_mut(key) {
        if let Body::Edge(e) = &mut el.body {
            f(e);
        }
        el.touch();
    }
}

/// The edge becomes double-lined.
pub fn make_double(doc: &mut Document, key: ElementKey) {
    edit_edge(doc, key, |e| e.opts.set("double", None));
}

/// The edge becomes single-lined.
pub fn make_single(doc: &mut Document, key: ElementKey) {
    edit_edge(doc, key, |e| {
        e.opts.remove("double");
        e.opts.remove("double distance");
    });
}

/// A weak relationship becomes a normal one: `weak` and `dependency=` go, and its links become
/// single-lined.
pub fn make_normal(doc: &mut Document, key: ElementKey) {
    let Some(id) = doc.get(key).and_then(Element::as_node).map(|n| n.id.clone()) else { return };
    if let Some(el) = doc.get_mut(key) {
        if let Body::Node(n) = &mut el.body {
            n.opts.remove("weak");
            n.opts.remove("dependency");
        }
        el.touch();
    }
    let mut links: Vec<ElementKey> = relationship_sides(doc, &id).iter().map(|s| s.edge).collect();
    links.extend(keyed_edges(doc).filter(|(_, e)| e.src_id() == id || e.dst_id() == id).map(|(k, _)| k));
    for k in links {
        if doc.get(k).and_then(Element::as_edge).is_some_and(|e| e.opts.contains("double")) {
            make_single(doc, k);
        }
    }
}

/// m -> n, mc -> cn, [0,1] -> c, ...
pub fn normalise_label(doc: &mut Document, key: ElementKey) {
    let label = doc.get(key).and_then(Element::as_edge).and_then(|e| e.label.clone());
    if let Some(new) = n_form(&parse_card(label.as_deref())) {
        edit_edge(doc, key, |e| e.label = Some(new.to_string()));
    }
}

// ---------------------------------------------------------------------------------------------
//  checking a whole document
// ---------------------------------------------------------------------------------------------

struct Problems(Vec<Problem>);

/// The rule of a check both kinds of diagram have, as each kind's book numbers it.
fn by_kind(doc: &Document, er: &'static str, gsm: &'static str) -> &'static str {
    if doc.kind == Kind::Er { er } else { gsm }
}

impl Problems {
    /// A problem: the rule it breaks in the rule books, and what it says here.
    fn rule(
        &mut self,
        rule: &'static str,
        element: ElementKey,
        severity: Severity,
        message: impl Into<String>,
        fix: Option<Fix>,
    ) {
        self.0.push(Problem { element, severity, message: message.into(), fix, rule });
    }
}

/// Everything wrong or unusual in a diagram: errors first, then warnings and hints, each in
/// the order of the file.
pub fn check(doc: &Document) -> Vec<Problem> {
    let mut out = Problems(Vec::new());
    // ids and references (both models)
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for e in &doc.elements {
        if let Some(id) = e.id() {
            *counts.entry(id).or_default() += 1;
        }
    }
    let mut seen = HashSet::new();
    for e in &doc.elements {
        if let Some(id) = e.id() {
            let n = counts.get(id).copied().unwrap_or(0);
            if n > 1 && seen.insert(id) {
                out.rule(
                    by_kind(doc, "ER-2", "GSM-2"),
                    e.key,
                    Severity::Error,
                    format!("the id '{id}' is used {n} times"),
                    None,
                );
            }
        }
    }
    let ids = doc.ids();
    for (key, e) in keyed_edges(doc) {
        let missing: Vec<&str> = [e.src_id(), e.dst_id()].into_iter().filter(|r| !ids.contains(r)).collect();
        if !missing.is_empty() {
            out.rule(
                by_kind(doc, "ER-3", "GSM-3"),
                key,
                Severity::Error,
                format!("connects to an unknown element: {}", missing.join(", ")),
                None,
            );
        }
    }
    for e in &doc.elements {
        if let Body::Attributes(t) = &e.body
            && !ids.contains(t.node.as_str())
        {
            out.rule(
                "ER-4",
                e.key,
                Severity::Error,
                format!("belongs to the unknown element '{}'", t.node),
                None,
            );
        }
    }
    if doc.kind == Kind::Er {
        check_er(doc, &mut out);
    } else {
        check_gsm(doc, &mut out);
    }
    let index: HashMap<ElementKey, usize> =
        doc.elements.iter().enumerate().map(|(k, e)| (e.key, k)).collect();
    let mut problems = out.0;
    problems.sort_by_key(|p| (p.severity, index.get(&p.element).copied().unwrap_or(0)));
    problems
}

/// As the user sees it in the diagram, not its id.
fn name(doc: &Document, ident: &str) -> String {
    match node(doc, ident).and_then(|n| n.label.as_deref()) {
        Some(label) if !label.is_empty() => plain_text(label),
        _ => ident.to_string(),
    }
}

fn check_er(doc: &Document, out: &mut Problems) {
    let mut kind: HashMap<&str, &str> = HashMap::new();
    for n in doc.nodes() {
        kind.insert(&n.id, if n.generalisation.is_some() { "isa" } else { &n.command });
    }
    for p in doc.points() {
        kind.insert(&p.id, "point");
    }
    let mut links: HashMap<&str, usize> = HashMap::new();
    let mut weak_linked: HashSet<&str> = HashSet::new();
    for (key, e) in keyed_edges(doc) {
        let (Some(a), Some(b)) = (kind.get(e.src_id()).copied(), kind.get(e.dst_id()).copied()) else {
            continue;
        };
        *links.entry(e.src_id()).or_default() += 1;
        *links.entry(e.dst_id()).or_default() += 1;
        if a == "entity" && b == "entity" {
            out.rule(
                "ER-5",
                key,
                Severity::Error,
                "two entities are linked directly; a relationship is needed in between",
                None,
            );
        }
        if ER_RELATIONSHIPS.contains(&a) && ER_RELATIONSHIPS.contains(&b) {
            out.rule("ER-6", key, Severity::Error, "two relationships are linked directly", None);
        }
        for (x, y) in [(a, b), (b, a)] {
            if (x == "isa" || x == "canbe") && y != "entity" && y != "point" {
                out.rule(
                    "ER-7",
                    key,
                    Severity::Error,
                    "only entities can be linked to a generalisation",
                    None,
                );
            }
        }
        let label = e.label.as_deref().filter(|l| !l.is_empty());
        if let Some(l) = label
            && a != "entity"
            && b != "entity"
        {
            out.rule(
                "ER-12",
                key,
                Severity::Warning,
                format!("the cardinality '{l}' is on an edge without an entity"),
                None,
            );
        }
        if is_double(e) && !may_be_double(doc, e) {
            out.rule(
                "ER-22",
                key,
                Severity::Error,
                "only a link between a weak relationship and a weak entity can be double-lined",
                Fix::new("Make single-lined", FixAction::MakeSingle),
            );
        } else if should_be_double(doc, e) && !is_double(e) {
            out.rule(
                "ER-21",
                key,
                Severity::Warning,
                "a link between a weak relationship and a weak entity should be double-lined",
                Fix::new("Make double-lined", FixAction::MakeDouble),
            );
        }
        for (ent, rel) in [(e.src_id(), e.dst_id()), (e.dst_id(), e.src_id())] {
            let (n, r) = (node(doc, ent), node(doc, rel));
            let weak_entity = n.is_some_and(|n| n.command == "entity" && n.opts.is_on("weak"));
            let weak_rel = r.is_some_and(|r| {
                r.command == "relationship"
                    && (r.opts.is_on("weak") || r.opts.get("dependency").is_some_and(|d| !d.is_empty()))
            });
            if weak_entity && weak_rel {
                // the owner in a relationship between two weak entities doesn't depend on it
                match dependent_link(doc, rel) {
                    Some((_, link)) if other_end(link, rel) != ent => {}
                    _ => {
                        weak_linked.insert(ent);
                    }
                }
            }
        }
    }
    check_missing_cardinalities(doc, out);
    check_entities_through_points(doc, out);
    check_cardinalities(doc, out);
    check_generalisation_types(doc, out);
    check_roles(doc, out);
    check_weak_owners(doc, out);
    check_recursive_weak(doc, out);
    check_attribute_owners(doc, out);
    check_candidate_keys(doc, out);
    let count = |id: &str| links.get(id).copied().unwrap_or(0);
    for (key, n) in keyed_nodes(doc) {
        if let Some(g) = &n.generalisation {
            match kind.get(endpoint_id(&g.supertype)) {
                None => out.rule(
                    "ER-29",
                    key,
                    Severity::Error,
                    format!("the supertype '{}' does not exist", g.supertype),
                    None,
                ),
                Some(&k) if k != "entity" => out.rule(
                    "ER-30",
                    key,
                    Severity::Error,
                    format!("the supertype '{}' is not an entity", g.supertype),
                    None,
                ),
                _ => {}
            }
            if count(&n.id) < 1 {
                out.rule(
                    "ER-31",
                    key,
                    Severity::Warning,
                    "this generalisation has no subtypes linked to it",
                    None,
                );
            }
            if doc.attribute_trees(Some(&n.id)).next().is_some() {
                out.rule("ER-32", key, Severity::Warning, "a generalisation cannot have attributes", None);
            }
        } else if ER_RELATIONSHIPS.contains(&n.command.as_str()) {
            if count(&n.id) < 2 {
                out.rule(
                    "ER-8",
                    key,
                    Severity::Warning,
                    "this relationship connects fewer than two elements",
                    None,
                );
            }
            let ends = weak_ends(doc, &n.id);
            if n.command == "relationship"
                && double_bordered(Some(n))
                && ends.len() > 1
                && dependent_link(doc, &n.id).is_none()
            {
                let mut names: Vec<String> = ends.iter().map(|x| name(doc, x)).collect();
                names.sort();
                out.rule(
                    "ER-23",
                    key,
                    Severity::Warning,
                    format!(
                        "this relationship joins two weak entities, {}: exactly one link has to be double-lined, \
                         the one to the entity that depends on the other. Choose it under Dependent",
                        names.join(" and ")
                    ),
                    None,
                );
            }
            if n.opts.get("dependency").is_some_and(|d| !d.is_empty()) || n.opts.is_on("weak") {
                let partners: Vec<Option<&Node>> = doc
                    .edges()
                    .filter(|e| e.src_id() == n.id || e.dst_id() == n.id)
                    .map(|e| node(doc, if e.src_id() == n.id { e.dst_id() } else { e.src_id() }))
                    .collect();
                if !partners.is_empty() && !partners.iter().any(|p| p.is_some_and(|p| p.opts.is_on("weak"))) {
                    out.rule(
                        "ER-20",
                        key,
                        Severity::Warning,
                        "a weak/dependency relationship should connect a weak entity",
                        None,
                    );
                }
            }
        } else if n.command == "entity" {
            if n.opts.is_on("weak") && !weak_linked.contains(n.id.as_str()) {
                out.rule(
                    "ER-19",
                    key,
                    Severity::Warning,
                    "this weak entity is not linked to a weak (E/Id) relationship",
                    None,
                );
            }
        } else if n.command == "attribute" {
            let owner = n.opts.get("of").filter(|o| !o.is_empty());
            if let Some(o) = owner
                && !kind.contains_key(o)
            {
                out.rule("ER-4", key, Severity::Error, format!("belongs to the unknown element '{o}'"), None);
            }
            if owner.is_none() && count(&n.id) == 0 {
                out.rule("ER-9", key, Severity::Warning, "this attribute is not connected to anything", None);
            }
        }
    }
    // entities without key attributes (subtypes of a generalisation inherit it)
    let isa_ids: HashSet<&str> =
        doc.nodes().filter(|n| n.generalisation.is_some()).map(|n| n.id.as_str()).collect();
    let mut subtypes: HashSet<&str> = HashSet::new();
    for e in doc.edges() {
        if isa_ids.contains(e.dst_id()) {
            subtypes.insert(e.src_id());
        }
        if isa_ids.contains(e.src_id()) {
            subtypes.insert(e.dst_id());
        }
    }
    let mut keyed: HashSet<&str> = HashSet::new();
    for t in doc.attribute_trees(None) {
        if t.items.iter().any(|i| i.key) {
            keyed.insert(&t.node);
        }
    }
    for n in doc.nodes() {
        if n.command == "attribute"
            && n.opts.contains("key")
            && let Some(of) = n.opts.get("of").filter(|o| !o.is_empty())
        {
            keyed.insert(of);
        }
    }
    for (key, n) in keyed_nodes(doc) {
        if n.command == "entity"
            && n.generalisation.is_none()
            && !keyed.contains(n.id.as_str())
            && !subtypes.contains(n.id.as_str())
            && !identified_by_subtypes(doc, &n.id, &keyed, &mut Vec::new())
        {
            out.rule("ER-1", key, Severity::Hint, "this entity has no key attribute", None);
        }
    }
}

/// Whether a supertype without a key of its own is identified through its subtypes: a complete IS_A
/// below it whose subtypes all have a key (see `has_key`). Every entity of a complete generalisation
/// is in one of its subtypes, so their keys identify it.
fn identified_by_subtypes<'a>(
    doc: &'a Document,
    id: &'a str,
    keyed: &HashSet<&str>,
    seen: &mut Vec<&'a str>,
) -> bool {
    if seen.contains(&id) {
        return false; // already being looked at: a cycle, or the way back to where the search began
    }
    seen.push(id);
    let found = doc.nodes().any(|g| {
        let Some(isa) = g.generalisation.as_ref().filter(|x| g.command == "isa" && x.supertype == id) else {
            return false;
        };
        if parse_constraint(isa.constraint.as_deref()).0 != Some(true) {
            return false; // incomplete, or not said: some entities are only in the supertype
        }
        let subs = subtypes_of(doc, &g.id, id);
        !subs.is_empty() && subs.iter().all(|s| has_key(doc, s, keyed, seen))
    });
    seen.pop();
    found
}

/// Whether an entity has a key: its own, one it inherits from any of its supertypes (a subtype of
/// several, such as a consumer that is a person and a customer, takes the one there is), or through a
/// complete IS_A below it.
fn has_key<'a>(doc: &'a Document, id: &'a str, keyed: &HashSet<&str>, seen: &mut Vec<&'a str>) -> bool {
    if keyed.contains(id) {
        return true;
    }
    if seen.contains(&id) {
        return false;
    }
    seen.push(id);
    let inherited = doc.nodes().any(|g| {
        g.generalisation.as_ref().is_some_and(|sup| {
            subtypes_of(doc, &g.id, &sup.supertype).contains(&id) && has_key(doc, &sup.supertype, keyed, seen)
        })
    });
    seen.pop();
    inherited || identified_by_subtypes(doc, id, keyed, seen)
}

/// The subtypes linked to a generalisation (IS_A or CAN_BE) `g`, without its supertype.
fn subtypes_of<'a>(doc: &'a Document, g: &str, supertype: &str) -> Vec<&'a str> {
    doc.edges()
        .filter_map(|e| {
            if e.dst_id() == g {
                Some(e.src_id())
            } else if e.src_id() == g {
                Some(e.dst_id())
            } else {
                None
            }
        })
        .filter(|s| *s != supertype && node(doc, s).is_some_and(|n| n.command == "entity"))
        .collect()
}

/// What a kind of weak relationship (`weak`, `E`, `Id`) means, with the real names.
pub fn explain_dependency(doc: &Document, relationship: &str, kind: &str) -> String {
    let sides = relationship_sides(doc, relationship);
    if let [a, b] = sides.as_slice()
        && a.entity == b.entity
    {
        // an entity type depending on itself (chapters in chapters): see check_recursive_weak
        let ent = name(doc, &a.entity);
        let above = recursive_owner(&sides).map_or(format!("{ent} above it"), |(_, o)| above(doc, o, false));
        let impossible = format!(
            "not possible here: the key of each {ent} would contain the key of the one above it, and that one's, \
             without end. Choose E, or make it a normal relationship."
        );
        return match kind {
            "E" => format!(
                "Existence dependency only. Each {ent} below another can only exist with the {above}: deleting a \
                 {ent} deletes those below it. Its own key is unique without it, so the key of the {above} becomes \
                 a foreign key, empty for the topmost {ent}, which depends on none."
            ),
            "Id" => format!("Identification dependency, {impossible}"),
            "weak" => {
                format!("Not specified, which counts as an identification dependency (Id), {impossible}")
            }
            _ => String::new(),
        };
    }
    let entities: Vec<&Node> =
        relationship_sides(doc, relationship).iter().filter_map(|s| node(doc, &s.entity)).collect();
    let weak: Vec<&Node> = entities.iter().copied().filter(|n| double_bordered(Some(n))).collect();
    let weak_ids: BTreeSet<&str> = weak.iter().map(|n| n.id.as_str()).collect();
    let link = dependent_link(doc, relationship);
    let undecided = weak_ids.len() > 1 && link.is_none();
    let dependent: Option<&Node> = if undecided {
        None
    } else if weak_ids.len() > 1 {
        link.and_then(|(_, l)| node(doc, other_end(l, relationship)))
    } else {
        weak.first().copied()
    };
    let owner = dependent.and_then(|d| entities.iter().copied().find(|n| n.id != d.id));
    let exists = format!(
        "Each {} can only exist with its {}",
        dependent.map_or("weak entity".to_string(), |d| name(doc, &d.id)),
        owner.map_or("owner".to_string(), |o| name(doc, &o.id))
    );
    let owner_key =
        owner.map_or("the key of the owner".to_string(), |o| format!("the key of {}", name(doc, &o.id)));
    let mut text = match kind {
        "E" => format!(
            "Existence dependency only. {exists}, but its own key is unique everywhere. So the key is underlined \
             solidly, and {owner_key} becomes a foreign key."
        ),
        "Id" => format!(
            "Identification dependency, which includes existence dependency. {exists}, and its key is only unique \
             together with {owner_key}. So the key is underlined with dashes, and {owner_key} becomes part of the \
             primary key."
        ),
        "weak" => format!(
            "Not specified. {exists}. The key is underlined with dashes, and the relational schema treats this as \
             an identification dependency (Id). E or Id settles it."
        ),
        _ => return String::new(),
    };
    if undecided {
        let names: BTreeSet<String> = weak.iter().map(|n| name(doc, &n.id)).collect();
        let names: Vec<String> = names.into_iter().collect();
        text += &format!(
            " Both {} are weak, so choose below which one depends on the other.",
            names.join(" and ")
        );
    }
    text
}

/// Roles are mandatory where an entity type takes part in a relationship more than once (ER-17).
fn check_roles(doc: &Document, out: &mut Problems) {
    for (key, n) in keyed_nodes(doc) {
        if !RELATIONSHIPS.contains(&n.command.as_str()) {
            continue;
        }
        let sides = relationship_sides(doc, &n.id);
        let mut counts: Vec<(&str, usize)> = Vec::new();
        for s in &sides {
            match counts.iter_mut().find(|(e, _)| *e == s.entity) {
                Some((_, c)) => *c += 1,
                None => counts.push((&s.entity, 1)),
            }
        }
        for (entity, count) in counts {
            if count < 2 {
                continue;
            }
            let roles: Vec<&str> =
                sides.iter().filter(|s| s.entity == entity).map(|s| crate::text::strip(&s.role)).collect();
            let distinct: HashSet<&&str> = roles.iter().collect();
            if roles.iter().any(|r| r.is_empty()) {
                out.rule(
                    "ER-17",
                    key,
                    Severity::Warning,
                    format!(
                        "'{}' takes part {count} times here, so each of these edges needs a role",
                        name(doc, entity)
                    ),
                    None,
                );
            } else if distinct.len() < roles.len() {
                out.rule(
                    "ER-18",
                    key,
                    Severity::Warning,
                    format!(
                        "'{}' takes part {count} times here with the same role; the roles have to tell the edges apart",
                        name(doc, entity)
                    ),
                    None,
                );
            }
        }
    }
}

/// A weak entity depends on exactly one owner, so more than that is impossible.
fn check_weak_owners(doc: &Document, out: &mut Problems) {
    let weak_entities: HashSet<&str> = doc
        .nodes()
        .filter(|n| n.command == "entity" && double_bordered(Some(n)))
        .map(|n| n.id.as_str())
        .collect();
    for (key, n) in keyed_nodes(doc) {
        if n.command != "relationship" || !double_bordered(Some(n)) {
            continue;
        }
        let sides = relationship_sides(doc, &n.id);
        let [a, b] = sides.as_slice() else { continue };
        if a.entity == b.entity {
            continue; // an entity type depending on itself: check_recursive_weak
        }
        let chosen = dependent_link(doc, &n.id).map(|(_, l)| other_end(l, &n.id).to_string());
        for (dependent, owner) in [(a, b), (b, a)] {
            if weak_entities.contains(dependent.entity.as_str())
                && (!weak_entities.contains(owner.entity.as_str())
                    || Some(&dependent.entity) == chosen.as_ref())
            {
                // n-notation counts at the far end, (min,max) at its own
                let many = (dependent.card.known && dependent.card.here && !dependent.card.single())
                    || (owner.card.known && !owner.card.here && !owner.card.single());
                if many {
                    out.rule(
                        "ER-24",
                        key,
                        Severity::Warning,
                        format!(
                            "'{}' depends on exactly one '{}', so its cardinality here can't allow more than one",
                            name(doc, &dependent.entity),
                            name(doc, &owner.entity)
                        ),
                        None,
                    );
                }
            }
        }
    }
}

/// How many owners a dependent has, as its cardinality says: in min-max at the dependent's end, in
/// n-notation at the owner's.
fn owners_card<'a>(dependent: &'a Side, owner: &'a Side) -> Option<&'a Card> {
    if dependent.card.known && dependent.card.here {
        Some(&dependent.card)
    } else if owner.card.known && !owner.card.here {
        Some(&owner.card)
    } else {
        None
    }
}

/// Of a weak relationship between an entity type and itself: the end that depends, and the end
/// it depends on (the one each dependent has at most one of).
pub fn recursive_owner(sides: &[Side]) -> Option<(&Side, &Side)> {
    let [a, b] = sides else { return None };
    if a.entity != b.entity {
        return None;
    }
    [(a, b), (b, a)].into_iter().find(|(d, o)| owners_card(d, o).is_none_or(Card::single))
}

/// The owner of a recursive weak relationship, for messages: its role, or "X above it" (quoted
/// for the problems, plain for the panel's explanations).
fn above(doc: &Document, owner: &Side, quoted: bool) -> String {
    let role = crate::text::strip(&owner.role);
    let q = if quoted { "'" } else { "" };
    if role.is_empty() {
        format!("{q}{}{q} above it", name(doc, &owner.entity))
    } else {
        format!("{q}{role}{q}")
    }
}

/// A weak relationship between an entity type and itself (chapters in chapters).  An
/// identification dependency is impossible there, since the key would contain itself; an
/// existence dependency is accepted, with a hint that the topmost ones depend on none.
fn check_recursive_weak(doc: &Document, out: &mut Problems) {
    for (key, n) in keyed_nodes(doc) {
        if n.command != "relationship" || !double_bordered(Some(n)) {
            continue;
        }
        let sides = relationship_sides(doc, &n.id);
        let [a, b] = sides.as_slice() else { continue };
        if a.entity != b.entity {
            continue;
        }
        let ent = name(doc, &a.entity);
        if n.opts.get("dependency").map(crate::text::strip) != Some("E") {
            out.rule(
                "ER-25",
                key,
                Severity::Warning,
                format!(
                    "'{ent}' depends on another '{ent}' here, which can't be an identification dependency: the key of \
                     each '{ent}' would contain the key of the one above it, and that one's, without end. Choose E \
                     (existence dependency), or make it a normal relationship"
                ),
                Fix::new("Make it a normal relationship", FixAction::MakeNormal),
            );
            continue;
        }
        let Some((dependent, owner)) = recursive_owner(&sides) else {
            out.rule(
                "ER-26",
                key,
                Severity::Warning,
                format!(
                    "each '{ent}' can only depend on one other '{ent}', so one of the two ends has to allow at most one"
                ),
                None,
            );
            continue;
        };
        let above = above(doc, owner, true);
        if owners_card(dependent, owner).is_some_and(|c| !c.optional()) {
            out.rule(
                "ER-27",
                key,
                Severity::Warning,
                format!(
                    "every '{ent}' needs a {above} here, so there can't be a topmost '{ent}'; allow none at that end \
                     (c, or (0,1))"
                ),
                None,
            );
        } else {
            out.rule(
                "ER-28",
                key,
                Severity::Hint,
                format!(
                    "the topmost '{ent}' has no {above}, so it can't depend on one: the existence dependency only \
                     applies to those below another '{ent}' (deleting a '{ent}' deletes those below it)"
                ),
                None,
            );
        }
    }
}

/// Attributes can only hang on attributes where those are composite.
fn check_attribute_owners(doc: &Document, out: &mut Problems) {
    // as Python's {n.id: n}: the order of first appearance, the last node of an id
    let mut order: Vec<&str> = Vec::new();
    let mut attributes: HashMap<&str, (ElementKey, &Node)> = HashMap::new();
    for (key, n) in keyed_nodes(doc).filter(|(_, n)| n.command == "attribute") {
        if attributes.insert(&n.id, (key, n)).is_none() {
            order.push(&n.id);
        }
    }
    // only the attribute the other belongs to has to be composite (the part may be simple); `composite`
    // takes no value in the package, so `composite=false` is composite too, as in the drawing
    let complain = |out: &mut Problems, child: (ElementKey, &Node), parent: (ElementKey, &Node)| {
        if !parent.1.opts.contains("composite") {
            let pname = name(doc, &parent.1.id);
            out.rule(
                "ER-10",
                child.0,
                Severity::Warning,
                format!("an attribute can only belong to another one if that is composite: '{pname}' isn't"),
                Fix::new(format!("Make '{pname}' composite"), FixAction::MakeComposite(parent.0)),
            );
        }
    };
    let mut done: HashSet<(&str, &str)> = HashSet::new();
    for id in &order {
        let Some(&child) = attributes.get(id) else { continue };
        let of = endpoint_id(child.1.opts.get("of").unwrap_or(""));
        if let Some(&parent) = attributes.get(of) {
            complain(out, child, parent);
            done.insert((&child.1.id, of));
        }
    }
    for e in doc.edges() {
        if let (Some(&src), Some(&dst)) = (attributes.get(e.src_id()), attributes.get(e.dst_id()))
            && !done.contains(&(e.src_id(), e.dst_id()))
            && !done.contains(&(e.dst_id(), e.src_id()))
        {
            complain(out, dst, src);
        }
    }
}

fn check_candidate_keys(doc: &Document, out: &mut Problems) {
    for el in &doc.elements {
        if let Body::Attributes(t) = &el.body {
            for (it, _) in flatten_items(&t.items) {
                if it.unique && (it.derived || it.multivalued) {
                    let what = if it.derived { "derived" } else { "multi-valued" };
                    out.rule(
                        "ER-34",
                        el.key,
                        Severity::Warning,
                        format!("'{}' is {what}, so it can't be a candidate key", it.text),
                        None,
                    );
                }
            }
        }
    }
    for (key, n) in keyed_nodes(doc) {
        if n.command == "attribute"
            && n.opts.contains("unique")
            && (n.opts.contains("derived") || n.opts.contains("multivalued"))
        {
            out.rule(
                "ER-34",
                key,
                Severity::Warning,
                "a derived or multi-valued attribute can't be a candidate key",
                None,
            );
        }
    }
}

fn check_generalisation_types(doc: &Document, out: &mut Problems) {
    for (key, n) in keyed_nodes(doc) {
        if let Some(g) = &n.generalisation {
            let (complete, disjoint) = parse_constraint(g.constraint.as_deref());
            let mut missing = Vec::new();
            if disjoint.is_none() {
                missing.push("disjoint or overlapping");
            }
            if complete.is_none() {
                missing.push("complete or incomplete");
            }
            if !missing.is_empty() {
                out.rule(
                    "ER-33",
                    key,
                    Severity::Warning,
                    format!("this generalisation does not say whether it is {}", missing.join(" and ")),
                    None,
                );
            }
        }
    }
}

/// Every side of a relationship type (a line to an entity type, also through junction points) without a
/// cardinality: a hint at the piece of line that would hold it.
fn check_missing_cardinalities(doc: &Document, out: &mut Problems) {
    for n in doc.nodes().filter(|n| n.command == "relationship" && n.generalisation.is_none()) {
        for s in relationship_sides(doc, &n.id) {
            if s.card.text.is_empty() {
                out.rule(
                    "ER-11",
                    s.edge,
                    Severity::Hint,
                    "no cardinality on this entity–relationship edge",
                    None,
                );
            }
        }
    }
}

/// Two entity types joined through junction points only (no relationship in between): an error at the
/// first piece of line, once for each pair. Direct lines are found with the other edge checks.
fn check_entities_through_points(doc: &Document, out: &mut Problems) {
    let links = crate::cardinality::Links::new(doc);
    let points: HashSet<&str> = doc.points().map(|p| p.id.as_str()).collect();
    let mut done: HashSet<(String, String)> = HashSet::new();
    for n in doc.nodes().filter(|n| n.command == "entity" && n.generalisation.is_none()) {
        let mut found: Vec<(String, ElementKey)> = Vec::new();
        crate::cardinality::walk_ends(&links, &points, &n.id, |node, path| {
            if let [first, _, ..] = path
                && node_kind(doc, node) == Some("entity")
            {
                found.push((node.to_string(), *first));
            }
        });
        for (other, first) in found {
            let pair = if n.id <= other { (n.id.clone(), other) } else { (other, n.id.clone()) };
            if done.insert(pair) {
                out.rule(
                    "ER-5",
                    first,
                    Severity::Error,
                    "two entities are linked through a junction point; a relationship is needed in between",
                    None,
                );
            }
        }
    }
}

fn check_cardinalities(doc: &Document, out: &mut Problems) {
    let wanted = diagram_notation(doc);
    for (key, n) in keyed_nodes(doc) {
        if !RELATIONSHIPS.contains(&n.command.as_str()) {
            continue;
        }
        for s in relationship_sides(doc, &n.id) {
            let label =
                doc.get(s.edge).and_then(Element::as_edge).and_then(|e| e.label.clone()).unwrap_or_default();
            match (label_notation(&s.card), n_form(&s.card)) {
                ("invalid", _) => out.rule(
                    "ER-13",
                    s.edge,
                    Severity::Warning,
                    format!("'{label}' is not a valid cardinality; use the picker in the properties panel"),
                    None,
                ),
                ("old", Some(form)) => out.rule(
                    "ER-14",
                    s.edge,
                    Severity::Hint,
                    format!("the old form '{label}' stands for '{form}'"),
                    Fix::new(format!("Write {form}"), FixAction::NormaliseLabel),
                ),
                ("old", None) => out.rule(
                    "ER-14",
                    s.edge,
                    Severity::Hint,
                    format!(
                        "'{label}' has no n-notation form; (min,max) can express it, written at the other end"
                    ),
                    None,
                ),
                _ => {}
            }
        }
        let current = relationship_notation(doc, &n.id);
        if current == "mixed" {
            out.rule(
                "ER-15",
                key,
                Severity::Warning,
                "this relationship mixes (min,max) and n-notation, which count from opposite ends",
                None,
            );
        } else if current != wanted {
            let (changes, why) = plan_relationship(doc, &n.id, wanted);
            let fix = if why.is_none() {
                Fix::new(format!("Use {}", notation_name(wanted)), FixAction::ConvertLabels(changes))
            } else {
                None
            };
            out.rule(
                "ER-16",
                key,
                Severity::Hint,
                format!(
                    "this relationship uses {}, the diagram {}",
                    notation_name(current),
                    notation_name(wanted)
                ),
                fix,
            );
        }
    }
}

fn check_gsm(doc: &Document, out: &mut Problems) {
    let mut kind: HashMap<&str, &str> = HashMap::new();
    for n in doc.nodes() {
        kind.insert(&n.id, &n.command);
    }
    for p in doc.points() {
        kind.insert(&p.id, "point");
    }
    let mut isa_out: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut components: HashMap<&str, usize> = HashMap::new();
    for (key, e) in keyed_edges(doc) {
        let (Some(a), Some(b)) = (kind.get(e.src_id()).copied(), kind.get(e.dst_id()).copied()) else {
            continue;
        };
        if GSM_ISA.contains(&e.command.as_str()) {
            isa_out.entry(e.src_id()).or_default().push(e.dst_id());
            if a != "subtype" && a != "point" {
                out.rule(
                    "GSM-4",
                    key,
                    Severity::Error,
                    "an IS-A edge must start at a subtype (circle)",
                    None,
                );
            }
            if !matches!(b, "subtype" | "abstract" | "point") {
                out.rule(
                    "GSM-5",
                    key,
                    Severity::Error,
                    "an IS-A edge must end at a subtype or an abstract type",
                    None,
                );
            }
            if e.src_id() == e.dst_id() {
                out.rule("GSM-6", key, Severity::Error, "an IS-A edge from an element to itself", None);
            }
        } else if e.command == "component" {
            *components.entry(e.src_id()).or_default() += 1;
            if !matches!(a, "aggregation" | "grouping" | "point") {
                out.rule(
                    "GSM-11",
                    key,
                    Severity::Error,
                    "a component edge must start at an aggregation or grouping",
                    None,
                );
            }
        }
    }
    // all end points of IS-A paths from `start`; None on a cycle
    let roots = |start: &str| -> Option<BTreeSet<String>> {
        let mut found = BTreeSet::new();
        let mut stack: Vec<(&str, Vec<&str>)> = vec![(start, vec![start])];
        while let Some((n, path)) = stack.pop() {
            let next = isa_out.get(n).map_or(&[][..], Vec::as_slice);
            if next.is_empty() {
                found.insert(n.to_string());
            }
            for m in next {
                if path.contains(m) {
                    return None;
                }
                let mut p = path.clone();
                p.push(m);
                stack.push((m, p));
            }
        }
        Some(found)
    };
    let mut reported_cycle = false;
    for (key, n) in keyed_nodes(doc) {
        if n.command != "subtype" {
            continue;
        }
        if isa_out.get(n.id.as_str()).is_none_or(Vec::is_empty) {
            out.rule("GSM-10", key, Severity::Warning, "this subtype has no IS-A edge to a supertype", None);
            continue;
        }
        match roots(&n.id) {
            None => {
                if !reported_cycle {
                    out.rule("GSM-7", key, Severity::Error, "the IS-A edges form a cycle", None);
                    reported_cycle = true;
                }
            }
            Some(r) if r.len() > 1 => {
                let r: Vec<String> = r.into_iter().collect();
                out.rule(
                    "GSM-8",
                    key,
                    Severity::Error,
                    format!("IS-A paths from this subtype end at different roots: {}", r.join(", ")),
                    None,
                );
            }
            Some(r) => {
                if let Some(root) = r.iter().next()
                    && kind.get(root.as_str()) == Some(&"subtype")
                {
                    out.rule(
                        "GSM-9",
                        key,
                        Severity::Warning,
                        format!(
                            "the IS-A hierarchy ends at the subtype '{root}' instead of an abstract type"
                        ),
                        None,
                    );
                }
            }
        }
    }
    for (key, n) in keyed_nodes(doc) {
        let c = components.get(n.id.as_str()).copied().unwrap_or(0);
        if n.command == "grouping" && c != 1 {
            out.rule(
                "GSM-12",
                key,
                Severity::Warning,
                format!("a grouping needs exactly one component (dashed arrow), it has {c}"),
                None,
            );
        }
        if n.command == "aggregation" && c < 2 {
            out.rule(
                "GSM-1",
                key,
                Severity::Warning,
                "an aggregation needs at least two components (dashed arrows)",
                None,
            );
        }
    }
}
