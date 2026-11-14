//! The transformation, step by step: entity types, keys (supertypes and owners first),
//! generalisations, relationships, multi-valued attributes, views; then unique names.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::names::{self, drawn_ident, drawn_lower, guess_type, name_key, qualify};
use super::{
    Choice, Column, Decision, FkNaming, ForeignKey, Issue, Names, Origin, Overrides, RelNaming, Relation,
    RelationKind, Schema, Settings, TableId, View,
};
use crate::cardinality::{Card, Count, Links, RELATIONSHIPS, label_and_role, parse_card, walk_ends};
use crate::document::Document;
use crate::generalisation::{letters, parse_constraint};
use crate::latex_text::plain_text;
use crate::layout::{DummyEngine, Layout};
use crate::model::{AttrItem, Body, ElementKey, Node, endpoint_id};
use crate::rules::{Severity, is_double};
use crate::style::LabelLanguage;
use crate::subdomains::titles_by_element;
use crate::text::{cap_first, strip};

/// The relational schema of an ER diagram, with names in SQL form, and the same with names as
/// drawn in [`Schema::drawn`].
pub fn transform(doc: &Document, settings: &Settings, overrides: &Overrides) -> Schema {
    let mut schema = Transformer::new(doc, settings.clone(), overrides).run();
    let drawn = Settings { names: Names::Drawn, ..settings.clone() };
    schema.drawn = Some(Box::new(Transformer::new(doc, drawn, overrides).run()));
    schema
}

/// A map that keeps the order in which keys first came, as Python's dicts do.
struct Ordered<V> {
    keys: Vec<String>,
    map: HashMap<String, V>,
}

impl<V> Ordered<V> {
    fn new() -> Self {
        Ordered { keys: vec![], map: HashMap::new() }
    }

    fn get(&self, key: &str) -> Option<&V> {
        self.map.get(key)
    }

    fn contains(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    fn insert(&mut self, key: &str, value: V) {
        if self.map.insert(key.to_string(), value).is_none() {
            self.keys.push(key.to_string());
        }
    }

    fn entry(&mut self, key: &str) -> &mut V
    where
        V: Default,
    {
        if !self.map.contains_key(key) {
            self.keys.push(key.to_string());
        }
        self.map.entry(key.to_string()).or_default()
    }

    fn remove(&mut self, key: &str) {
        if self.map.remove(key).is_some() {
            self.keys.retain(|k| k != key);
        }
    }

    fn iter(&self) -> impl Iterator<Item = (&String, &V)> {
        self.keys.iter().filter_map(|k| self.map.get(k).map(|v| (k, v)))
    }
}

/// One side of a relationship (or any node reached by links).
#[derive(Clone, Debug)]
struct End {
    id: String,
    card: Card,
    role: String,
    edge: ElementKey,
    /// the label as written (see `normalise`)
    raw: Card,
}

impl End {
    /// Python's dataclass equality.
    fn same(&self, o: &End) -> bool {
        self.id == o.id
            && self.card == o.card
            && self.role == o.role
            && self.edge == o.edge
            && self.raw == o.raw
    }
}

/// The position of `sides[i]` among the sides with its id, counted from 1 (as Python's
/// `list.index`, which compares by value).
fn occurrence(sides: &[End], i: usize) -> usize {
    let Some(s) = sides.get(i) else { return 1 };
    let same: Vec<&End> = sides.iter().filter(|x| x.id == s.id).collect();
    same.iter().position(|x| x.same(s)).unwrap_or(0) + 1
}

#[derive(Clone, Debug, Default)]
struct Attr {
    name: String,
    key: bool,
    ty: Option<String>,
    derived: bool,
    multivalued: bool,
    parts: Vec<Attr>,
    formula: Option<String>,
    /// a further candidate key
    unique: bool,
}

impl Attr {
    fn has_key(&self) -> bool {
        self.key || self.leaves().iter().any(|l| l.key)
    }

    /// Those of its columns (in the order of `leaves`) that are key columns.
    fn key_names(&self, names: &[String]) -> Vec<String> {
        names.iter().zip(self.leaves()).filter(|(_, l)| self.key || l.key).map(|(n, _)| n.clone()).collect()
    }

    fn leaves(&self) -> Vec<&Attr> {
        if self.parts.is_empty() {
            return vec![self];
        }
        self.parts.iter().filter(|p| !p.derived).flat_map(Attr::leaves).collect()
    }
}

/// A Chen attribute while the trees are assembled: its parts by index.
struct ChenAttr {
    attr: Attr,
    parts: Vec<usize>,
}

#[derive(Clone, Debug)]
struct Isa<'a> {
    node: &'a Node,
    sup: String,
    subs: Vec<String>,
    complete: Option<bool>,
    disjoint: Option<bool>,
    option: &'static str,
    /// attribute | relation | none
    category: &'static str,
}

/// The weak relationship of a dependent entity: (dependent, owner, E or Id).
type Dependency = (String, String, &'static str);

/// What a decision is about, for its title.
#[derive(Clone)]
struct DecisionHead {
    element: String,
    title: String,
    involved: BTreeSet<String>,
    /// the rule in the rule books that describes the decision
    rule: &'static str,
}

impl DecisionHead {
    /// The same, described by `rule`.
    fn ruled(&self, rule: &'static str) -> DecisionHead {
        DecisionHead { rule, ..self.clone() }
    }
}

#[derive(Default)]
struct KeyFk<'s> {
    pk: bool,
    nullable: bool,
    cascade: bool,
    reason: String,
    merge: bool,
    front: bool,
    force_qualify: bool,
    unique: bool,
    role: &'s str,
}

impl KeyFk<'_> {
    fn new(reason: String) -> Self {
        KeyFk { nullable: true, reason, ..KeyFk::default() }
    }
}

struct Transformer<'a> {
    doc: &'a Document,
    st: Settings,
    overrides: &'a Overrides,
    issues: Vec<Issue>,
    decisions: Vec<Decision>,
    notes: Vec<String>,
    views: Vec<View>,
    tables: Vec<Relation>,
    relations: Vec<TableId>,
    unenforced: Vec<String>,
    rules: String,
    next_uid: usize,
    nodes: HashMap<&'a str, &'a Node>,
    points: HashSet<&'a str>,
    links: Links<'a>,
    entities: Vec<&'a Node>,
    rels: Vec<&'a Node>,
    attrs: Ordered<Vec<Attr>>,
    isas: Vec<Isa<'a>>,
    super_of: Ordered<usize>,
    more_supers: Ordered<Vec<usize>>,
    dep_rel: Ordered<Dependency>,
    /// recursive relationships with an existence dependency (chapters in chapters): the foreign key
    /// to the one above deletes those below it
    recursive_e: HashSet<String>,
    home: HashMap<String, Vec<TableId>>,
    own: HashMap<String, TableId>,
    rel_home: HashMap<String, TableId>,
    done_rels: HashSet<String>,
}

impl<'a> Transformer<'a> {
    fn new(doc: &'a Document, st: Settings, overrides: &'a Overrides) -> Self {
        let mut t = Transformer {
            doc,
            overrides,
            issues: vec![],
            decisions: vec![],
            notes: vec![],
            views: vec![],
            tables: vec![],
            relations: vec![],
            unenforced: vec![],
            rules: st.rules.clone().unwrap_or_else(|| "cdm".into()),
            next_uid: 0,
            nodes: HashMap::new(),
            points: HashSet::new(),
            links: Links::new(doc),
            entities: vec![],
            rels: vec![],
            attrs: Ordered::new(),
            isas: vec![],
            super_of: Ordered::new(),
            more_supers: Ordered::new(),
            dep_rel: Ordered::new(),
            recursive_e: HashSet::new(),
            home: HashMap::new(),
            own: HashMap::new(),
            rel_home: HashMap::new(),
            done_rels: HashSet::new(),
            st,
        };
        if t.st.rules.is_none() {
            for el in &doc.elements {
                if let Body::Setup(s) = &el.body
                    && s.opts.contains("rm rules")
                {
                    let v = s.opts.get("rm rules").filter(|v| !v.is_empty()).unwrap_or("cdm");
                    t.rules = strip(v).to_lowercase();
                }
            }
        }
        if t.rules != "cdm" && t.rules != "db1" {
            let msg = format!("unknown value '{}' of rm rules: ignored", t.rules);
            t.issue(Severity::Warning, msg, None);
            t.rules = "cdm".into();
        }
        t
    }

    // -- names: sanitised for SQL, as drawn for the schema output ---------------------------

    fn column_ident(&self, text: &str) -> String {
        if self.st.names == Names::Drawn { drawn_ident(text) } else { names::column_ident(text) }
    }

    fn relation_ident(&self, text: &str) -> String {
        if self.st.names == Names::Drawn { drawn_ident(text) } else { names::relation_ident(text) }
    }

    fn lower_ident(&self, text: &str) -> String {
        if self.st.names == Names::Drawn { drawn_lower(text) } else { names::lower_ident(text) }
    }

    /// In the schema with names as drawn: would they be the same name in SQL?
    fn same_name(&self, a: &str, b: &str) -> bool {
        if self.st.names == Names::Drawn { names::column_ident(a) == names::column_ident(b) } else { a == b }
    }

    // -- helpers ----------------------------------------------------------------------------

    fn opt(&self, n: &Node, key: &str) -> Option<String> {
        if let Some(ov) = self.overrides.get(&n.id)
            && let Some(v) = ov.get(key)
        {
            return v.clone();
        }
        n.opts.get(key).map(str::to_string)
    }

    fn opt_stripped(&self, n: &Node, key: &str) -> String {
        strip(&self.opt(n, key).unwrap_or_default()).to_string()
    }

    fn issue(&mut self, severity: Severity, message: impl Into<String>, element: Option<&str>) {
        self.issues.push(Issue {
            severity,
            message: message.into(),
            element_id: element.map(str::to_string),
        });
    }

