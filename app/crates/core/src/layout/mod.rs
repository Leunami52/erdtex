//! The layout engine: turns a document into drawing primitives.
//!
//! A port of what erdiagram.sty / gsmdiagram.sty compute in TeX (shape sizes, border points,
//! bends, arrow heads, attribute lists, cardinality and label placement), by way of the Python
//! app's `layout.py`, which must make the same decisions.  Text sizes come from a
//! [`TextEngine`].  Coordinates are TeX points with y pointing up (TikZ's convention).
//!
//! Errors while laying out one element become warnings on that element (as in the Python
//! app: what was drawn before the error stays).
//!
//! | Module | What it lays out |
//! |---|---|
//! | `prims` | the primitives: paths, text, handles |
//! | `text` | text boxes and how they are measured |
//! | `position` | coordinates, anchors and TikZ's positioning keys |
//! | `nodes` | nodes, IS_A / CAN_BE and the labels beside them |
//! | `edges` | edges, arrow heads, cardinalities, roles, crow's foot, GSM labels |
//! | `attributes` | attribute lists, compact and as ovals |
//! | `other` | notes, subdomains, the grid |

mod attributes;
mod edges;
mod nodes;
mod other;
mod position;
mod prims;
mod text;

use std::collections::{BTreeMap, HashMap};

pub use prims::{
    Arrow, BLACK, Gfx, Handle, LineLayout, Oval, PathPrim, Prim, Rgb, Stroke, TextLayout, TextPrim, WHITE,
};
pub use text::{DummyEngine, FontEngine, TextEngine};

use crate::document::{Document, Kind};
use crate::geometry::{self as g, Shape};
use crate::model::{Body, Element, Node, endpoint_id};
use crate::style::{LabelLanguage, Style};
use crate::text::{py_mod, round_half_even, strip};

/// What can fail while laying out an element: the message becomes a warning.
pub type Fallible<T = ()> = Result<T, String>;

/// TikZ length: `2pt`, `0.3cm`; a plain number counts as cm.
pub fn length(v: Option<&str>, default: f64) -> f64 {
    let Some(v) = v else { return default };
    let v = strip(v);
    if is_plain_number(v) {
        return v.parse::<f64>().map_or(0.0, |x| x * g::CM);
    }
    crate::latex_text::parse_length_pt(v, 10.0)
}

/// A pgf dimension: a plain number counts as pt.
pub fn plain_length(v: Option<&str>, default: f64) -> f64 {
    let Some(v) = v else { return default };
    let v = strip(v);
    if is_plain_number(v) {
        return v.parse::<f64>().unwrap_or(0.0);
    }
    crate::latex_text::parse_length_pt(v, 10.0)
}

/// `[-+]?\d*\.?\d+`
pub(crate) fn is_plain_number(v: &str) -> bool {
    let body = v.strip_prefix(['+', '-']).unwrap_or(v);
    let (int, frac) = match body.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (body, None),
    };
    let digits = |p: &str| p.bytes().all(|b| b.is_ascii_digit());
    match frac {
        None => !int.is_empty() && digits(int),
        Some(f) => digits(int) && !f.is_empty() && digits(f),
    }
}

pub const LINE_WIDTHS: &[(&str, f64)] = &[
    ("ultra thin", 0.1),
    ("very thin", 0.2),
    ("thin", 0.4),
    ("semithick", 0.6),
    ("thick", 0.8),
    ("very thick", 1.2),
    ("ultra thick", 1.6),
];

/// The compass anchor nearest to a direction (degrees).
pub fn octant_anchor(angle: f64) -> &'static str {
    const NAMES: [&str; 8] =
        ["east", "north east", "north", "north west", "west", "south west", "south", "south east"];
    let k = round_half_even(py_mod(angle, 360.0) / 45.0) as i64;
    NAMES.get(k.rem_euclid(8) as usize).copied().unwrap_or("east")
}

/// The offset of a rectangle's anchor from its centre.
pub fn anchor_offset(anchor: &str, hw: f64, hh: f64) -> (f64, f64) {
    let Some(a) = g::COMPASS.iter().find(|(n, _)| *n == anchor).map(|(_, a)| *a) else {
        return (0.0, 0.0);
    };
    let sx = if a == 90.0 || a == 270.0 {
        0.0
    } else if a == 0.0 || a == 45.0 || a == 315.0 {
        1.0
    } else {
        -1.0
    };
    let sy = if a == 0.0 || a == 180.0 {
        0.0
    } else if a == 45.0 || a == 90.0 || a == 135.0 {
        1.0
    } else {
        -1.0
    };
    (sx * hw, sy * hh)
}

/// Port of `\er@chooseside`: true = left of (vx, vy), false = right.
pub fn choose_side(req: &str, vx: f64, vy: f64) -> bool {
    let right = || {
        if vy < -0.01 {
            true
        } else if vy > 0.01 {
            false
        } else {
            vx <= 0.0
        }
    };
    let left = || {
        if vy < -0.01 {
            false
        } else if vy > 0.01 {
            true
        } else {
            vx > 0.0
        }
    };
    let req = if req == "auto" { if vx.abs() >= vy.abs() { "above" } else { "right" } } else { req };
    match req {
        "above" => {
            if vx > 0.01 {
                true
            } else if vx < -0.01 {
                false
            } else {
                right()
            }
        }
        "below" => {
            if vx > 0.01 {
                false
            } else if vx < -0.01 {
                true
            } else {
                left()
            }
        }
        "left" => left(),
        _ => right(),
    }
}

/// A cardinality to place once all edges are known.
#[derive(Clone, Debug)]
struct DeferredCard {
    gfx: usize,
    text: String,
    cp: g::Pt,
    ct: g::Pt,
    entity: String,
    edge_no: u32,
    pref: bool,
    forced: bool,
}

/// The layout of a document.
pub struct Layout<'a> {
    doc: &'a Document,
    kind: Kind,
    engine: &'a dyn TextEngine,
    pub style: Style,
    language: LabelLanguage,
    pub shapes: HashMap<String, Shape>,
    pub types: HashMap<String, String>,
    pub gfx: Vec<Gfx>,
    /// the directions in which edges and attribute lists leave each node: (edge number, degrees)
    pub angles: BTreeMap<String, Vec<(u32, f64)>>,
    edge_no: u32,
    deferred_cards: Vec<DeferredCard>,
    pub card_boxes: Vec<(f64, f64, f64, f64)>,
    deferred_constraints: Vec<(String, String, usize)>,
    deferred_conlabels: Vec<(String, String, usize)>,
    /// attribute lists drawn as ovals (element index = drawing index)
    deferred_chen: Vec<usize>,
    weak_depends: HashMap<String, String>,
    attribute_owner: HashMap<String, String>,
}

impl<'a> Layout<'a> {
    pub fn new(doc: &'a Document, engine: &'a dyn TextEngine, language: LabelLanguage) -> Self {
        let mut style = Style::new(doc.kind);
        for e in &doc.elements {
            if let Body::Setup(s) = &e.body {
                style.update(&s.opts);
            }
        }
        Layout {
            doc,
            kind: doc.kind,
            engine,
            style,
            language,
            shapes: HashMap::new(),
            types: HashMap::new(),
            gfx: Vec::new(),
            angles: BTreeMap::new(),
            edge_no: 0,
            deferred_cards: Vec::new(),
            card_boxes: Vec::new(),
            deferred_constraints: Vec::new(),
            deferred_conlabels: Vec::new(),
            deferred_chen: Vec::new(),
            weak_depends: HashMap::new(),
            attribute_owner: HashMap::new(),
        }
    }

    fn is_er(&self) -> bool {
        self.kind == Kind::Er
    }