    fn kind(&self, ident: &str) -> Option<&'a str> {
        match self.nodes.get(ident) {
            None => self.points.contains(ident).then_some("point"),
            Some(n) if n.generalisation.is_some() => Some("isa"),
            Some(n) => Some(n.command.as_str()),
        }
    }

    fn label(&self, ident: &str) -> String {
        match self.nodes.get(ident) {
            None => ident.to_string(),
            Some(n) if n.command == "partof" => "part of".into(),
            Some(n) if n.generalisation.is_some() => n.id.clone(),
            Some(n) => {
                let t = plain_text(n.label.as_deref().unwrap_or(""));
                if t.is_empty() { n.id.clone() } else { t }
            }
        }
    }

    fn table(&self, id: TableId) -> &Relation {
        static EMPTY: std::sync::OnceLock<Relation> = std::sync::OnceLock::new();
        self.tables.get(id.0).unwrap_or_else(|| {
            EMPTY.get_or_init(|| Relation::new(String::new(), RelationKind::Entity, &[], String::new()))
        })
    }

    fn table_mut(&mut self, id: TableId) -> Option<&mut Relation> {
        self.tables.get_mut(id.0)
    }

    fn add_table(&mut self, rel: Relation) -> TableId {
        self.tables.push(rel);
        TableId(self.tables.len() - 1)
    }

    fn remove_from_schema(&mut self, id: TableId) {
        if let Some(i) = self.relations.iter().position(|r| *r == id) {
            self.relations.remove(i);
        }
    }

    fn column(&mut self, name: String, ty: String) -> Column {
        self.next_uid += 1;
        Column {
            name,
            ty,
            pk: false,
            nullable: true,
            guessed: false,
            origin: Origin::Attribute,
            check_values: None,
            flag: false,
            qualifier: String::new(),
            ref_ent: String::new(),
            uid: self.next_uid,
        }
    }

    /// The nodes connected to `start` by links (through junction points).
    fn ends(&self, start: &str) -> Vec<End> {
        let mut out = Vec::new();
        walk_ends(&self.links, &self.points, start, |node, path| {
            let (labelled, role) = label_and_role(&self.links, path);
            let card = parse_card(labelled.and_then(|k| self.links.edge(k)).and_then(|e| e.label.as_deref()));
            let Some(edge) = path.last() else { return };
            out.push(End {
                id: node.to_string(),
                card: card.clone(),
                role: plain_text(role.unwrap_or("")),
                edge: *edge,
                raw: card,
            });
        });
        out
    }

    fn entity_sides(&self, start: &str) -> Vec<End> {
        self.ends(start).into_iter().filter(|e| self.kind(&e.id) == Some("entity")).collect()
    }

    /// Binary: make every side's `card` say how many of its entity one partner has.  A
    /// (min,max) at one side says exactly that about the *other* side.
    fn normalise(&mut self, r: &Node, sides: &mut [End]) {
        let lab = self.label(&r.id);
        for s in sides.iter() {
            if s.raw.unknown_min {
                self.add_note(format!("'{lab}': the unknown minimum in {} is treated as 0", s.raw.text));
            }
        }
        let [a, b] = sides else { return };
        if a.raw.known && b.raw.known && a.raw.here != b.raw.here {
            let msg = format!(
                "'{lab}' mixes the notations {} and {}, which count from opposite ends",
                a.raw.text, b.raw.text
            );
            self.issue(Severity::Warning, msg, Some(&r.id));
        }
        let effective = |own: &Card, other: &Card| {
            if other.here {
                Card { here: false, ..other.clone() }
            } else if !own.here {
                own.clone()
            } else {
                Card::unknown("")
            }
        };
        let (ca, cb) = (effective(&a.raw, &b.raw), effective(&b.raw, &a.raw));
        a.card = ca;
        b.card = cb;
    }

    fn add_note(&mut self, text: String) {
        if !self.notes.contains(&text) {
            self.notes.push(text);
        }
    }

    /// Bounds that foreign keys cannot enforce.  `holder`: the entity whose relation holds the
    /// (mandatory) foreign key of this relationship.
    fn note_bounds(&mut self, r: &Node, sides: &[End], holder: Option<&str>) {
        let lab = self.label(&r.id);
        let [a, b] = sides else {
            for s in sides {
                let open = s.raw.hi.as_ref().is_none_or(|h| *h == 1);
                if s.raw.here && s.raw.known && (s.raw.lo >= 1 || !open) {
                    let what = format!("each {} takes part in {lab}", self.label(&s.id));
                    self.note_count(what, &s.raw.lo, s.raw.hi.as_ref());
                }
            }
            return;
        };
        for (s, o) in [(a, b), (b, a)] {
            if !o.card.known {
                continue;
            }
            let lo = if (o.card.lo >= 1 && Some(s.id.as_str()) != holder) || o.card.lo > 1 {
                o.card.lo.clone()
            } else {
                Count::from(0)
            };
            let hi = o.card.hi.clone().filter(|h| *h != 1);
            if lo != 0 || hi.is_some() {
                let what = format!("each {} has, in {lab}, {}", self.label(&s.id), self.label(&o.id));
                self.note_count(what, &lo, hi.as_ref());
            }
        }
    }

    fn note_count(&mut self, what: String, lo: &Count, hi: Option<&Count>) {
        let mut bits = Vec::new();
        if *lo != 0 {
            bits.push(format!("at least {lo}"));
        }
        if let Some(h) = hi.filter(|h| **h != 1) {
            bits.push(format!("at most {h}"));
        }
        if !bits.is_empty() {
            let text = format!("{what}: {}", bits.join(" and "));
            if !self.unenforced.contains(&text) {
                self.unenforced.push(text);
            }
        }
    }

    // -- entry point ------------------------------------------------------------------------

    fn run(mut self) -> Schema {
        let doc = self.doc;
        for n in doc.nodes() {
            self.nodes.insert(&n.id, n);
        }
        self.points = doc.points().map(|p| p.id.as_str()).collect();
        self.entities = doc.nodes().filter(|n| n.command == "entity" && n.generalisation.is_none()).collect();
        self.rels = doc.nodes().filter(|n| RELATIONSHIPS.contains(&n.command.as_str())).collect();
        if self.entities.is_empty() {
            self.issue(Severity::Warning, "The diagram has no entity types.", None);
        }
        self.collect_attributes();
        self.collect_generalisations();
        self.collect_weak();
        self.phase_entities();
        self.phase_keys();
        self.phase_generalisations();
        self.phase_relationships();
        self.phase_multivalued();
        self.phase_views();
        self.finish();
        self.assign_boxes();
        let mut relations = self.relations.clone();
        relations.sort_by_key(|id| self.table(*id).kind);
        Schema {
            tables: self.tables,
            relations,
            decisions: self.decisions,
            issues: self.issues,
            notes: self.notes,
            unenforced: self.unenforced,
            rules: self.rules,
            views: self.views,
            drawn: None,
        }
    }

    // -- attributes -------------------------------------------------------------------------

    fn collect_attributes(&mut self) {
        let doc = self.doc;
        let chen: Vec<&Node> = doc.nodes().filter(|n| n.command == "attribute").collect();
        let mut owner: HashMap<&str, Option<String>> = HashMap::new();
        let mut nbrs: HashMap<&str, Vec<String>> = HashMap::new();
        for a in &chen {
            let (mut owners, mut anbrs): (Vec<String>, Vec<String>) = (vec![], vec![]);
            let of = a.opts.get("of").filter(|o| !o.is_empty()).map(endpoint_id);
            if let Some(of) = of {
                if self.kind(of) == Some("attribute") {
                    anbrs.push(of.to_string()); // component of a composite attribute
                } else if !of.is_empty() && self.kind(of).is_some() {
                    owners.push(of.to_string());
                }
            }
            for end in self.ends(&a.id) {
                match self.kind(&end.id) {
                    Some("entity" | "relationship" | "partof") => {
                        if !owners.contains(&end.id) {
                            owners.push(end.id);
                        }
                    }
                    Some("attribute") => anbrs.push(end.id),
                    _ => {}
                }
            }
            if owners.len() > 1 {
                let msg = format!(
                    "attribute '{}' belongs to several elements; used for '{}'",
                    self.label(&a.id),
                    owners.first().map(|o| self.label(o)).unwrap_or_default()
                );
                self.issue(Severity::Warning, msg, Some(&a.id));
            }
            owner.insert(&a.id, owners.into_iter().next());
            nbrs.insert(&a.id, anbrs);
        }
        // the Chen attributes as Python's shared objects: one per id, parts by index
        let mut arena: Vec<ChenAttr> = Vec::new();
        let mut made: HashMap<String, usize> = HashMap::new();
        let mut make = |n: &Node, arena: &mut Vec<ChenAttr>| -> usize {
            if let Some(&i) = made.get(&n.id) {
                return i;
            }
            let label = plain_text(n.label.as_deref().unwrap_or(""));
            arena.push(ChenAttr {
                attr: Attr {
                    name: if label.is_empty() { n.id.clone() } else { label },
                    key: n.opts.contains("key"),
                    ty: n.opts.get("type").map(str::to_string),
                    derived: n.opts.contains("derived"),
                    multivalued: n.opts.contains("multivalued"),
                    parts: vec![],
                    formula: n.opts.get("formula").map(str::to_string),
                    unique: n.opts.contains("unique"),
                },
                parts: vec![],
            });
            made.insert(n.id.clone(), arena.len() - 1);
            arena.len() - 1
        };
        let mut parent: HashMap<&str, String> = HashMap::new();
        for a in &chen {
            if owner.get(a.id.as_str()).is_some_and(Option::is_some) {
                continue;
            }
            let cands = nbrs.get(a.id.as_str()).cloned().unwrap_or_default();
            let pick = cands
                .iter()
                .find(|b| owner.get(b.as_str()).is_some_and(|o| o.as_ref().is_some_and(|o| !o.is_empty())))
                .or_else(|| {
                    cands
                        .iter()
                        .find(|b| self.nodes.get(b.as_str()).is_some_and(|n| n.opts.contains("composite")))
                })
                .or_else(|| cands.first());
            match pick {
                Some(p) => {
                    parent.insert(&a.id, p.clone());
                }
                None => {
                    let msg = format!(
                        "attribute '{}' is not connected to an entity or relationship and is ignored",
                        self.label(&a.id)
                    );
                    self.issue(Severity::Warning, msg, Some(&a.id));
                }
            }
        }
        // components of composites (guard against cycles)
        for a in &chen {
            let Some(first) = parent.get(a.id.as_str()) else { continue };
            let mut p = first.clone();
            let mut seen: HashSet<String> = HashSet::from([a.id.clone()]);
            while let Some(up) = parent.get(p.as_str()) {
                if seen.contains(&p) {
                    break;
                }
                seen.insert(p.clone());
                p = up.clone();
            }
            if seen.contains(&p) {
                let msg = format!(
                    "attribute '{}' is part of a cycle of attributes and is ignored",
                    self.label(&a.id)
                );
                self.issue(Severity::Warning, msg, Some(&a.id));
                continue;
            }
            let Some(&pn) = self.nodes.get(first.as_str()) else { continue };
            let pi = make(pn, &mut arena);
            let ci = make(a, &mut arena);
            if let Some(slot) = arena.get_mut(pi) {
                slot.parts.push(ci);
            }
        }
        fn build(arena: &[ChenAttr], i: usize, depth: usize) -> Attr {
            let Some(c) = arena.get(i) else { return Attr::default() };
            let mut a = c.attr.clone();
            if depth < 64 {
                a.parts = c.parts.iter().map(|p| build(arena, *p, depth + 1)).collect();
            }
            a
        }
        // in document order: trees and top-level ellipses
        for el in &doc.elements {
            match &el.body {
                Body::Attributes(t) => {
                    for it in &t.items {
                        let a = self.attr_of_item(it, &t.node);
                        self.attrs.entry(&t.node).push(a);
                    }
                }
                Body::Node(n) if n.command == "attribute" => {
                    let Some(Some(o)) = owner.get(n.id.as_str()).cloned() else { continue };
                    if o.is_empty() {
                        continue;
                    }
                    if self.kind(&o) == Some("isa") {
                        self.issue(
                            Severity::Warning,
                            "attributes of a generalisation are ignored",
                            Some(&n.id),
                        );
                        continue;
                    }
                    let i = make(n, &mut arena);
                    let a = build(&arena, i, 0);
                    self.attrs.entry(&o).push(a);
                }
                _ => {}
            }
        }
    }

    fn attr_of_item(&mut self, it: &AttrItem, node: &str) -> Attr {
        let name = plain_text(&it.text);
        let parts = it.parts.iter().map(|p| self.attr_of_item(p, node)).collect();
        let a = Attr {
            name: if name.is_empty() { it.text.clone() } else { name },
            key: it.key,
            ty: it.r#type.clone(),
            derived: it.derived,
            multivalued: it.multivalued,
            parts,
            formula: it.formula.clone(),
            unique: it.unique,
        };
        for p in &it.parts {
            if p.multivalued && !p.derived {
                let msg = format!(
                    "multi-valued part {} of the composite attribute {}: stored as one column",
                    plain_text(&p.text),
                    a.name
                );
                self.issue(Severity::Warning, msg, Some(node));
            }
        }
        a
    }

    fn columns_for(&mut self, attr: &Attr, pk_allowed: bool) -> Vec<Column> {
        let mut out = Vec::new();
        for leaf in attr.leaves() {
            let given = leaf.ty.clone().filter(|t| !t.is_empty());
            let ty = given.clone().unwrap_or_else(|| guess_type(&leaf.name).to_string());
            let key = attr.key || leaf.key;
            let name = self.column_ident(&leaf.name);
            let mut c = self.column(name, ty);
            c.pk = key && pk_allowed;
            c.nullable = !key;
            c.guessed = given.is_none();
            out.push(c);
        }
        out
    }

    /// The columns of an attribute; on a clash, the parts of a composite attribute are prefixed
    /// with its name, all others with their owner's.
    fn add_attribute(
        &mut self,
        rel: TableId,
        attr: &Attr,
        owner: &str,
        pk_allowed: bool,
        nullable_all: bool,
    ) -> Vec<String> {
        let mut names = Vec::new();
        let prefix =
            self.lower_ident(&if attr.parts.is_empty() { self.label(owner) } else { attr.name.clone() });
        for mut c in self.columns_for(attr, pk_allowed) {
            if nullable_all && !c.pk {
                c.nullable = true;
            }
            names.push(self.add_column(rel, c, &prefix, false));
        }
        // further candidate keys: UNIQUE, where NULL stays allowed
        if let Some(r) = self.table_mut(rel) {
            if attr.unique && !attr.key {
                r.uniques.push(names.clone());
            } else if !attr.key && !attr.parts.is_empty() {
                for (n, leaf) in names.iter().zip(attr.leaves()) {
                    if leaf.unique {
                        r.uniques.push(vec![n.clone()]);
                    }
                }
            }
        }
        names
    }

    // -- generalisations --------------------------------------------------------------------

    fn collect_generalisations(&mut self) {
        let doc = self.doc;
        for n in doc.nodes() {
            let Some(g) = &n.generalisation else { continue };
            let sup = endpoint_id(&g.supertype).to_string();
            if self.kind(&sup) != Some("entity") {
                self.issue(
                    Severity::Warning,
                    format!("the supertype of generalisation '{}' is not an entity", n.id),
                    Some(&n.id),
                );
                continue;
            }
            let mut subs: Vec<String> = Vec::new();
            for end in self.ends(&n.id) {
                if self.kind(&end.id) == Some("entity") && end.id != sup && !subs.contains(&end.id) {
                    subs.push(end.id);
                }
            }
            if subs.is_empty() {
                self.issue(
                    Severity::Warning,
                    format!("generalisation '{}' has no subtypes", n.id),
                    Some(&n.id),
                );
                continue;
            }
            let (complete, disjoint) = parse_constraint(g.constraint.as_deref());
            let index = self.isas.len();
            for s in &subs {
                if self.super_of.contains(s) {
                    self.more_supers.entry(s).push(index);
                } else {
                    self.super_of.insert(s, index);
                }
            }
            self.isas.push(Isa { node: n, sup, subs, complete, disjoint, option: "1", category: "none" });
        }
        // cycles
        for s in self.super_of.keys.clone() {
            if let Some(cycle) = self.cycles_through(&s).into_iter().next() {
                self.issue(
                    Severity::Error,
                    format!("generalisation cycle through '{}'", self.label(&s)),
                    None,
                );
                for x in cycle {
                    self.super_of.remove(&x);
                    self.more_supers.remove(&x);
                }
            }
        }
    }

    fn supers_of(&self, ent: &str) -> Vec<String> {
        let first = self.super_of.get(ent).and_then(|i| self.isas.get(*i)).map(|i| i.sup.clone());
        let more = self
            .more_supers
            .get(ent)
            .into_iter()
            .flatten()
            .filter_map(|i| self.isas.get(*i))
            .map(|i| i.sup.clone());
        first.into_iter().chain(more).collect()
    }

    fn cycles_through(&self, start: &str) -> Vec<Vec<String>> {
        fn walk(
            t: &Transformer<'_>,
            start: &str,
            ent: &str,
            path: &mut Vec<String>,
            found: &mut Vec<Vec<String>>,
        ) {
            for sup in t.supers_of(ent) {
                if sup == start {
                    let mut c = path.clone();
                    c.push(ent.to_string());
                    found.push(c);
                } else if !path.contains(&sup) && sup != ent {
                    path.push(ent.to_string());
                    walk(t, start, &sup, path, found);
                    path.pop();
                }
            }
        }
        let mut found = Vec::new();
        walk(self, start, start, &mut Vec::new(), &mut found);
        found
    }

    fn merged_into_super(&self, ent: &str) -> bool {
        self.super_of
            .get(ent)
            .and_then(|i| self.isas.get(*i))
            .is_some_and(|i| i.option == "3" || i.option == "4")
    }

    fn decide_isa(&mut self, index: usize) {
        let Some(info) = self.isas.get(index).cloned() else { return };
        let n = info.node;
        let (c, d) = (info.complete, info.disjoint);
        let sup = self.label(&info.sup);
        let subs = info.subs.iter().map(|s| self.label(s)).collect::<Vec<_>>().join(", ");
        let cons = letters(c, d).unwrap_or_else(|| "constraint not given".into());
        let unknown = c.is_none() || d.is_none();
        let has_rels =
            self.ends(&info.sup).iter().any(|e| self.kind(&e.id).is_some_and(|k| RELATIONSHIPS.contains(&k)))
                || !self.weak_owner_of_any(&info.sup).is_empty();
        let merged = self.merged_into_super(&info.sup);
        let shared = self.isas.iter().filter(|i| i.sup == info.sup).count() > 1;
        let several = info.subs.iter().find(|s| self.supers_of(s).len() > 1).cloned();
        let need = |choice: &mut Choice, ok: bool, why: String| {
            choice.valid = ok;
            choice.reason = if ok { String::new() } else { why };
        };
        let rel_ident = self.relation_ident(&sup);
        let mut opts = vec![
            Choice::new(
                "(1) Multiple relations + categorisation attribute",
                &[("rm", Some("1")), ("rm category", Some("attribute"))],
                format!(
                    "One relation per entity type; the subtype relations inherit the key of {sup}.  A categorisation \
                     attribute in {sup} names the subtype of each tuple."
                ),
            ),
            Choice::new(
                "(1) Multiple relations + categorisation relation",
                &[("rm", Some("1")), ("rm category", Some("relation"))],
                format!(
                    "One relation per entity type plus a relation {rel_ident}Category (key, category) listing the \
                     subtypes of each tuple."
                ),
            ),
            Choice::new(
                "(1) Multiple relations, no categorisation",
                &[("rm", Some("1")), ("rm category", Some("none"))],
                format!(
                    "One relation per entity type; the subtype relations inherit the key of {sup}.  The subtype of a \
                     tuple follows from the subtype relations."
                ),
            ),
            Choice::new(
                "(2) Subtype relations only",
                &[("rm", Some("2")), ("rm category", None)],
                format!(
                    "No relation for {sup}; each subtype relation contains all attributes of {sup}.  Outer joins are \
                     needed to list all {sup} tuples."
                ),
            ),
            Choice::new(
                "(3) Single relation + categorisation attribute",
                &[("rm", Some("3")), ("rm category", None)],
                format!(
                    "One relation for {sup} and all subtypes ({subs}); subtype attributes may be NULL; a \
                     categorisation attribute names the subtype."
                ),
            ),
            Choice::new(
                "(4) Single relation + Boolean flags",
                &[("rm", Some("4")), ("rm category", None)],
                format!(
                    "One relation for {sup} and all subtypes; one Boolean attribute per subtype (is…) marks \
                     membership, so overlapping is possible."
                ),
            ),
        ];
        let several_label = several.as_deref().map(|s| self.label(s));
        if let [o1, _, _, o2, o3, o4] = opts.as_mut_slice() {
            need(o1, d != Some(false), "needs a disjoint generalisation (one value per tuple)".into());
            let why2 = if let Some(s) = &several_label {
                format!("{s} has several supertypes")
            } else if merged {
                format!("{sup} is itself merged into its supertype's relation")
            } else if shared {
                format!("{sup} has another generalisation, which needs its relation")
            } else {
                "needs a complete and disjoint generalisation whose supertype has no relationships of its own"
                    .into()
            };
            need(
                o2,
                c != Some(false) && d != Some(false) && !has_rels && !merged && !shared && several.is_none(),
                why2,
            );
            let why3 = match &several_label {
                Some(s) => format!("{s} has several supertypes, and can't be merged into one"),
                None => "needs a disjoint generalisation".into(),
            };
            need(o3, d != Some(false) && several.is_none(), why3);
            let why4 = several_label
                .as_ref()
                .map(|s| format!("{s} has several supertypes, and can't be merged into one"));
            need(o4, several.is_none(), why4.unwrap_or_default());
        }
        if unknown {
            for o in &mut opts {
                let rm = o.assigned("rm").map(str::to_string);
                if (o.valid && matches!(rm.as_deref(), Some("2" | "3")))
                    || o.assigned("rm category") == Some("attribute")
                {
                    o.reason = format!(
                        "assumes the generalisation is {}",
                        if rm.as_deref() == Some("2") { "complete and disjoint" } else { "disjoint" }
                    );
                }
            }
        }
        let default = if self.rules == "db1" {
            2
        } else {
            match d {
                Some(true) => 0,
                Some(false) => 1,
                None => 2,
            }
        };
        // the current choice, from the element
        let rm = self.opt_stripped(n, "rm");
        let cat = self.opt_stripped(n, "rm category");
        let mut current = match rm.as_str() {
            "1" => match cat.as_str() {
                "attribute" => 0,
                "relation" => 1,
                "none" => 2,
                _ => default,
            },
            "2" => 3,
            "3" => 4,
            "4" => 5,
            "" => default,
            other => {
                self.issue(
                    Severity::Warning,
                    format!("unknown option rm={other} of '{}'", n.id),
                    Some(&n.id),
                );
                default
            }
        };
        if let Some(o) = opts.get(current)
            && !o.valid
        {
            let msg = format!(
                "generalisation {sup}: {} is not possible ({}); the default is used",
                o.title, o.reason
            );
            self.issue(Severity::Warning, msg, Some(&n.id));
            current = default;
        }
        if let Some(o) = opts.get_mut(default) {
            o.assign = vec![("rm".into(), None), ("rm category".into(), None)];
            o.title += "  (default)";
        }
        let mut involved: BTreeSet<String> = info.subs.iter().cloned().collect();
        involved.insert(info.sup.clone());
        involved.insert(n.id.clone());
        self.decisions.push(Decision {
            element_id: n.id.clone(),
            title: format!("Generalisation {sup} ← {subs} ({cons})"),
            question: "How should the generalisation be represented?".into(),
            options: opts,
            current,
            default,
            involved,
            minor: false,
            rule: "REL-17",
        });
        if let Some(i) = self.isas.get_mut(index) {
            i.option = match current {
                3 => "2",
                4 => "3",
                5 => "4",
                _ => "1",
            };
            i.category = match current {
                0 => "attribute",
                1 => "relation",
                _ => "none",
            };
        }
    }

    // -- weak entities ------------------------------------------------------------------------

    /// Identifying / existence relationships: rel id -> (dependent, owner, mode).
    fn collect_weak(&mut self) {
        for r in self.rels.clone() {
            let dep = strip(r.opts.get("dependency").unwrap_or("")).to_string();
            let weak = r.opts.is_on("weak");
            if dep.is_empty() && !weak {
                continue;
            }
            let mut sides = self.entity_sides(&r.id);
            self.normalise(r, &mut sides);
            let weak_sides: Vec<usize> =
                (0..sides.len()).filter(|i| sides.get(*i).is_some_and(|s| self.is_weak(&s.id))).collect();
            if sides.len() != 2 || weak_sides.is_empty() {
                let msg = format!(
                    "weak relationship '{}' is not a binary relationship with a weak entity; treated as normal relationship",
                    self.label(&r.id)
                );
                self.issue(Severity::Hint, msg, Some(&r.id));
                continue;
            }
            let side = |i: usize| sides.get(i).cloned();
            let doubled: Vec<usize> = weak_sides
                .iter()
                .copied()
                .filter(|i| side(*i).and_then(|s| self.links.edge(s.edge)).is_some_and(is_double))
                .collect();
            let dependent = match (weak_sides.as_slice(), doubled.as_slice()) {
                ([_, _], [one]) => *one,
                ([w0, w1], _) => {
                    let d =
                        (0..sides.len()).find(|i| side(*i).is_some_and(|s| !s.card.single())).unwrap_or(1);
                    let (a, b) = (side(*w0).map(|s| s.id), side(*w1).map(|s| s.id));
                    if a != b {
                        let msg = format!(
                            "'{}' joins two weak entities, and no single double line says which one depends on the other; {} was taken.",
                            self.label(&r.id),
                            side(d).map(|s| self.label(&s.id)).unwrap_or_default()
                        );
                        self.notes.push(msg);
                    }
                    d
                }
                (ws, _) => ws.first().copied().unwrap_or(0),
            };
            let owner = 1 - dependent.min(1);
            let (Some(dep_side), Some(owner_side)) = (side(dependent), side(owner)) else { continue };
            if owner_side.id == dep_side.id {
                // an entity type depending on itself: a normal recursive relationship, whose foreign
                // key deletes those below (E); Id would put the key into itself, without end
                if dep == "E" {
                    self.recursive_e.insert(r.id.clone());
                } else {
                    let msg = format!(
                        "the weak relationship '{}' would make {} identification dependent on itself, which is \
                         impossible (its key would contain itself); treated as a normal relationship",
                        self.label(&r.id),
                        self.label(&owner_side.id)
                    );
                    self.issue(Severity::Warning, msg, Some(&r.id));
                }
                continue;
            }
            let mode = if dep == "E" { "E" } else { "Id" };
            if dep.is_empty() {
                let msg = format!(
                    "'{}' is weak without E/Id; treated as an identification dependency (Id).",
                    self.label(&r.id)
                );
                self.notes.push(msg);
            }
            self.dep_rel.insert(&r.id, (dep_side.id.clone(), owner_side.id.clone(), mode));
            if owner_side.card.known && !owner_side.card.single() {
                let msg = format!(
                    "the owner side of '{}' should have cardinality 1 (each weak entity depends on one owner)",
                    self.label(&r.id)
                );
                self.issue(Severity::Warning, msg, Some(&r.id));
            }
        }
    }

    fn is_weak(&self, ent: &str) -> bool {
        self.nodes.get(ent).is_some_and(|n| n.opts.is_on("weak"))
    }

    fn weak_owner_of_any(&self, ent: &str) -> Vec<String> {
        self.dep_rel.iter().filter(|(_, (_, o, _))| o == ent).map(|(r, _)| r.clone()).collect()
    }

    // -- phase A: own relations -------------------------------------------------------------

    fn table_name(&self, el: &Node) -> String {
        let custom = self.opt_stripped(el, "rm name");
        if custom.is_empty() { self.relation_ident(&self.label(&el.id)) } else { custom }
    }

    fn phase_entities(&mut self) {
        for e in self.entities.clone() {
            let comment = format!(
                "entity type {}{}",
                self.label(&e.id),
                if self.is_weak(&e.id) { " (weak)" } else { "" }
            );
            let rel =
                self.add_table(Relation::new(self.table_name(e), RelationKind::Entity, &[&e.id], comment));
            let subtype = self.super_of.contains(&e.id);
            let attrs = self.attrs.get(&e.id).cloned().unwrap_or_default();
            for a in &attrs {
                if a.derived {
                    self.add_note(format!(
                        "derived attribute {}.{} is not stored",
                        self.label(&e.id),
                        a.name
                    ));
                    continue;
                }
                if a.multivalued {
                    continue;
                }
                let names = self.add_attribute(rel, a, &e.id, !subtype, false);
                if subtype && a.has_key() {
                    let key = a.key_names(&names);
                    if let Some(r) = self.table_mut(rel) {
                        for c in &mut r.columns {
                            if key.contains(&c.name) {
                                c.nullable = false;
                            }
                        }
                        r.uniques.push(key);
                    }
                }
            }
            let weak_dep = self.dep_rel.iter().any(|(_, (d, _, m))| *d == e.id && *m == "Id");
            let has_pk = !self.table(rel).pk().is_empty();
            if !subtype && !has_pk && !weak_dep {
                let base = self.lower_ident(&self.label(&e.id)) + "ID";
                let mut name = base.clone();
                let mut k = 2;
                while self.table(rel).col(&name).is_some() {
                    name = format!("{base}{k}");
                    k += 1;
                }
                let mut c = self.column(name.clone(), "integer".into());
                c.pk = true;
                c.nullable = false;
                c.origin = Origin::Surrogate;
                if let Some(r) = self.table_mut(rel) {
                    r.columns.insert(0, c);
                }
                let msg = format!(
                    "entity type {} has no key attribute; a key column '{name}' was added",
                    self.label(&e.id)
                );
                self.issue(Severity::Warning, msg, Some(&e.id));
            }
            if weak_dep && !has_pk {
                let msg = format!(
                    "weak entity type {} has no partial key; its key consists of the owner's key only",
                    self.label(&e.id)
                );
                self.issue(Severity::Hint, msg, Some(&e.id));
            }
            self.relations.push(rel);
            self.own.insert(e.id.clone(), rel);
            self.home.insert(e.id.clone(), vec![rel]);
        }
    }

    /// Adds a column, renamed with `prefix` if the name is taken; returns its name.
    fn add_column(&mut self, rel: TableId, mut col: Column, prefix: &str, front: bool) -> String {
        let Some(r) = self.table_mut(rel) else { return col.name };
        if r.col(&col.name).is_some() {
            let base = if prefix.is_empty() { col.name.clone() } else { qualify(prefix, &col.name) };
            let (mut name, mut k) = (base.clone(), 2);
            while r.col(&name).is_some() {
                name = format!("{base}{k}");
                k += 1;
            }
            col.name = name;
        }
        let name = col.name.clone();
        if front {
            // after the columns already inserted at the front (primary key first)
            let idx = r.columns.iter().position(|c| !c.pk).unwrap_or(r.columns.len());
            r.columns.insert(idx, col);
        } else {
            r.columns.push(col);
        }
        name
    }

    // -- phase B: keys (topological: supertypes and owners first) ---------------------------

    fn phase_keys(&mut self) {
        // decisions for generalisations, top-down (merged supertypes known)
        fn depth(
            t: &Transformer<'_>,
            ent: &str,
            seen: &mut Vec<String>,
            memo: &mut HashMap<String, usize>,
        ) -> usize {
            if let Some(d) = memo.get(ent) {
                return *d;
            }
            let supers: Vec<String> =
                t.supers_of(ent).into_iter().filter(|s| !seen.contains(s) && s != ent).collect();
            let mut best = 0;
            for s in supers {
                seen.push(ent.to_string());
                best = best.max(depth(t, &s, seen, memo) + 1);
                seen.pop();
            }
            memo.insert(ent.to_string(), best);
            best
        }
        let mut memo = HashMap::new();
        let keys: Vec<usize> =
            self.isas.iter().map(|i| depth(self, &i.sup, &mut Vec::new(), &mut memo)).collect();
        let mut order: Vec<usize> = (0..self.isas.len()).collect();
        order.sort_by_key(|i| keys.get(*i).copied().unwrap_or(0));
        for i in order {
            self.decide_isa(i);
        }
        // dependency order
        let mut deps: HashMap<String, Vec<String>> = HashMap::new();
        let mut subtypes: Vec<String> = self.super_of.keys.clone();
        subtypes.extend(self.more_supers.keys.iter().cloned());
        let subtypes: BTreeSet<String> = subtypes.into_iter().collect();
        for s in subtypes {
            let supers = self.supers_of(&s);
            deps.entry(s).or_default().extend(supers);
        }
        for (_, (dep, owner, mode)) in self.dep_rel.iter() {
            if *mode == "Id" {
                deps.entry(dep.clone()).or_default().push(owner.clone());
            }
        }
        let mut order: Vec<String> = Vec::new();
        let mut state: HashMap<String, u8> = HashMap::new();
        fn visit(
            t: &mut Transformer<'_>,
            ent: &str,
            deps: &HashMap<String, Vec<String>>,
            state: &mut HashMap<String, u8>,
            order: &mut Vec<String>,
        ) {
            match state.get(ent) {
                Some(2) => return,
                Some(1) => {
                    let msg = format!("cyclic key dependency through '{}'", t.label(ent));
                    t.issue(Severity::Error, msg, Some(ent));
                    return;
                }
                _ => {}
            }
            state.insert(ent.to_string(), 1);
            for x in deps.get(ent).into_iter().flatten() {
                visit(t, x, deps, state, order);
            }
            state.insert(ent.to_string(), 2);
            order.push(ent.to_string());
        }
        for e in self.entities.clone() {
            visit(self, &e.id, &deps, &mut state, &mut order);
        }
        for ent in order {
            if let Some(&info) = self.super_of.get(&ent) {
                self.attach_subtype(&ent, info);
                for extra in self.more_supers.get(&ent).cloned().unwrap_or_default() {
                    self.attach_further_supertype(&ent, info, extra);
                }
            }
            let deps: Vec<(String, Dependency)> =
                self.dep_rel.iter().map(|(r, d)| (r.clone(), d.clone())).collect();
            for (r, (dep, owner, mode)) in deps {
                if dep == ent && mode == "Id" {
                    self.attach_dependent(&r, &dep, &owner, mode);
                }
            }
        }
    }

    fn homes(&self, ent: &str) -> Vec<TableId> {
        self.home.get(ent).cloned().unwrap_or_default()
    }

    /// The key columns of an entity's relation: (name, type, guessed).
    fn key_of(&self, ent: &str) -> Vec<(String, String, bool)> {
        let Some(first) = self.homes(ent).first().copied() else { return vec![] };
        self.table(first)
            .columns
            .iter()
            .filter(|c| c.pk)
            .map(|c| (c.name.clone(), c.ty.clone(), c.guessed))
            .collect()
    }

    fn attach_subtype(&mut self, ent: &str, info: usize) {
        let Some(info) = self.isas.get(info).cloned() else { return };
        let sup_home = self.homes(&info.sup);
        let Some(&own) = self.own.get(ent) else { return };
        let sup_label = self.label(&info.sup);
        match info.option {
            "1" => {
                let fk = KeyFk {
                    pk: true,
                    nullable: false,
                    cascade: true,
                    merge: true,
                    front: true,
                    ..KeyFk::new(format!("subtype of {sup_label}"))
                };
                self.add_key_fk(own, &info.sup, "", fk);
                if let Some(r) = self.table_mut(own) {
                    r.comment += &format!(", subtype of {sup_label}");
                }
            }
            "2" => {
                let Some(&src) = sup_home.first() else { return };
                let src_rel = self.table(src).clone();
                let inherited: Vec<Column> = src_rel
                    .columns
                    .iter()
                    .map(|c| {
                        self.next_uid += 1;
                        Column { uid: self.next_uid, ..c.clone() }
                    })
                    .collect();
                let mine = match self.table_mut(own) {
                    Some(r) => std::mem::replace(&mut r.columns, inherited),
                    None => return,
                };
                let prefix = self.lower_ident(&self.label(ent));
                let declared: HashSet<String> =
                    self.table(own).uniques.iter().flatten().map(|n| name_key(n)).collect();
                for c in mine {
                    // the supertype's key repeated in the subtype is the same column; any other
                    // attribute of the same name is a different one
                    if declared.contains(&name_key(&c.name)) && self.table(own).col(&c.name).is_some() {
                        continue;
                    }
                    self.add_column(own, c, &prefix, false);
                }
                if let Some(r) = self.table_mut(own) {
                    let pk: HashSet<String> =
                        r.columns.iter().filter(|c| c.pk).map(|c| name_key(&c.name)).collect();
                    let uniques = std::mem::take(&mut r.uniques);
                    let kept: Vec<Vec<String>> = uniques
                        .into_iter()
                        .filter(|u| {
                            u.iter().all(|n| r.col(n).is_some())
                                && u.iter().map(|n| name_key(n)).collect::<HashSet<_>>() != pk
                        })
                        .collect();
                    r.fks.extend(src_rel.fks.iter().cloned());
                    r.uniques = src_rel.uniques.iter().cloned().chain(kept).collect();
                    r.sources.insert(info.sup.clone());
                    r.comment += &format!(", includes the attributes of {sup_label} (option 2)");
                }
            }
            _ => {
                // 3 / 4: merged into the supertype's relation
                let Some(&target) = sup_home.first() else { return };
                let prefix = self.lower_ident(&self.label(ent));
                let own_rel = self.table(own).clone();
                let mut renamed: HashMap<String, String> = HashMap::new();
                for mut c in own_rel.columns {
                    let old = c.name.clone();
                    c.pk = false;
                    c.nullable = true;
                    let new = self.add_column(target, c, &prefix, false);
                    renamed.insert(old, new);
                }
                let rename = |x: &String| renamed.get(x).cloned().unwrap_or_else(|| x.clone());
                if let Some(t) = self.table_mut(target) {
                    for u in &own_rel.uniques {
                        t.uniques.push(u.iter().map(rename).collect());
                    }
                    for fk in &own_rel.fks {
                        t.fks.push(ForeignKey {
                            columns: fk.columns.iter().map(rename).collect(),
                            ..fk.clone()
                        });
                    }
                    t.sources.extend(own_rel.sources.iter().cloned());
                }
                if let Some(r) = self.table_mut(own) {
                    // the columns moved, as Python moves the objects
                    r.columns.clear();
                    r.fks.clear();
                }
                self.remove_from_schema(own);
                self.home.insert(ent.to_string(), vec![target]);
                self.retarget(own, target);
            }
        }
    }

    /// Where the supertypes of an entity end.
    fn roots(&self, ent: &str) -> BTreeSet<String> {
        let mut all = vec![ent.to_string()];
        all.extend(self.ancestors(ent));
        let tops: BTreeSet<String> = all.into_iter().filter(|x| self.supers_of(x).is_empty()).collect();
        if tops.is_empty() { BTreeSet::from([ent.to_string()]) } else { tops }
    }

    /// The relation refers to every supertype.  Its key comes from the first.  A second one
    /// above the same root has the same key: the same column gets a second foreign key.
    /// Otherwise its key is a further column, one for one and never empty.
    fn attach_further_supertype(&mut self, ent: &str, first: usize, extra: usize) {
        let (Some(first), Some(extra)) = (self.isas.get(first).cloned(), self.isas.get(extra).cloned())
        else {
            return;
        };
        let Some(&own) = self.own.get(ent) else { return };
        let sup = extra.sup.clone();
        let label = self.label(&sup);
        let homes = self.homes(&sup);
        if sup == first.sup || self.table(own).fks.iter().any(|fk| homes.contains(&fk.table)) {
            return;
        }
        let reason = format!("subtype of {label}");
        if let Some(r) = self.table_mut(own) {
            r.comment += &format!(", {reason}");
        }
        let names: Vec<String> = self.key_of(&sup).into_iter().map(|k| k.0).collect();
        let common = !self.roots(&first.sup).is_disjoint(&self.roots(&sup));
        let same_key =
            common && !names.is_empty() && names.iter().all(|n| self.table(own).col(n).is_some_and(|c| c.pk));
        match homes.as_slice() {
            [home] if same_key => {
                if let Some(r) = self.table_mut(own) {
                    r.fks.push(ForeignKey {
                        columns: names.clone(),
                        table: *home,
                        ref_columns: names,
                        cascade: true,
                        reason,
                    });
                }
            }
            _ => {
                let q = self.lower_ident(&label);
                let fk = KeyFk { nullable: false, cascade: true, unique: true, ..KeyFk::new(reason) };
                self.add_key_fk(own, &sup, &q, fk);
            }
        }
    }

    fn retarget(&mut self, old: TableId, new: TableId) {
        for id in self.relations.clone() {
            if let Some(r) = self.table_mut(id) {
                for fk in &mut r.fks {
                    if fk.table == old {
                        fk.table = new;
                    }
                }
            }
        }
    }

    fn attach_dependent(&mut self, r: &str, dep: &str, owner: &str, mode: &str) {
        let Some(&rel) = self.homes(dep).first() else { return };
        // a weak subtype merged into its supertype can't extend that relation's key
        let own = self.own.get(dep) == Some(&rel);
        if mode == "Id" && !own {
            let msg = format!(
                "{} is merged into {}, so its dependency on {} is only a foreign key",
                self.label(dep),
                self.table(rel).name,
                self.label(owner)
            );
            self.issue(Severity::Hint, msg, Some(r));
        }
        let identifying = mode == "Id" && own;
        let what = if mode == "Id" { "identification" } else { "existence" };
        let fk = KeyFk {
            pk: identifying,
            nullable: !own,
            cascade: own,
            merge: own,
            front: identifying,
            ..KeyFk::new(format!("{what} dependency '{}'", self.label(r)))
        };
        let q = self.lower_ident(&self.label(owner));
        self.add_key_fk(rel, owner, &q, fk);
        self.relationship_attributes(r, rel, !own, identifying);
        self.done_rels.insert(r.to_string());
    }

    /// Python's `list.remove`: the first column equal to `uid`'s column in every field.
    fn remove_column(rel: &mut Relation, uid: usize) -> Option<Column> {
        let target = rel.columns.iter().find(|c| c.uid == uid)?.clone();
        let i = rel.columns.iter().position(|c| Column { uid, ..c.clone() } == target)?;
        Some(rel.columns.remove(i))
    }

    /// Adds the key of `ref_ent` to `rel` as foreign key; returns the column names.
    /// `qualifier`: the relationship's name, used when two foreign keys clash.
    fn add_key_fk(&mut self, rel: TableId, ref_ent: &str, qualifier: &str, o: KeyFk<'_>) -> Vec<String> {
        let key = self.key_of(ref_ent);
        let homes = self.homes(ref_ent);
        if key.is_empty() {
            let msg = format!(
                "{} has no key; foreign key in {} omitted",
                self.label(ref_ent),
                self.table(rel).name
            );
            self.issue(Severity::Warning, msg, Some(ref_ent));
            return vec![];
        }
        let mut names = Vec::new();
        let qualified =
            o.force_qualify || (self.st.fk_naming == FkNaming::Relationship && !qualifier.is_empty());
        let mut pos = {
            let r = self.table(rel);
            r.columns
                .iter()
                .position(|c| !(c.pk && c.origin == Origin::ForeignKey))
                .unwrap_or(r.columns.len())
        };
        for (k_name, k_type, k_guessed) in &key {
            let name = if !o.role.is_empty() {
                if key.len() == 1 {
                    self.column_ident(o.role)
                } else {
                    qualify(&self.lower_ident(o.role), k_name)
                }
            } else if qualified {
                qualify(qualifier, k_name)
            } else {
                k_name.clone()
            };
            let existing = self.table(rel).col_index(&name);
            let fk_cols = self.table(rel).fk_columns();
            if o.merge
                && let Some(ei) = existing
                && !fk_cols.iter().any(|n| self.same_name(&name, n))
            {
                let Some(r) = self.table_mut(rel) else { break };
                let Some(ex) = r.columns.get_mut(ei) else { break };
                ex.pk = ex.pk || o.pk;
                ex.nullable = ex.nullable && o.nullable;
                ex.origin = Origin::ForeignKey;
                ex.ref_ent = ref_ent.to_string();
                if ex.guessed {
                    ex.ty = k_type.clone();
                    ex.guessed = *k_guessed;
                }
                let (uid, ex_name) = (ex.uid, ex.name.clone());
                if o.front
                    && o.pk
                    && let Some(col) = Self::remove_column(r, uid)
                {
                    pos = pos.min(r.columns.len());
                    r.columns.insert(pos, col);
                    pos += 1;
                }
                names.push(ex_name);
                continue;
            }
            let mut c = self.column(name, k_type.clone());
            c.pk = o.pk;
            c.nullable = o.nullable && !o.pk;
            c.guessed = *k_guessed;
            c.origin = Origin::ForeignKey;
            c.qualifier = qualifier.to_string();
            c.ref_ent = ref_ent.to_string();
            if let Some(ei) = existing
                && o.role.is_empty()
                && !qualified
                && let Some(ex) = self.table(rel).columns.get(ei).cloned()
            {
                let unrenamed = ex.origin == Origin::ForeignKey && !ex.pk && self.same_name(&ex.name, k_name);
                if ex.origin == Origin::ForeignKey && ex.ref_ent == ref_ent {
                    // two foreign keys to one entity: the relationships tell them apart
                    if unrenamed && !ex.qualifier.is_empty() {
                        Self::rename_column(self.table_mut(rel), ex.uid, &qualify(&ex.qualifier, k_name));
                    }
                    c.name = qualify(qualifier, k_name);
                } else {
                    // the same name in another entity: the entities tell them apart
                    if unrenamed && !ex.ref_ent.is_empty() {
                        let new = self.relation_ident(&self.label(&ex.ref_ent)) + &cap_first(k_name);
                        Self::rename_column(self.table_mut(rel), ex.uid, &new);
                    }
                    c.name = self.relation_ident(&self.label(ref_ent)) + &cap_first(k_name);
                }
            }
            if o.front {
                let uid = c.uid;
                let mut c = c;
                if self.table(rel).col(&c.name).is_some() {
                    let name = self.add_column(rel, c.clone(), qualifier, false); // a unique name
                    if let Some(r) = self.table_mut(rel) {
                        Self::remove_column(r, uid);
                    }
                    c.name = name;
                }
                names.push(c.name.clone());
                if let Some(r) = self.table_mut(rel) {
                    let at = pos.min(r.columns.len());
                    r.columns.insert(at, c);
                }
                pos += 1;
            } else {
                names.push(self.add_column(rel, c, qualifier, false));
            }
        }
        if o.unique
            && let Some(r) = self.table_mut(rel)
        {
            r.uniques.push(names.clone());
        }
        match homes.as_slice() {
            [home] => {
                let fk = ForeignKey {
                    columns: names.clone(),
                    table: *home,
                    ref_columns: key.iter().map(|k| k.0.clone()).collect(),
                    cascade: o.cascade,
                    reason: o.reason,
                };
                if let Some(r) = self.table_mut(rel) {
                    r.fks.push(fk);
                }
            }
            _ => {
                let msg = format!(
                    "{}.{} refers to {}, which has no single relation (generalisation option 2); the foreign key cannot be declared",
                    self.table(rel).name,
                    names.join(", "),
                    self.label(ref_ent)
                );
                self.issue(Severity::Warning, msg, Some(ref_ent));
            }
        }
        names
    }

    fn rename_column(rel: Option<&mut Relation>, uid: usize, new: &str) {
        let Some(rel) = rel else { return };
        if rel.col(new).is_some() {
            return;
        }
        let Some(col) = rel.columns.iter_mut().find(|c| c.uid == uid) else { return };
        let old = std::mem::replace(&mut col.name, new.to_string());
        for fk in &mut rel.fks {
            for x in &mut fk.columns {
                if *x == old {
                    *x = new.to_string();
                }
            }
        }
        for u in &mut rel.uniques {
            for x in u {
                if *x == old {
                    *x = new.to_string();
                }
            }
        }
    }

    /// The attributes of relationship `r` in `rel`; with `keys`, its key attributes join the
    /// primary key.  Returns the names of those key columns.
    fn relationship_attributes(
        &mut self,
        r: &str,
        rel: TableId,
        nullable_all: bool,
        keys: bool,
    ) -> Vec<String> {
        self.rel_home.insert(r.to_string(), rel);
        let mut key_names = Vec::new();
        for a in self.attrs.get(r).cloned().unwrap_or_default() {
            if a.derived {
                self.add_note(format!("derived attribute {}.{} is not stored", self.label(r), a.name));
                continue;
            }
            if a.multivalued {
                continue;
            }
            let names = self.add_attribute(rel, &a, r, keys, nullable_all);
            key_names.extend(a.key_names(&names));
        }
        key_names
    }

    fn has_key_attributes(&self, r: &str) -> bool {
        self.attrs
            .get(r)
            .is_some_and(|attrs| attrs.iter().any(|a| a.has_key() && !a.derived && !a.multivalued))
    }

    // -- phase C: categorisation, dissolving option-2 supertypes ----------------------------

    fn phase_generalisations(&mut self) {
        for info in self.isas.clone() {
            let sup_home = self.homes(&info.sup);
            let sub_labels: Vec<String> = info.subs.iter().map(|s| self.label(s)).collect();
            let n = info.node;
            let sup_label = self.label(&info.sup);
            if info.option == "2" {
                if let Some(&old) = sup_home.first()
                    && self.relations.contains(&old)
                {
                    self.remove_from_schema(old);
                    let subs: Vec<TableId> =
                        info.subs.iter().filter_map(|s| self.own.get(s).copied()).collect();
                    self.home.insert(info.sup.clone(), subs);
                }
                continue;
            }
            let Some(&target) = sup_home.first() else { continue };
            if info.category == "attribute" || info.option == "3" {
                let mut name = self.opt_stripped(n, "rm name");
                if name.is_empty() {
                    // in a relation shared with the supertype's own supertype
                    name = if self.own.get(&info.sup) == Some(&target) {
                        "category".into()
                    } else {
                        self.lower_ident(&sup_label) + "Category"
                    };
                }
                let name = self.column_ident(&name);
                let mut c = self.column(name, "string".into());
                c.nullable = info.complete != Some(true);
                c.origin = Origin::Category;
                c.check_values = Some(sub_labels.clone());
                let prefix = self.lower_ident(&sup_label);
                self.add_column(target, c, &prefix, false);
            } else if info.category == "relation" {
                let rel = self.add_table(Relation::new(
                    self.relation_ident(&sup_label) + "Category",
                    RelationKind::Category,
                    &[&info.sup, &n.id],
                    format!("categorisation of {sup_label} (overlapping)"),
                ));
                let fk = KeyFk {
                    pk: true,
                    nullable: false,
                    cascade: true,
                    ..KeyFk::new(format!("categorisation of {sup_label}"))
                };
                self.add_key_fk(rel, &info.sup, "", fk);
                let custom = self.opt_stripped(n, "rm name");
                let name = self.column_ident(if custom.is_empty() { "category" } else { &custom });
                let mut c = self.column(name, "string".into());
                c.pk = true;
                c.nullable = false;
                c.origin = Origin::Category;
                c.check_values = Some(sub_labels.clone());
                self.add_column(rel, c, "", false);
                self.relations.push(rel);
            }
            if info.option == "4" {
                for s in &info.subs {
                    let name = format!("is{}", cap_first(&self.relation_ident(&self.label(s))));
                    let mut c = self.column(name, "boolean".into());
                    c.nullable = false;
                    c.origin = Origin::Flag;
                    c.flag = true;
                    self.add_column(target, c, "", false);
                }
            }
        }
    }

    // -- phase D: relationships ---------------------------------------------------------------

    fn side_token(sides: &[End], i: usize) -> String {
        let Some(s) = sides.get(i) else { return String::new() };
        if sides.iter().filter(|x| x.id == s.id).count() == 1 {
            return s.id.clone();
        }
        if s.role.is_empty() {
            format!("{}#{}", s.id, occurrence(sides, i))
        } else {
            names::lower_ident(&s.role)
        }
    }

    /// Rows of the relation are not all of this entity type (merged subtype).
    fn is_conditional(&self, ent: &str) -> bool {
        self.merged_into_super(ent) || self.ancestors(ent).iter().any(|x| self.merged_into_super(x))
    }

    fn ancestors(&self, ent: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut todo: std::collections::VecDeque<String> = self.supers_of(ent).into();
        while let Some(cur) = todo.pop_front() {
            if !out.contains(&cur) && cur != ent {
                todo.extend(self.supers_of(&cur));
                out.push(cur);
            }
        }
        out
    }

    /// A foreign key to `reference` in the relation(s) of `target`.
    #[allow(clippy::too_many_arguments)]
    fn fk_into(
        &mut self,
        target: &str,
        reference: &End,
        r: &Node,
        nullable: bool,
        unique: bool,
        attrs: bool,
        force: bool,
        cascade: bool,
    ) {
        let cond = self.is_conditional(target);
        for rel in self.homes(target) {
            let fk = KeyFk {
                nullable: nullable || cond,
                unique,
                cascade,
                force_qualify: force && reference.role.is_empty(),
                role: &reference.role,
                ..KeyFk::new(format!("relationship {}", self.label(&r.id)))
            };
            let q = self.lower_ident(&self.label(&r.id));
            self.add_key_fk(rel, &reference.id, &q, fk);
            if attrs {
                self.relationship_attributes(&r.id, rel, cond, false);
            }
        }
    }

    fn rel_table_name(&self, r: &Node, sides: &[End]) -> String {
        let custom = self.opt_stripped(r, "rm name");
        if !custom.is_empty() {
            return custom;
        }
        let lab = self.label(&r.id);
        if let (RelNaming::Entities, [a, b]) = (self.st.rel_naming, sides) {
            let name = |s: &End| match self.nodes.get(s.id.as_str()) {
                Some(n) => self.table_name(n),
                None => self.relation_ident(&self.label(&s.id)),
            };
            return format!("{}_{}_{}", name(a), self.lower_ident(&lab), name(b));
        }
        if self.st.rel_naming == RelNaming::Relationship {
            self.lower_ident(&lab)
        } else {
            self.relation_ident(&lab)
        }
    }

    /// An own relation.  `unique_sets`: further candidate keys, as sets of sides.
    /// `required`: all foreign keys are NOT NULL (a tuple always has all its partners).
    fn make_table(
        &mut self,
        r: &Node,
        sides: &[End],
        key_sides: &[usize],
        kind_text: &str,
        unique_sets: &[Vec<usize>],
        required: bool,
    ) -> TableId {
        let comment = format!("relationship {} ({kind_text})", self.label(&r.id));
        let rel = self.add_table(Relation::new(
            self.rel_table_name(r, sides),
            RelationKind::Relationship,
            &[&r.id],
            comment,
        ));
        let dup: HashSet<&str> = sides
            .iter()
            .filter(|s| sides.iter().filter(|x| x.id == s.id).count() > 1)
            .map(|s| s.id.as_str())
            .collect();
        let mut columns: HashMap<usize, Vec<usize>> = HashMap::new(); // columns by uid: later clashes may rename them
        let required = required || sides.len() == 2;
        let reason = format!("relationship {}", self.label(&r.id));
        for (i, s) in sides.iter().enumerate() {
            let pk = key_sides.contains(&i);
            let nullable = !pk && !required && s.card.optional();
            let mut names;
            if dup.contains(s.id.as_str()) && s.role.is_empty() {
                // no role: number them (playerNo_1, playerNo_2)
                names = self.add_key_fk(rel, &s.id, "", KeyFk { pk, nullable, ..KeyFk::new(reason.clone()) });
                let idx = occurrence(sides, i);
                let key = self.key_of(&s.id);
                for (j, (k_name, _, _)) in key.iter().enumerate() {
                    let new = format!("{k_name}_{idx}");
                    let Some(nm) = names.get(j).cloned() else { continue };
                    if self.table(rel).col(&new).is_none() {
                        if let Some(uid) = self.table(rel).col(&nm).map(|c| c.uid) {
                            Self::rename_column(self.table_mut(rel), uid, &new);
                        }
                        if let Some(slot) = names.get_mut(j) {
                            *slot = new;
                        }
                    }
                }
            } else {
                let q = self.lower_ident(&self.label(&r.id));
                names = self.add_key_fk(
                    rel,
                    &s.id,
                    &q,
                    KeyFk { pk, nullable, role: &s.role, ..KeyFk::new(reason.clone()) },
                );
            }
            let uids = names.iter().filter_map(|n| self.table(rel).col(n).map(|c| c.uid)).collect();
            columns.insert(i, uids);
        }
        let key_attrs = self.relationship_attributes(&r.id, rel, false, true);
        for group in unique_sets {
            let r = self.table(rel);
            let mut u: Vec<String> = group
                .iter()
                .flat_map(|s| columns.get(s).cloned().unwrap_or_default())
                .filter_map(|uid| r.columns.iter().find(|c| c.uid == uid).map(|c| c.name.clone()))
                .collect();
            u.extend(key_attrs.iter().cloned());
            if let Some(r) = self.table_mut(rel) {
                r.uniques.push(u);
            }
        }
        self.relations.push(rel);
        rel
    }

    fn phase_relationships(&mut self) {
        for r in self.rels.clone() {
            let mut sides = self.entity_sides(&r.id);
            if self.done_rels.contains(&r.id) {
                // identifying relationship of a weak entity
                self.normalise(r, &mut sides);
                let holder = self.dep_rel.get(&r.id).map(|d| d.0.clone());
                self.note_bounds(r, &sides, holder.as_deref());
                continue;
            }
            if sides.len() < 2 {
                let msg = format!(
                    "relationship '{}' connects fewer than two entity types and is skipped",
                    self.label(&r.id)
                );
                self.issue(Severity::Warning, msg, Some(&r.id));
                continue;
            }
            for s in &sides {
                if !s.card.known {
                    let what = if s.card.text.is_empty() {
                        "is missing".to_string()
                    } else {
                        format!("'{}' is not understood", s.card.text)
                    };
                    let msg = format!(
                        "the cardinality between {} and '{}' {what}; n assumed",
                        self.label(&s.id),
                        self.label(&r.id)
                    );
                    self.issue(Severity::Warning, msg, Some(&r.id));
                }
            }
            self.normalise(r, &mut sides);
            if let Some((dep, owner, _)) = self.dep_rel.get(&r.id).cloned() {
                // existence dependency (E)
                if self.has_key_attributes(&r.id) {
                    let msg = format!(
                        "the key attributes of '{}' are not part of the key, because the weak entity's own key identifies it",
                        self.label(&r.id)
                    );
                    self.issue(Severity::Hint, msg, Some(&r.id));
                }
                for rel in self.homes(&dep) {
                    let fk = KeyFk {
                        nullable: false,
                        cascade: true,
                        ..KeyFk::new(format!("existence dependency '{}'", self.label(&r.id)))
                    };
                    let q = self.lower_ident(&self.label(&owner));
                    self.add_key_fk(rel, &owner, &q, fk);
                    self.relationship_attributes(&r.id, rel, false, false);
                }
                self.note_bounds(r, &sides, Some(&dep));
                continue;
            }
            self.relationship(r, &sides);
        }
    }

    fn decide(
        &mut self,
        head: &DecisionHead,
        question: &str,
        mut opts: Vec<Choice>,
        current: usize,
        default: usize,
        minor: bool,
    ) -> usize {
        if let Some(o) = opts.get_mut(default) {
            let mut assign: Vec<(String, Option<String>)> =
                o.assign.iter().map(|(k, _)| (k.clone(), None)).collect();
            if assign.is_empty() {
                assign.push(("rm".into(), None));
            }
            o.assign = assign;
            o.title += "  (default)";
        }
        self.decisions.push(Decision {
            element_id: head.element.clone(),
            title: head.title.clone(),
            question: question.into(),
            options: opts,
            current,
            default,
            involved: head.involved.clone(),
            minor,
            rule: head.rule,
        });
        current
    }

    fn relationship(&mut self, r: &Node, sides: &[End]) {
        let lab = self.label(&r.id);
        let singles: Vec<usize> =
            (0..sides.len()).filter(|i| sides.get(*i).is_some_and(|s| s.card.single())).collect();
        let complexes: Vec<usize> = (0..sides.len()).filter(|i| !singles.contains(i)).collect();
        let ids: HashSet<&str> = sides.iter().map(|s| s.id.as_str()).collect();
        let recursive = ids.len() < sides.len();
        let cards = sides
            .iter()
            .map(|s| if s.raw.text.is_empty() { "?" } else { s.raw.text.as_str() })
            .collect::<Vec<_>>()
            .join(":");
        let rm = self.opt_stripped(r, "rm");
        let names: Vec<String> = sides
            .iter()
            .map(|s| {
                self.label(&s.id) + &if s.role.is_empty() { String::new() } else { format!(" ({})", s.role) }
            })
            .collect();
        let mut involved: BTreeSet<String> = sides.iter().map(|s| s.id.clone()).collect();
        involved.insert(r.id.clone());
        let head = DecisionHead {
            element: r.id.clone(),
            title: format!("Relationship {lab} ({cards}): {}", names.join(" – ")),
            involved,
            rule: "",
        };
        let forced = self.has_key_attributes(&r.id); // key attributes: always an own relation
        let own_because_of_keys = |t: &mut Self| {
            t.add_note(format!("'{lab}' has key attributes, so it always gets an own relation."))
        };
        let side = |i: usize| {
            sides.get(i).cloned().unwrap_or(End {
                id: String::new(),
                card: Card::unknown(""),
                role: String::new(),
                edge: ElementKey(0),
                raw: Card::unknown(""),
            })
        };
        let first = |v: &[usize]| v.first().copied().unwrap_or(0);

        if sides.len() > 2 && sides.iter().any(|s| s.raw.here) {
            self.nary_min_max(r, sides, &head, &cards);
            return;
        }
        if sides.len() == 2 {
            let (a, b) = (side(0), side(1));
            let table = self.rel_table_name(r, sides);
            if recursive {
                if singles.is_empty() {
                    self.make_table(r, sides, &[0, 1], &format!("recursive, {cards}"), &[], false);
                    self.note_bounds(r, sides, None);
                    return;
                }
                // the referenced side's role names the foreign key column
                let (ref_i, tgt_i) =
                    if singles.len() == 1 { (first(&singles), first(&complexes)) } else { (1, 0) };
                let (reference, tgt) = (side(ref_i), side(tgt_i));
                let unique: Vec<Vec<usize>> = if singles.len() == 2 { vec![vec![ref_i]] } else { vec![] };
                if forced {
                    own_because_of_keys(self);
                    self.make_table(r, sides, &[tgt_i], &format!("recursive, {cards}"), &unique, false);
                    self.note_bounds(r, sides, None);
                    return;
                }
                let column = if reference.role.is_empty() {
                    let key = self
                        .key_of(&reference.id)
                        .first()
                        .map(|k| k.0.clone())
                        .unwrap_or_else(|| "key".into());
                    qualify(&self.lower_ident(&lab), &key)
                } else {
                    self.column_ident(&reference.role)
                };
                // an existence dependency (weak, E): only the foreign key can delete those below
                let existence = self.recursive_e.contains(&r.id);
                let default = if existence || self.rules == "db1" { 1 } else { 0 };
                let cur = match rm.as_str() {
                    "table" => 0,
                    "fk" => 1,
                    _ => default,
                };
                let ent = self.label(&a.id);
                let (own_extra, fk_extra) = if existence {
                    (
                        format!(
                            " Deleting a {ent} only deletes its rows here: the {ent} below it stay, although they depend on it."
                        ),
                        format!(
                            " With ON DELETE CASCADE, so that deleting a {ent} deletes those below it (existence dependency)."
                        ),
                    )
                } else {
                    (String::new(), String::new())
                };
                let opts = vec![
                    Choice::new(
                        "Own relation with renamed keys",
                        &[("rm", Some("table"))],
                        format!(
                            "A relation {table} with the key of {ent} twice, named by the roles.{own_extra}"
                        ),
                    ),
                    Choice::new(
                        format!("Foreign key {column} in {ent}"),
                        &[("rm", Some("fk"))],
                        format!(
                            "Each {} refers to at most one other, so the key can be repeated in the same relation.{fk_extra}",
                            self.label(&tgt.id)
                        ),
                    ),
                ];
                self.decide(
                    &head.ruled("REL-12"),
                    "Own relation or foreign key for this recursive relationship?",
                    opts,
                    cur,
                    default,
                    false,
                );
                if cur == 1 {
                    let optional = reference.card.optional();
                    self.fk_into(&tgt.id, &reference, r, optional, singles.len() == 2, true, true, existence);
                    let holder = (!optional).then_some(tgt.id.as_str());
                    self.note_bounds(r, sides, holder);
                } else {
                    self.make_table(r, sides, &[tgt_i], &format!("recursive, {cards}"), &unique, false);
                    self.note_bounds(r, sides, None);
                }
                return;
            }
            if singles.is_empty() {
                // n:n
                self.make_table(r, sides, &[0, 1], "complex-complex", &[], false);
                self.note_bounds(r, sides, None);
                return;
            }
            if singles.len() == 1 {
                // 1:n
                let (ref_i, tgt_i) = (first(&singles), first(&complexes));
                let (reference, tgt) = (side(ref_i), side(tgt_i));
                if forced {
                    own_because_of_keys(self);
                    self.make_table(r, sides, &[tgt_i], "single-complex", &[], false);
                    self.note_bounds(r, sides, None);
                    return;
                }
                let cur = usize::from(rm == "table");
                let opts = vec![
                    Choice::new(
                        format!("Foreign key in {}", self.label(&tgt.id)),
                        &[("rm", None)],
                        format!(
                            "The key of {} becomes a foreign key of {}; no own relation.",
                            self.label(&reference.id),
                            self.label(&tgt.id)
                        ),
                    ),
                    Choice::new(
                        "Own relation",
                        &[("rm", Some("table"))],
                        format!(
                            "A relation {table} with the key of {} as primary key (relations with the same key can, but need not, be merged).",
                            self.label(&tgt.id)
                        ),
                    ),
                ];
                self.decide(&head.ruled("REL-8"), "Foreign key or own relation?", opts, cur, 0, true);
                if cur == 1 {
                    self.make_table(r, sides, &[tgt_i], "single-complex", &[], false);
                    self.note_bounds(r, sides, None);
                } else {
                    let optional = reference.card.optional();
                    self.fk_into(&tgt.id, &reference, r, optional, false, true, false, false);
                    self.note_bounds(r, sides, (!optional).then_some(tgt.id.as_str()));
                }
                return;
            }
            // single-single
            if a.card.optional() != b.card.optional() {
                // 1:c -> the foreign key on the c side
                let tgt_i = if a.card.optional() { 0 } else { 1 };
                let ref_i = 1 - tgt_i;
                let (reference, tgt) = (side(ref_i), side(tgt_i));
                if forced {
                    own_because_of_keys(self);
                    self.make_table(r, sides, &[tgt_i], "c:1", &[vec![ref_i]], false);
                    self.note_bounds(r, sides, None);
                    return;
                }
                let cur = usize::from(rm == "table");
                let opts = vec![
                    Choice::new(
                        format!("Foreign key in {}", self.label(&tgt.id)),
                        &[("rm", None)],
                        format!(
                            "The key of {} becomes a unique, mandatory foreign key of {}, which gives no NULL values.",
                            self.label(&reference.id),
                            self.label(&tgt.id)
                        ),
                    ),
                    Choice::new(
                        "Own relation",
                        &[("rm", Some("table"))],
                        format!(
                            "A relation {table} with the key of {} as primary key; the key of {} is unique.",
                            self.label(&tgt.id),
                            self.label(&reference.id)
                        ),
                    ),
                ];
                self.decide(&head.ruled("REL-9"), "Foreign key or own relation?", opts, cur, 0, true);
                if cur == 1 {
                    self.make_table(r, sides, &[tgt_i], "c:1", &[vec![ref_i]], false);
                    self.note_bounds(r, sides, None);
                } else {
                    self.fk_into(&tgt.id, &reference, r, false, true, true, false, false);
                    self.note_bounds(r, sides, Some(&tgt.id));
                }
                return;
            }
            let both_c = a.card.optional();
            if forced {
                own_because_of_keys(self);
                self.make_table(r, sides, &[0], &cards, &[vec![1]], false);
                self.note_bounds(r, sides, None);
                return;
            }
            let why = if both_c {
                "Fewer NULL values result on the side whose entities more often take part."
            } else {
                ""
            };
            let null = if both_c { ", may be NULL" } else { ", not NULL" };
            let (la, lb) = (self.label(&a.id), self.label(&b.id));
            let opts = vec![
                Choice::new(
                    format!("Foreign key in {la}"),
                    &[("rm", Some(&a.id))],
                    format!("The key of {lb} becomes a foreign key (unique{null}) of {la}. {why}"),
                ),
                Choice::new(
                    format!("Foreign key in {lb}"),
                    &[("rm", Some(&b.id))],
                    format!("The key of {la} becomes a foreign key (unique{null}) of {lb}. {why}"),
                ),
                Choice::new(
                    "Own relation",
                    &[("rm", Some("table"))],
                    format!(
                        "A relation {table} with both keys and no NULL values; the key of {la} is the primary key, the other is unique."
                    ),
                ),
            ];
            let cur = if rm == b.id {
                1
            } else if rm == "table" {
                2
            } else {
                0
            };
            self.decide(&head.ruled("REL-9"), "Which relation gets the foreign key?", opts, cur, 0, false);
            if cur == 2 {
                self.make_table(r, sides, &[0], &cards, &[vec![1]], false);
                self.note_bounds(r, sides, None);
            } else {
                let (tgt, reference) = if cur == 0 { (a, b) } else { (b, a) };
                self.fk_into(&tgt.id, &reference, r, both_c, true, true, false, false);
                self.note_bounds(r, sides, if both_c { None } else { Some(&tgt.id) });
            }
            return;
        }
        // n-ary in n-notation
        let kind_text = format!("{}-ary {cards}", sides.len());
        if singles.len() < 2 {
            // none or one single side: only one candidate key
            let key: Vec<usize> =
                if complexes.is_empty() { (0..sides.len()).collect() } else { complexes.clone() };
            self.make_table(r, sides, &key, &kind_text, &[], false);
            return;
        }
        let tokens: Vec<String> = (0..sides.len()).map(|i| Self::side_token(sides, i)).collect();
        let token = |i: usize| tokens.get(i).cloned().unwrap_or_default();
        let chosen: BTreeSet<String> = self
            .opt(r, "rm key")
            .unwrap_or_default()
            .split(',')
            .map(|t| strip(t).to_string())
            .filter(|t| !t.is_empty())
            .collect();
        let mut opts: Vec<Choice> = Vec::new();
        let mut actions: Vec<(&str, Vec<usize>)> = Vec::new();
        let side_names = complexes.iter().map(|i| self.label(&side(*i).id)).collect::<Vec<_>>().join(", ");
        if complexes.len() == 1 && !forced {
            let c = self.label(&side(first(&complexes)).id);
            let mut o = Choice::new(
                format!("Foreign keys in {c}"),
                &[("rm", Some("fold")), ("rm key", None)],
                format!("The keys of the single sides become foreign keys of {c}."),
            );
            o.reason = format!(
                "Only valid if the key of {c} alone identifies the relationship, so that each {c} takes part at most once."
            );
            opts.push(o);
            actions.push(("fold", vec![]));
        }
        if !complexes.is_empty() {
            let mut o = Choice::new(
                format!("Own relation, key: {side_names}"),
                &[("rm", Some("table")), ("rm key", None)],
                "Primary key = keys of the complex sides; the single sides are foreign keys.",
            );
            o.reason = format!("Only valid if the keys of {side_names} alone identify the relationship.");
            opts.push(o);
            actions.push(("table", complexes.clone()));
        }
        let first_candidate = opts.len();
        for &drop in &singles {
            let key: Vec<usize> = (0..sides.len()).filter(|i| *i != drop).collect();
            let extras: Vec<String> = key.iter().filter(|i| singles.contains(i)).map(|i| token(*i)).collect();
            let title =
                key.iter().map(|i| names.get(*i).cloned().unwrap_or_default()).collect::<Vec<_>>().join(", ");
            let joined = extras.join(",");
            opts.push(Choice::new(
                format!("Own relation, key: {title}"),
                &[("rm", Some("table")), ("rm key", Some(&joined))],
                format!(
                    "For a side marked 1, the keys of all other sides are a candidate key: these determine {}. The other candidate keys become UNIQUE.",
                    self.label(&side(drop).id)
                ),
            ));
            actions.push(("table", key));
        }
        // The default is a candidate key in both rule sets: the keys of the complex sides alone (and
        // folding into the only complex side) are a key only if each of their entities takes part
        // at most once, which n-notation can't say (README, "Keys from n-ary relationships").
        let default = first_candidate;
        let mut cur = default;
        if rm == "fold" && actions.first().is_some_and(|a| a.0 == "fold") {
            cur = 0;
        } else if rm == "table" || !chosen.is_empty() {
            cur = actions
                .iter()
                .position(|(kind, key)| {
                    *kind == "table"
                        && key
                            .iter()
                            .filter(|i| singles.contains(i))
                            .map(|i| token(*i))
                            .collect::<BTreeSet<_>>()
                            == chosen
                })
                .unwrap_or(default);
        }
        self.decide(
            &head.ruled("REL-13"),
            "Which relation holds this relationship, and what is its key?",
            opts,
            cur,
            default,
            false,
        );
        let Some((kind, key)) = actions.get(cur).cloned() else { return };
        if kind == "fold" {
            let target = side(first(&complexes)).id;
            for &s in &singles {
                let sd = side(s);
                self.fk_into(&target, &sd, r, sd.card.optional(), false, s == first(&singles), false, false);
            }
            return;
        }
        let mut unique = Vec::new();
        if cur >= first_candidate {
            for (k, _) in singles.iter().enumerate() {
                let index = first_candidate + k;
                if index != cur
                    && let Some((_, other_key)) = actions.get(index)
                {
                    unique.push(other_key.clone());
                }
            }
        }
        self.make_table(r, sides, &key, &kind_text, &unique, false);
    }

    /// (min,max) on an n-ary relationship: how often *one* entity takes part.  max = 1 makes
    /// that side's key a key of the relation; nothing can be said about combinations of sides.
    fn nary_min_max(&mut self, r: &Node, sides: &[End], head: &DecisionHead, cards: &str) {
        let lab = self.label(&r.id);
        let kind_text = format!("{}-ary {cards}", sides.len());
        if !sides.iter().all(|s| s.raw.here) {
            self.issue(
                Severity::Warning,
                format!("'{lab}' mixes (min,max) with other notations"),
                Some(&r.id),
            );
        }
        let ones: Vec<usize> = (0..sides.len())
            .filter(|i| {
                sides.get(*i).is_some_and(|s| s.raw.here && s.raw.hi.as_ref().is_some_and(|h| *h == 1))
            })
            .collect();
        self.note_bounds(r, sides, None);
        if ones.is_empty() {
            let all: Vec<usize> = (0..sides.len()).collect();
            self.make_table(r, sides, &all, &kind_text, &[], true);
            self.add_note(format!(
                "'{lab}': all foreign keys form the key, because (min,max) cannot express that some sides determine another."
            ));
            return;
        }
        let rm = self.opt_stripped(r, "rm");
        let tokens: Vec<String> = (0..sides.len()).map(|i| Self::side_token(sides, i)).collect();
        let token = |i: usize| tokens.get(i).cloned().unwrap_or_default();
        let chosen = self.opt_stripped(r, "rm key");
        let forced = self.has_key_attributes(&r.id);
        let label_of = |t: &Self, i: usize| sides.get(i).map(|s| t.label(&s.id)).unwrap_or_default();
        let mut opts = Vec::new();
        let mut actions: Vec<(&str, usize)> = Vec::new();
        for &s in &ones {
            let l = label_of(self, s);
            let t = token(s);
            opts.push(Choice::new(
                format!("Own relation, key: {l}"),
                &[("rm", Some("table")), ("rm key", Some(&t))],
                format!("Each {l} takes part at most once, so its key identifies the relationship."),
            ));
            actions.push(("table", s));
        }
        if !forced {
            for &s in &ones {
                let l = label_of(self, s);
                let t = token(s);
                opts.push(Choice::new(
                    format!("Foreign keys in {l}"),
                    &[("rm", Some("fold")), ("rm key", Some(&t))],
                    format!("Each {l} takes part at most once, so the keys of the other sides can be stored in {l}."),
                ));
                actions.push(("fold", s));
            }
        }
        let wanted = if rm.is_empty() { "table" } else { rm.as_str() };
        let cur = actions.iter().position(|(kind, s)| *kind == wanted && token(*s) == chosen).unwrap_or(0);
        self.decide(&head.ruled("REL-14"), "Which relation holds this relationship?", opts, cur, 0, false);
        let Some(&(kind, side)) = actions.get(cur) else { return };
        if kind == "table" {
            let unique: Vec<Vec<usize>> = ones.iter().filter(|o| **o != side).map(|o| vec![*o]).collect();
            self.make_table(r, sides, &[side], &kind_text, &unique, true);
            return;
        }
        let Some(target) = sides.get(side).cloned() else { return };
        let others: Vec<usize> = (0..sides.len()).filter(|o| *o != side).collect();
        for &o in &others {
            let Some(other) = sides.get(o).cloned() else { continue };
            self.fk_into(
                &target.id,
                &other,
                r,
                target.raw.lo == 0,
                false,
                Some(&o) == others.first(),
                false,
                false,
            );
        }
    }

    // -- phase E: multi-valued attributes ---------------------------------------------------

    fn phase_multivalued(&mut self) {
        let all: Vec<(String, Vec<Attr>)> = self.attrs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        for (owner, attrs) in all {
            for a in attrs {
                if !a.multivalued || a.derived {
                    continue;
                }
                let host = if self.kind(&owner) == Some("entity") {
                    self.homes(&owner).first().copied()
                } else {
                    self.rel_home.get(&owner).copied()
                };
                let ref_label = self.label(&owner);
                let Some(host) = host else {
                    let msg =
                        format!("multi-valued attribute {ref_label}.{}: no relation to refer to", a.name);
                    self.issue(Severity::Warning, msg, Some(&owner));
                    continue;
                };
                let host_rel = self.table(host).clone();
                let name = host_rel.name.clone() + &cap_first(&self.relation_ident(&a.name));
                let rel = self.add_table(Relation::new(
                    name,
                    RelationKind::Multivalued,
                    &[&owner],
                    format!("multi-valued attribute {ref_label}.{}", a.name),
                ));
                let key: Vec<&Column> = host_rel.columns.iter().filter(|c| c.pk).collect();
                if key.is_empty() {
                    let msg = format!(
                        "multi-valued attribute {ref_label}.{}: its relation has no key to refer to",
                        a.name
                    );
                    self.issue(Severity::Warning, msg, Some(&owner));
                    continue;
                }
                let names: Vec<String> = key.iter().map(|k| k.name.clone()).collect();
                for k in &key {
                    let mut c = self.column(k.name.clone(), k.ty.clone());
                    c.pk = true;
                    c.nullable = false;
                    c.guessed = k.guessed;
                    c.origin = Origin::ForeignKey;
                    if let Some(r) = self.table_mut(rel) {
                        r.columns.push(c);
                    }
                }
                if let Some(r) = self.table_mut(rel) {
                    r.fks.push(ForeignKey {
                        columns: names.clone(),
                        table: host,
                        ref_columns: names,
                        cascade: true,
                        reason: format!("multi-valued attribute {}", a.name),
                    });
                }
                let value = Attr {
                    name: a.name.clone(),
                    key: true,
                    ty: a.ty.clone(),
                    parts: a.parts.clone(),
                    ..Attr::default()
                };
                let prefix = self.column_ident(&a.name);
                for mut c in self.columns_for(&value, true) {
                    c.origin = Origin::Value;
                    self.add_column(rel, c, &prefix, false);
                }
                self.relations.push(rel);
            }
        }
    }

    // -- which subdomain box a relation belongs to --------------------------------------------

    fn assign_boxes(&mut self) {
        let doc = self.doc;
        if !doc.elements.iter().any(|e| matches!(e.body, Body::Subdomain(_))) {
            return;
        }
        let engine = DummyEngine;
        let Ok(layout) = Layout::new(doc, &engine, LabelLanguage::English).run() else { return };
        let titles = titles_by_element(doc, &layout);
        let mut order: HashMap<&str, usize> = HashMap::new();
        for (k, n) in doc.nodes().enumerate() {
            order.insert(&n.id, k);
        }
        for id in self.relations.clone() {
            let Some(rel) = self.table_mut(id) else { continue };
            let mut sources: Vec<&String> = rel.sources.iter().filter(|s| titles.contains_key(*s)).collect();
            sources.sort_by_key(|s| order.get(s.as_str()).copied().unwrap_or(0));
            rel.box_title = sources.first().and_then(|s| titles.get(*s)).cloned();
        }
    }

    // -- derived attributes with a formula ----------------------------------------------------

    /// Not generated columns: those can't use the current date, which rules out the classic
    /// derived attribute, age.
    fn phase_views(&mut self) {
        let all: Vec<(String, Vec<Attr>)> = self.attrs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        for (owner, attrs) in all {
            let derived: Vec<&Attr> = attrs
                .iter()
                .filter(|a| a.derived && a.formula.as_deref().is_some_and(|f| !f.is_empty()))
                .collect();
            if derived.is_empty() {
                continue;
            }
            let hosts: Vec<Option<TableId>> = if self.kind(&owner) == Some("entity") {
                self.homes(&owner).into_iter().map(Some).collect()
            } else {
                vec![self.rel_home.get(&owner).copied()]
            };
            for host in hosts.into_iter().flatten() {
                let columns = derived
                    .iter()
                    .map(|a| (self.column_ident(&a.name), a.formula.clone().unwrap_or_default()))
                    .collect();
                self.views.push(View {
                    name: self.table(host).name.clone() + "View",
                    relation: host,
                    columns,
                });
            }
        }
    }

    // -- final: unique relation names, empty relations ----------------------------------------

    fn finish(&mut self) {
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut unique = |name: &mut String| {
            let low = name_key(name);
            match seen.get_mut(&low) {
                Some(n) => {
                    *n += 1;
                    *name = format!("{name}{n}");
                }
                None => {
                    seen.insert(low, 1);
                }
            }
        };
        // a table and a view can't share a name
        for id in self.relations.clone() {
            if let Some(r) = self.tables.get_mut(id.0) {
                unique(&mut r.name);
            }
        }
        for v in &mut self.views {
            unique(&mut v.name);
        }
        for id in self.relations.clone() {
            let r = self.table(id);
            if r.pk().is_empty() {
                let msg = format!("relation {} has no primary key", r.name);
                self.issue(Severity::Warning, msg, None);
            }
        }
        let guessed = self
            .relations
            .iter()
            .flat_map(|id| self.table(*id).columns.iter())
            .filter(|c| c.origin != Origin::ForeignKey && c.guessed)
            .count();
        if guessed > 0 {
            self.notes.push(format!(
                "{guessed} data type(s) were guessed from attribute names (set them with 'name : type')."
            ));
        }
    }
}