    /// What the links say about keys: what weak entities depend on, what attributes belong to.
    /// (Port of `\er@notedep`, `\er@noteowner`.)  If a weak entity has double links, only
    /// those count: over the others it can be the owner of something else.
    fn scan_dependencies(&mut self) {
        let nodes: HashMap<&str, &Node> = self.doc.nodes().map(|n| (n.id.as_str(), n)).collect();
        let weak = |n: &Node| n.opts.contains("dependency") || n.opts.is_on("weak");
        let note = |depends: &mut HashMap<String, String>, y: &str, dep: &str| {
            if !depends.contains_key(y) {
                depends.insert(y.to_string(), dep.to_string());
            } else if dep != "E" {
                depends.insert(y.to_string(), "other".to_string());
            }
        };
        let mut by_double: HashMap<String, String> = HashMap::new();
        let mut weak_depends: HashMap<String, String> = HashMap::new();
        let mut owner: HashMap<String, String> = HashMap::new();
        let mut link = |a: &str, b: &str, double: bool| {
            for (x, y) in [(a, b), (b, a)] {
                let n = nodes.get(x);
                if let Some(n) = n
                    && n.command != "attribute"
                    && weak(n)
                {
                    let dep = strip(n.opts.get("dependency").unwrap_or(""));
                    note(&mut weak_depends, y, dep);
                    if double {
                        note(&mut by_double, y, dep);
                    }
                }
                let kind = |id: &str| nodes.get(id).map(|n| n.command.as_str());
                if kind(x) == Some("attribute") {
                    if kind(y) != Some("attribute") {
                        owner.insert(x.to_string(), y.to_string());
                    } else {
                        owner.entry(x.to_string()).or_insert_with(|| y.to_string());
                    }
                }
            }
        };
        for e in &self.doc.elements {
            match &e.body {
                Body::Node(n)
                    if n.command == "attribute" && n.opts.get("of").is_some_and(|o| !o.is_empty()) =>
                {
                    link(endpoint_id(n.opts.get("of").unwrap_or("")), &n.id, false);
                }
                Body::Edge(ed) if ed.command == "link" => {
                    link(
                        ed.src_id(),
                        ed.dst_id(),
                        ed.opts.contains("double") && ed.opts.get("double") != Some("false"),
                    );
                }
                _ => {}
            }
        }
        weak_depends.extend(by_double);
        self.weak_depends = weak_depends;
        self.attribute_owner = owner;
    }

    /// The node an attribute belongs to (through composite attributes).
    pub fn owner_of_attribute(&self, ident: &str) -> String {
        let mut ident = ident.to_string();
        for _ in 0..9 {
            match self.attribute_owner.get(&ident) {
                Some(o) => ident = o.clone(),
                None => break,
            }
        }
        ident
    }

    /// Dashed for the key of a weak entity type (unique only with its owner's key), unless it
    /// depends on E alone: then it's unique on its own.
    pub fn key_label(&self, text: &str, owner: &str) -> String {
        let partial = self.doc.nodes().find(|n| n.id == owner).is_some_and(|n| {
            n.command == "entity"
                && n.opts.is_on("weak")
                && self.weak_depends.get(owner).map(String::as_str) != Some("E")
        });
        if partial { format!("\\dashuline{{{text}}}") } else { format!("\\underline{{{text}}}") }
    }

    /// Lays out the whole document.  An error in the steps after the elements (ovals,
    /// cardinalities, labels beside shapes) ends the layout, as it did in the Python app.
    pub fn run(mut self) -> Result<Self, String> {
        self.scan_dependencies();
        let doc = self.doc;
        for (index, e) in doc.elements.iter().enumerate() {
            let mut gfx = Gfx::new(e.key, index as f64);
            if let Err(msg) = self.dispatch(e, &mut gfx) {
                gfx.warnings.push(format!("cannot draw: {msg}"));
            }
            self.gfx.push(gfx);
        }
        for index in std::mem::take(&mut self.deferred_chen) {
            let Some(Body::Attributes(tree)) = doc.elements.get(index).map(|e| &e.body) else { continue };
            self.with_gfx(index, |l, gfx| l.layout_chen(tree, index, gfx))?;
        }
        for card in std::mem::take(&mut self.deferred_cards) {
            let gi = card.gfx;
            self.with_gfx(gi, |l, gfx| {
                l.place_card(gfx, &card);
                Ok(())
            })?;
        }
        for (ident, text, gi) in std::mem::take(&mut self.deferred_constraints) {
            self.with_gfx(gi, |l, gfx| l.place_beside(&ident, &text, gfx, nodes::BesideKind::Constraint))?;
        }
        for (ident, text, gi) in std::mem::take(&mut self.deferred_conlabels) {
            self.with_gfx(gi, |l, gfx| {
                l.place_beside(&ident, &text, gfx, nodes::BesideKind::ConstructorLabel)
            })?;
        }
        Ok(self)
    }

    /// Calls `f` with the drawing of element `gi`, taken out of `self` meanwhile.
    fn with_gfx(&mut self, gi: usize, f: impl FnOnce(&mut Self, &mut Gfx) -> Fallible) -> Fallible {
        let Some(slot) = self.gfx.get_mut(gi) else { return Ok(()) };
        let mut gfx = std::mem::take(slot);
        let result = f(self, &mut gfx);
        if let Some(slot) = self.gfx.get_mut(gi) {
            *slot = gfx;
        }
        result
    }

    /// The drawing of an element.
    pub fn gfx_for(&self, key: crate::model::ElementKey) -> Option<&Gfx> {
        self.gfx.iter().find(|g| g.key == Some(key))
    }

    fn dispatch(&mut self, e: &Element, gfx: &mut Gfx) -> Fallible {
        match &e.body {
            Body::Node(n) if n.generalisation.is_some() => self.layout_isa(n, gfx),
            Body::Node(n) => self.layout_node(n, gfx, false),
            Body::Point(p) => {
                self.layout_point(p, gfx);
                Ok(())
            }
            Body::Edge(ed) => self.layout_edge(ed, gfx),
            Body::Attributes(t) => self.layout_attributes(t, self.gfx.len(), gfx),
            Body::Note(n) => self.layout_note(n, gfx),
            Body::Subdomain(s) => {
                gfx.z -= 1000.0;
                self.layout_subdomain(s, gfx);
                Ok(())
            }
            Body::Grid(gr) => {
                gfx.z -= 2000.0;
                self.layout_grid(gr, gfx);
                Ok(())
            }
            Body::Raw(r) => {
                if !matches!(r.command.as_str(), "" | "begin" | "end") {
                    gfx.warnings.push(format!("\\{} is not shown in the editor", r.command));
                }
                Ok(())
            }
            Body::Setup(_) => Ok(()),
        }
    }
}

/// Lays out a document.
pub fn layout_document<'a>(
    doc: &'a Document,
    engine: &'a dyn TextEngine,
    language: LabelLanguage,
) -> Result<Layout<'a>, String> {
    Layout::new(doc, engine, language).run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert!((length(Some("2"), 0.0) - 2.0 * g::CM).abs() < 1e-9);
        assert_eq!(plain_length(Some("2"), 0.0), 2.0);
        assert_eq!(length(None, 7.0), 7.0);
        assert_eq!(octant_anchor(-45.0), "south east");
        assert_eq!(octant_anchor(22.5), "east"); // halves go to the even neighbour
        assert!(choose_side("auto", 1.0, 0.0));
        assert!(!choose_side("below", 1.0, 0.0));
    }

    #[test]
    fn a_small_diagram() {
        let doc = Document::parse(
            "\\entity{a}(0,0){A}\\relationship{r}(4,0){R}\\entity{b}(8,0){B}\\link{a}{r}{1}\\link{r}{b}{n}\\attributes{a}{*k, x}",
            Kind::Er,
        )
        .unwrap();
        let lay = layout_document(&doc, &DummyEngine, LabelLanguage::English).unwrap();
        assert_eq!(lay.gfx.len(), 6);
        assert!(
            lay.gfx.iter().all(|g| g.warnings.is_empty()),
            "{:?}",
            lay.gfx.iter().map(|g| &g.warnings).collect::<Vec<_>>()
        );
        assert!(lay.shapes.contains_key("r"));
    }
}
