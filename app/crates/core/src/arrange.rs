//! Arranging a diagram automatically (the root README, "Arranging a diagram automatically"): the
//! diagram as a graph for the layout in the page (WebCola), the layout's result written back as the
//! positions and options the packages know, and quality figures measured on the drawing itself, by
//! which the page chooses among its candidates.
//!
//! Lengths are cm with y pointing up, as in the files. Elements are named by their keys (`k12`), since
//! notes have no ids and ids need not be unique in a broken file.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::document::{Document, Kind};
use crate::fonts::Metrics;
use crate::geometry::{CM, Pt};
use crate::layout::{FontEngine, Gfx, Handle, Layout};
use crate::model::{AttrItem, Body, Coord, ElementKey};
use crate::style::LabelLanguage;
use crate::subdomains::{SubdomainBox, boxes_of, members};
use crate::svg::bbox;
use crate::text::py_round;

/// x0, y0, x1, y1.
pub type Rect = (f64, f64, f64, f64);

/// The sides an attribute list can hang on, as dragging it around its node chooses them.
pub const SIDES: [&str; 8] =
    ["east", "north east", "north", "north west", "west", "south west", "south", "south east"];

const POSITIONING: &[&str] =
    &["right", "left", "above", "below", "above right", "above left", "below right", "below left", "at"];

/// The options that shape a line between its ends (curves, `ortho`); way points (`via`) come first.
const SHAPING: &[&str] = &["bend", "bend left", "bend right", "out", "in", "looseness", "ortho"];

static ENGINE: FontEngine = FontEngine { metrics: Metrics::DesignSizes };

fn run(doc: &Document, language: LabelLanguage) -> Option<Layout<'_>> {
    Layout::new(doc, &ENGINE, language).run().ok()
}

/// An element's name in the graph.
pub fn name(key: ElementKey) -> String {
    format!("k{}", key.0)
}

fn key_of(name: &str) -> Option<ElementKey> {
    name.strip_prefix('k')?.parse().ok().map(ElementKey)
}

fn cm(r: Rect) -> Rect {
    (r.0 / CM, r.1 / CM, r.2 / CM, r.3 / CM)
}

// -- the graph --------------------------------------------------------------------------------

/// One side an attribute list can hang on, with the box it takes there, relative to its node's centre.
#[derive(Clone, Debug, PartialEq)]
pub struct ListSide {
    pub anchor: String,
    pub rect: Rect,
}

/// An attribute list of a node.
#[derive(Clone, Debug, PartialEq)]
pub struct List {
    pub id: String,
    /// drawn as ovals: placed automatically around its node, it has no side to choose
    pub chen: bool,
    pub current: String,
    pub sides: Vec<ListSide>,
}

/// An element the layout places: a node, a junction point or a note.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: String,
    /// the command (`entity`, `relationship`, `isa`, ...), or `point`, `note`
    pub kind: String,
    /// the centre of its shape now, and the shape's size
    pub cx: f64,
    pub cy: f64,
    pub w: f64,
    pub h: f64,
    pub lists: Vec<List>,
}

/// A connection the layout keeps short. `id` is the edge's element, or empty for one that isn't an
/// edge of the file (an IS_A's arrow to its supertype, a Chen attribute's `of=`).
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub id: String,
    pub src: String,
    pub dst: String,
    pub label: String,
    /// its way points now (`via=`)
    pub via: Vec<(f64, f64)>,
    /// curved or routed by the packages (`bend=`, `ortho`): relative to its ends, so it can stay
    pub curved: bool,
}

/// A subdomain box with the elements inside it (innermost box only) and the box it lies in.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub id: String,
    pub members: Vec<String>,
    pub parent: Option<String>,
    /// the box's outline now, as a rectangle around its corners (cm)
    pub rect: Rect,
}

/// The diagram as the layout sees it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub groups: Vec<Group>,
    /// (above, below): generalisations run top to bottom, the supertype above its IS_A, the IS_A above
    /// its subtypes (in GSM, the supertype above the subtype)
    pub above: Vec<(String, String)>,
    /// connectors between boxes (see [`between`]), with the box their centre is in now: in no box,
    /// placed between their elements (or left in that box, as a candidate)
    pub between: Vec<(String, Option<String>)>,
}

/// The attribute list with its side set as dragging it does (`east` is the default and isn't written).
fn set_side(t: &mut crate::model::AttributeTree, anchor: &str) {
    if anchor == "east" {
        t.opts.remove("anchor");
    } else {
        t.opts.set("anchor", Some(anchor));
    }
    for k in ["dir", "side", "xshift", "yshift", "align"] {
        t.opts.remove(k);
    }
}

fn clear_ovals(items: &mut [AttrItem]) {
    for it in items {
        it.pos = None;
        clear_ovals(&mut it.parts);
    }
}

fn is_chen(g: &Gfx) -> bool {
    g.handles.values().any(|h| matches!(h, Handle::Ovals(_)))
}

/// The diagram as a graph for the layout.
pub fn graph(doc: &Document, language: LabelLanguage) -> Graph {
    let Some(lay) = run(doc, language) else { return Graph::default() };
    let gfx_of = |key: ElementKey| lay.gfx.iter().find(|g| g.key == Some(key));
    let mut out = Graph::default();
    let mut by_id: HashMap<String, String> = HashMap::new(); // the file's id -> the graph's name
    for el in &doc.elements {
        let (id, kind) = match &el.body {
            Body::Node(n) => (n.id.clone(), n.command.clone()),
            Body::Point(p) => (p.id.clone(), "point".to_string()),
            Body::Note(_) => {
                if let Some((x0, y0, x1, y1)) = gfx_of(el.key).and_then(bbox) {
                    out.nodes.push(Node {
                        id: name(el.key),
                        kind: "note".into(),
                        cx: (x0 + x1) / 2.0 / CM,
                        cy: (y0 + y1) / 2.0 / CM,
                        w: (x1 - x0) / CM,
                        h: (y1 - y0) / CM,
                        lists: vec![],
                    });
                }
                continue;
            }
            _ => continue,
        };
        let Some(shape) = lay.shapes.get(&id) else { continue };
        let (x0, y0, x1, y1) = shape.bbox();
        by_id.insert(id, name(el.key));
        out.nodes.push(Node {
            id: name(el.key),
            kind,
            cx: shape.cx / CM,
            cy: shape.cy / CM,
            w: (x1 - x0) / CM,
            h: (y1 - y0) / CM,
            lists: vec![],
        });
    }
    // each attribute list's box on each side: the diagram laid out once per side, all lists on it
    // (Chen lists with their ovals placed automatically, as they will be)
    let mut side_boxes: HashMap<(ElementKey, usize), Rect> = HashMap::new();
    for (si, side) in SIDES.iter().enumerate() {
        let mut d = doc.clone();
        for el in &mut d.elements {
            if let Body::Attributes(t) = &mut el.body {
                set_side(t, side);
                clear_ovals(&mut t.items);
            }
        }
        if let Some(l) = run(&d, language) {
            for el in &d.elements {
                if let Body::Attributes(t) = &el.body
                    && let (Some(g), Some(owner)) =
                        (l.gfx.iter().find(|g| g.key == Some(el.key)), l.shapes.get(&t.node))
                    && let Some((x0, y0, x1, y1)) = bbox(g)
                {
                    side_boxes.insert(
                        (el.key, si),
                        cm((x0 - owner.cx, y0 - owner.cy, x1 - owner.cx, y1 - owner.cy)),
                    );
                }
            }
        }
    }
    for el in &doc.elements {
        let Body::Attributes(t) = &el.body else { continue };
        let Some(owner) = by_id.get(&t.node) else { continue };
        let chen = gfx_of(el.key).is_some_and(is_chen);
        let current = t.opts.get("anchor").filter(|a| !a.is_empty()).unwrap_or("east").to_string();
        let sides: Vec<ListSide> = SIDES
            .iter()
            .enumerate()
            .filter(|(si, side)| !chen || **side == "east" || *si == 0)
            .filter_map(|(si, side)| {
                side_boxes.get(&(el.key, si)).map(|r| ListSide { anchor: (*side).to_string(), rect: *r })
            })
            .collect();
        if let Some(node) = out.nodes.iter_mut().find(|n| &n.id == owner) {
            node.lists.push(List { id: name(el.key), chen, current, sides });
        }
    }
    // edges; an IS_A's arrow to its supertype, a Chen attribute's `of=` as edges not in the file
    for el in &doc.elements {
        let Body::Edge(e) = &el.body else { continue };
        if let (Some(a), Some(b)) = (by_id.get(e.src_id()), by_id.get(e.dst_id()))
            && a != b
        {
            let via: Vec<(f64, f64)> = e
                .opts
                .get("via")
                .map(crate::model::parse_coord_list)
                .unwrap_or_default()
                .iter()
                .filter_map(|c| lay.coord(Some(c)))
                .map(|(x, y)| (x / CM, y / CM))
                .collect();
            let curved = SHAPING.iter().any(|k| e.opts.contains(k));
            out.edges.push(Edge {
                id: name(el.key),
                src: a.clone(),
                dst: b.clone(),
                label: e.label.clone().unwrap_or_default(),
                via,
                curved,
            });
            if doc.kind == Kind::Gsm && matches!(e.command.as_str(), "generalization" | "specialization") {
                out.above.push((b.clone(), a.clone()));
            }
        }
    }
    for n in doc.nodes() {
        let Some(me) = by_id.get(&n.id) else { continue };
        if let Some(g) = &n.generalisation
            && let Some(sup) = by_id.get(crate::model::endpoint_id(&g.supertype))
        {
            out.edges.push(Edge {
                id: String::new(),
                src: sup.clone(),
                dst: me.clone(),
                label: String::new(),
                via: vec![],
                curved: false,
            });
            out.above.push((sup.clone(), me.clone()));
            for e in doc.edges() {
                let other = if e.dst_id() == n.id {
                    e.src_id()
                } else if e.src_id() == n.id {
                    e.dst_id()
                } else {
                    continue;
                };
                if let Some(sub) = by_id.get(other)
                    && sub != sup
                {
                    out.above.push((me.clone(), sub.clone()));
                }
            }
        }
        if n.command == "attribute"
            && let Some(owner) = n.opts.get("of").and_then(|o| by_id.get(o))
        {
            out.edges.push(Edge {
                id: String::new(),
                src: owner.clone(),
                dst: me.clone(),
                label: String::new(),
                via: vec![],
                curved: false,
            });
        }
    }
    out.groups = groups(doc, &lay);
    let inner = innermost(doc, &lay, &boxes_of(doc, &lay));
    let mut free: Vec<(String, Option<String>)> =
        between(doc, &inner).into_iter().map(|k| (name(k), inner.get(&k).map(|b| name(*b)))).collect();
    free.sort();
    out.between = free;
    out
}

/// The graph's nodes the selection stands for: selected elements, the element of a selected attribute
/// list, everything inside a selected subdomain box (and the boxes in it).
pub fn selected(doc: &Document, g: &Graph, selection: &[ElementKey]) -> Vec<String> {
    let nodes: HashSet<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut boxes: Vec<String> = Vec::new();
    for &k in selection {
        match doc.get(k).map(|e| &e.body) {
            Some(Body::Attributes(t)) => {
                if let Some(owner) = doc.by_id(&t.node) {
                    out.insert(name(owner.key));
                }
            }
            Some(Body::Subdomain(_)) => boxes.push(name(k)),
            _ => {
                out.insert(name(k));
            }
        }
    }
    while let Some(b) = boxes.pop() {
        for grp in &g.groups {
            if grp.id == b {
                out.extend(grp.members.iter().cloned());
            }
            if grp.parent.as_deref() == Some(b.as_str()) {
                boxes.push(grp.id.clone());
            }
        }
    }
    out.into_iter().filter(|n| nodes.contains(n.as_str())).collect()
}

/// The commands that connect entities: a relationship, IS_A, PART_OF, CAN_BE.
const CONNECTORS: &[&str] = &["relationship", "isa", "partof", "canbe"];

/// The connectors whose elements lie in different subdomain boxes, or some in a box and some in none:
/// such a connector belongs to no box (it joins them), whichever box its centre happens to be in.
fn between(doc: &Document, inner: &BTreeMap<ElementKey, ElementKey>) -> HashSet<ElementKey> {
    let key: HashMap<&str, ElementKey> = doc
        .elements
        .iter()
        .filter_map(|el| match &el.body {
            Body::Node(n) => Some((n.id.as_str(), el.key)),
            _ => None,
        })
        .collect();
    let mut out = HashSet::new();
    for el in &doc.elements {
        let Body::Node(n) = &el.body else { continue };
        if !CONNECTORS.contains(&n.command.as_str()) {
            continue;
        }
        let mut ends: Vec<&str> = doc
            .edges()
            .filter_map(|e| {
                if e.src_id() == n.id {
                    Some(e.dst_id())
                } else if e.dst_id() == n.id {
                    Some(e.src_id())
                } else {
                    None
                }
            })
            .collect();
        if let Some(g) = &n.generalisation {
            ends.push(crate::model::endpoint_id(&g.supertype));
        }
        let boxes: HashSet<Option<ElementKey>> =
            ends.iter().filter_map(|id| key.get(id)).map(|k| inner.get(k).copied()).collect();
        if boxes.len() > 1 {
            out.insert(el.key);
        }
    }
    out
}

/// The subdomain boxes: each element in its innermost box, each box in the box it lies in.
fn groups(doc: &Document, lay: &Layout<'_>) -> Vec<Group> {
    let boxes = boxes_of(doc, lay);
    let mut innermost = innermost(doc, lay, &boxes);
    let free = between(doc, &innermost);
    innermost.retain(|k, _| !free.contains(k));
    let mut parents: HashMap<ElementKey, (ElementKey, f64)> = HashMap::new();
    for outer in &boxes {
        for inner in boxes.iter().filter(|b| b.element != outer.element && b.within(outer)) {
            let e = parents.entry(inner.element).or_insert((outer.element, outer.area));
            if outer.area < e.1 {
                *e = (outer.element, outer.area);
            }
        }
    }
    boxes
        .iter()
        .map(|b| {
            let mut m: Vec<String> =
                innermost.iter().filter(|(_, v)| **v == b.element).map(|(k, _)| name(*k)).collect();
            m.sort();
            let xs = b.corners.iter().map(|c| c.0 / CM);
            let ys = b.corners.iter().map(|c| c.1 / CM);
            let rect = (
                xs.clone().fold(f64::INFINITY, f64::min),
                ys.clone().fold(f64::INFINITY, f64::min),
                xs.fold(f64::NEG_INFINITY, f64::max),
                ys.fold(f64::NEG_INFINITY, f64::max),
            );
            Group {
                id: name(b.element),
                members: m,
                parent: parents.get(&b.element).map(|p| name(p.0)),
                rect,
            }
        })
        .collect()
}

/// For each element inside a box (not boxes themselves), its innermost box.
fn innermost(doc: &Document, lay: &Layout<'_>, boxes: &[SubdomainBox]) -> BTreeMap<ElementKey, ElementKey> {
    let mut best: BTreeMap<ElementKey, (ElementKey, f64)> = BTreeMap::new();
    for b in boxes {
        for key in members(b, doc, lay, boxes) {
            if matches!(
                doc.get(key).map(|e| &e.body),
                Some(Body::Subdomain(_)) | Some(Body::Attributes(_)) | None
            ) {
                continue;
            }
            let e = best.entry(key).or_insert((b.element, b.area));
            if b.area < e.1 {
                *e = (b.element, b.area);
            }
        }
    }
    best.into_iter().map(|(k, (b, _))| (k, b)).collect()
}

// -- writing a result back ------------------------------------------------------------------------

/// What the layout decided: shape centres, attribute list sides, the bends of edges (none: straight),
/// subdomain boxes. Elements not named keep what they have.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Arrangement {
    pub nodes: HashMap<String, (f64, f64)>,
    pub lists: HashMap<String, String>,
    pub edges: HashMap<String, Vec<(f64, f64)>>,
    pub groups: HashMap<String, Rect>,
}

fn round(v: f64) -> f64 {
    py_round((v * 20.0).round() / 20.0, 2) // to 0.05 cm: tidy numbers in the file
}

fn coord(x: f64, y: f64) -> Coord {
    Coord::numeric(round(x), round(y))
}

/// Writes an arrangement into the document: positions (relative ones made absolute), attribute list
/// sides (Chen ovals placed automatically again), edges straight or through their bends, subdomain
/// boxes as rectangles, and the helper grid fitted around the result.
pub fn apply(doc: &mut Document, a: &Arrangement, language: LabelLanguage) {
    for el in &mut doc.elements {
        let id = name(el.key);
        let mut touched = true;
        match &mut el.body {
            Body::Node(n) => match a.nodes.get(&id) {
                Some((x, y)) => {
                    for k in POSITIONING {
                        n.opts.remove(k);
                    }
                    n.opts.remove("anchor");
                    n.pos = Some(coord(*x, *y));
                }
                None => touched = false,
            },
            Body::Point(p) => match a.nodes.get(&id) {
                Some((x, y)) => p.pos = Some(coord(*x, *y)),
                None => touched = false,
            },
            Body::Note(n) => match a.nodes.get(&id) {
                Some((x, y)) => {
                    for k in POSITIONING {
                        n.opts.remove(k);
                    }
                    n.opts.remove("anchor");
                    n.pos = Some(coord(*x, *y));
                }
                None => touched = false,
            },
            Body::Edge(e) => match a.edges.get(&id) {
                Some(bends) => {
                    for k in SHAPING.iter().chain(&["via"]) {
                        e.opts.remove(k);
                    }
                    if !bends.is_empty() {
                        let pts: Vec<String> = bends.iter().map(|(x, y)| coord(*x, *y).to_tex()).collect();
                        e.opts.set("via", Some(&format!("{{{}}}", pts.join(" "))));
                    }
                }
                None => touched = false,
            },
            Body::Attributes(t) => match a.lists.get(&id) {
                Some(side) => {
                    clear_ovals(&mut t.items);
                    if SIDES.contains(&side.as_str()) {
                        set_side(t, side);
                    }
                }
                None => touched = false,
            },
            Body::Subdomain(s) => match a.groups.get(&id) {
                // top left first, as the files have it: the title goes in the first corner
                Some((x0, y0, x1, y1)) => s.points = vec![coord(*x0, *y1), coord(*x1, *y0)],
                None => touched = false,
            },
            _ => touched = false,
        }
        if touched {
            el.touch();
        }
    }
    fit_grid(doc, language);
}

/// The helper grid (`\grid`) around the diagram again, on whole cm.
fn fit_grid(doc: &mut Document, language: LabelLanguage) {
    let Some(grid_key) = doc.elements.iter().find(|e| matches!(e.body, Body::Grid(_))).map(|e| e.key) else {
        return;
    };
    let Some(lay) = run(doc, language) else { return };
    let mut all: Option<Rect> = None;
    for g in lay.gfx.iter().filter(|g| g.key != Some(grid_key)) {
        if let Some(r) = bbox(g) {
            all = Some(all.map_or(r, |a| (a.0.min(r.0), a.1.min(r.1), a.2.max(r.2), a.3.max(r.3))));
        }
    }
    let Some((x0, y0, x1, y1)) = all.map(cm) else { return };
    if let Some(el) = doc.get_mut(grid_key)
        && let Body::Grid(g) = &mut el.body
    {
        g.a = Coord::numeric(x0.floor(), y0.floor());
        g.b = Coord::numeric(x1.ceil(), y1.ceil());
        el.touch();
    }
}

// -- quality figures ---------------------------------------------------------------------------

/// How good an arrangement is, measured on the drawing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Figures {
    /// pairs of elements whose boxes overlap (an attribute list counts as an element; a node and its
    /// own list don't count)
    pub overlaps: usize,
    /// (line, element) pairs where a line runs through an element's box (also its own end's list)
    pub through: usize,
    /// pairs of lines that cross (lines meeting at an element don't count)
    pub crossings: usize,
    /// pairs of lines that run on top of one another, or so close side by side that they can hardly
    /// be told apart (`close_length` cm in all; near an element both meet at doesn't count), or where
    /// a bend of one lies on the other (they seem to join there)
    pub close: usize,
    pub close_length: f64,
    /// the length of all lines (cm)
    pub length: f64,
    /// the mean gap between neighbouring elements (cm), and how uneven the gaps are (deviation / mean)
    pub gap: f64,
    pub unevenness: f64,
    /// the size of the drawing (cm)
    pub width: f64,
    pub height: f64,
    /// elements whose subdomain box changed (compared with the document before)
    pub moved_out: usize,
    /// pairs of subdomain boxes that overlap, neither inside the other
    pub box_overlaps: usize,
}

impl Figures {
    /// One number to choose by: overlaps and elements leaving their box first, then lines through
    /// elements and crossings, then length and size.
    pub fn score(&self) -> f64 {
        1000.0 * (self.overlaps + self.moved_out + self.box_overlaps) as f64
            + 400.0 * self.close as f64
            + 30.0 * self.close_length
            + 150.0 * self.through as f64
            + 50.0 * self.crossings as f64
            + self.length
            + 0.05 * self.width * self.height
    }
}

struct Item {
    owner: String,
    list: bool,
    rect: Rect,
}

/// Whether the segment p-q passes through the rectangle (Liang-Barsky).
fn clips(p: Pt, q: Pt, r: &Rect) -> bool {
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    for (num_p, num_q) in [(-dx, p.0 - r.0), (dx, r.2 - p.0), (-dy, p.1 - r.1), (dy, r.3 - p.1)] {
        if num_p == 0.0 {
            if num_q < 0.0 {
                return false;
            }
            continue;
        }
        let t = num_q / num_p;
        if num_p < 0.0 {
            if t > t1 {
                return false;
            }
            t0 = t0.max(t);
        } else {
            if t < t0 {
                return false;
            }
            t1 = t1.min(t);
        }
    }
    t0 < t1
}

fn crosses(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
    let side = |p: Pt, q: Pt, r: Pt| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0
}

/// A drawn line: the elements at its ends, and its straight pieces.
type Line = (HashSet<String>, Vec<(Pt, Pt)>);

/// Lines closer than this, in nearly the same direction, can't be told apart.
const CLOSE: f64 = 0.2 * CM;

fn distance(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

/// Whether a bend of line `a` (where two of its pieces meet at an angle) lies on or next to line `b`.
fn bend_on(a: &[(Pt, Pt)], b: &[(Pt, Pt)]) -> bool {
    a.iter().zip(a.iter().skip(1)).any(|(&(p, q), &(_, r))| {
        let (u, v) = ((q.0 - p.0, q.1 - p.1), (r.0 - q.0, r.1 - q.1));
        let (lu, lv) = (u.0.hypot(u.1), v.0.hypot(v.1));
        // more than 25° (a curve, drawn in eight pieces, turns less at each)
        let bent = lu > 1e-6 && lv > 1e-6 && (u.0 * v.0 + u.1 * v.1) / (lu * lv) < 0.906;
        bent && b.iter().any(|(c, d)| distance(q, *c, *d) < 2.0 * CLOSE)
    })
}

/// How long (pt) line `a` runs close beside line `b` in nearly the same direction (within 20°),
/// sampled every 0.05 cm; `skip`: points near the ends both lines share.
fn close_beside(a: &[(Pt, Pt)], b: &[(Pt, Pt)], skip: &[Pt]) -> f64 {
    let step = 0.05 * CM;
    let mut along = 0.0;
    for (p, q) in a {
        let len = (q.0 - p.0).hypot(q.1 - p.1);
        if len < 1e-9 {
            continue;
        }
        let (ux, uy) = ((q.0 - p.0) / len, (q.1 - p.1) / len);
        let n = (len / step).ceil() as usize;
        for k in 0..n {
            let t = (k as f64 + 0.5) / n as f64;
            let s = (p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1));
            if skip.iter().any(|e| (s.0 - e.0).hypot(s.1 - e.1) < 0.6 * CM) {
                continue;
            }
            let near = b.iter().any(|(c, d)| {
                let l = (d.0 - c.0).hypot(d.1 - c.1);
                l > 1e-9 && distance(s, *c, *d) < CLOSE && {
                    let cos = (ux * (d.0 - c.0) + uy * (d.1 - c.1)) / l;
                    cos.abs() > 0.94
                }
            });
            if near {
                along += len / n as f64;
            }
        }
    }
    along
}

/// A drawn edge as straight pieces (curves in eight).
fn pieces(g: &Gfx) -> Vec<(Pt, Pt)> {
    let mut out = Vec::new();
    for s in g.segments.iter().chain(g.stub.iter()) {
        match s.controls {
            None => out.push((s.p0, s.p3)),
            Some((c1, c2)) => {
                let at = |t: f64| {
                    let u = 1.0 - t;
                    (
                        u * u * u * s.p0.0
                            + 3.0 * u * u * t * c1.0
                            + 3.0 * u * t * t * c2.0
                            + t * t * t * s.p3.0,
                        u * u * u * s.p0.1
                            + 3.0 * u * u * t * c1.1
                            + 3.0 * u * t * t * c2.1
                            + t * t * t * s.p3.1,
                    )
                };
                let mut prev = s.p0;
                for k in 1..=8 {
                    let p = at(f64::from(k) / 8.0);
                    out.push((prev, p));
                    prev = p;
                }
            }
        }
    }
    out
}

/// The quality figures of a document; with `before`, also the elements whose box changed.
pub fn figures(doc: &Document, language: LabelLanguage, before: Option<&Document>) -> Figures {
    let Some(lay) = run(doc, language) else { return Figures::default() };
    let mut f = Figures::default();
    let mut items: Vec<Item> = Vec::new();
    for el in &doc.elements {
        let Some(g) = lay.gfx.iter().find(|g| g.key == Some(el.key)) else { continue };
        match &el.body {
            Body::Node(n) => {
                if let Some(s) = lay.shapes.get(&n.id) {
                    items.push(Item { owner: n.id.clone(), list: false, rect: s.bbox() });
                }
            }
            Body::Attributes(t) => {
                if let Some(r) = bbox(g) {
                    items.push(Item { owner: t.node.clone(), list: true, rect: r });
                }
            }
            Body::Note(_) => {
                if let Some(r) = bbox(g) {
                    items.push(Item { owner: name(el.key), list: false, rect: r });
                }
            }
            _ => {}
        }
    }
    let tiny = 0.1 * CM;
    for (i, a) in items.iter().enumerate() {
        for b in items.iter().skip(i + 1) {
            if a.owner == b.owner && (a.list || b.list) {
                continue; // a node and its own list
            }
            let w = a.rect.2.min(b.rect.2) - a.rect.0.max(b.rect.0);
            let h = a.rect.3.min(b.rect.3) - a.rect.1.max(b.rect.1);
            if w > tiny && h > tiny {
                f.overlaps += 1;
            }
        }
    }
    // lines: their pieces, and the elements at their ends
    let mut lines: Vec<Line> = Vec::new();
    for el in &doc.elements {
        if let Body::Edge(e) = &el.body
            && let Some(g) = lay.gfx.iter().find(|g| g.key == Some(el.key))
        {
            let ends: HashSet<String> =
                [e.src_id().to_string(), e.dst_id().to_string()].into_iter().collect();
            lines.push((ends, pieces(g)));
        }
    }
    let pt = 1.0;
    for (ends, ps) in &lines {
        f.length += ps.iter().map(|(a, b)| (b.0 - a.0).hypot(b.1 - a.1)).sum::<f64>() / CM;
        for it in &items {
            if !it.list && ends.contains(&it.owner) {
                continue;
            }
            let r = (it.rect.0 + pt, it.rect.1 + pt, it.rect.2 - pt, it.rect.3 - pt);
            if r.0 < r.2 && r.1 < r.3 && ps.iter().any(|(a, b)| clips(*a, *b, &r)) {
                f.through += 1;
            }
        }
    }
    for (i, (ea, pa)) in lines.iter().enumerate() {
        for (eb, pb) in lines.iter().skip(i + 1) {
            // ends both lines share: where each of them starts or ends there
            let ends = |ps: &[(Pt, Pt)]| [ps.first().map(|s| s.0), ps.last().map(|s| s.1)];
            let skip: Vec<Pt> = if ea.is_disjoint(eb) {
                vec![]
            } else {
                ends(pa).into_iter().chain(ends(pb)).flatten().collect()
            };
            let together = close_beside(pa, pb, &skip).min(close_beside(pb, pa, &skip));
            if together > 0.3 * CM {
                f.close += 1;
                f.close_length += together / CM;
            } else if ea.is_disjoint(eb) && (bend_on(pa, pb) || bend_on(pb, pa)) {
                f.close += 1;
            }
            if !ea.is_disjoint(eb) {
                continue;
            }
            let n = pa
                .iter()
                .map(|(a, b)| pb.iter().filter(|(c, d)| crosses(*a, *b, *c, *d)).count())
                .sum::<usize>();
            f.crossings += n;
        }
    }
    // gaps between elements (a node together with its lists)
    let mut whole: BTreeMap<&str, Rect> = BTreeMap::new();
    for it in &items {
        let e = whole.entry(it.owner.as_str()).or_insert(it.rect);
        *e = (e.0.min(it.rect.0), e.1.min(it.rect.1), e.2.max(it.rect.2), e.3.max(it.rect.3));
    }
    let rects: Vec<Rect> = whole.values().copied().collect();
    let gaps: Vec<f64> = rects
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            rects
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, b)| {
                    let dx = (a.0 - b.2).max(b.0 - a.2).max(0.0);
                    let dy = (a.1 - b.3).max(b.1 - a.3).max(0.0);
                    dx.hypot(dy) / CM
                })
                .reduce(f64::min)
        })
        .collect();
    if !gaps.is_empty() {
        let n = gaps.len() as f64;
        f.gap = gaps.iter().sum::<f64>() / n;
        let sd = (gaps.iter().map(|g| (g - f.gap).powi(2)).sum::<f64>() / n).sqrt();
        f.unevenness = if f.gap > 0.0 { sd / f.gap } else { 0.0 };
    }
    if let Some(all) =
        rects.iter().copied().reduce(|a, r| (a.0.min(r.0), a.1.min(r.1), a.2.max(r.2), a.3.max(r.3)))
    {
        f.width = (all.2 - all.0) / CM;
        f.height = (all.3 - all.1) / CM;
    }
    // boxes overlapping: their outlines' rectangles, for boxes neither of which lies in the other
    let boxes = boxes_of(doc, &lay);
    let outline = |b: &SubdomainBox| {
        b.corners.iter().fold((f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY), |r, c| {
            (r.0.min(c.0), r.1.min(c.1), r.2.max(c.0), r.3.max(c.1))
        })
    };
    for (i, a) in boxes.iter().enumerate() {
        for b in boxes.iter().skip(i + 1) {
            if a.within(b) || b.within(a) {
                continue;
            }
            let (ra, rb) = (outline(a), outline(b));
            let w = ra.2.min(rb.2) - ra.0.max(rb.0);
            let h = ra.3.min(rb.3) - ra.1.max(rb.1);
            if w > tiny && h > tiny {
                f.box_overlaps += 1;
            }
        }
    }
    if let Some(before) = before
        && let Some(lb) = run(before, language)
    {
        // connectors between boxes belong to none: wherever they go, they don't leave one
        let mut was = innermost(before, &lb, &boxes_of(before, &lb));
        let free = between(before, &was);
        was.retain(|k, _| !free.contains(k));
        let mut now = innermost(doc, &lay, &boxes_of(doc, &lay));
        now.retain(|k, _| !free.contains(k));
        f.moved_out = was.iter().filter(|(k, b)| now.get(k) != Some(b)).count()
            + now.keys().filter(|k| !was.contains_key(k)).count();
    }
    f
}

/// The figures of the document as it would be with the arrangement written into it.
pub fn try_arrangement(doc: &Document, a: &Arrangement, language: LabelLanguage) -> Figures {
    let mut d = doc.clone();
    apply(&mut d, a, language);
    figures(&d, language, Some(doc))
}

/// Parses an element name of the graph back into its key (for the page's results).
pub fn element(name: &str) -> Option<ElementKey> {
    key_of(name)
}

/// The graph, arrangements and figures as JSON, for the page (`--features json`).
#[cfg(feature = "json")]
pub mod json {
    use serde_json::{Value, json};

    use super::{Arrangement, Figures, Graph, Rect};

    fn rect(r: &Rect) -> Value {
        json!([r.0, r.1, r.2, r.3])
    }

    /// The graph for `lib/arrange.ts`.
    pub fn graph(g: &Graph) -> String {
        graph_with(g, &[])
    }

    /// The graph, with the nodes the selection stands for (`selected`).
    pub fn graph_with(g: &Graph, selected: &[String]) -> String {
        let nodes: Vec<Value> = g
            .nodes
            .iter()
            .map(|n| {
                let lists: Vec<Value> = n
                    .lists
                    .iter()
                    .map(|l| {
                        let sides: Vec<Value> =
                            l.sides.iter().map(|s| json!({ "anchor": s.anchor, "rect": rect(&s.rect) })).collect();
                        json!({ "id": l.id, "chen": l.chen, "current": l.current, "sides": sides })
                    })
                    .collect();
                json!({ "id": n.id, "kind": n.kind, "cx": n.cx, "cy": n.cy, "w": n.w, "h": n.h, "lists": lists })
            })
            .collect();
        let edges: Vec<Value> = g
            .edges
            .iter()
            .map(|e| {
                let via: Vec<Value> = e.via.iter().map(|(x, y)| json!([x, y])).collect();
                json!({ "id": e.id, "src": e.src, "dst": e.dst, "label": e.label, "via": via, "curved": e.curved })
            })
            .collect();
        let groups: Vec<Value> = g
            .groups
            .iter()
            .map(|x| json!({ "id": x.id, "members": x.members, "parent": x.parent, "rect": rect(&x.rect) }))
            .collect();
        let above: Vec<Value> = g.above.iter().map(|(a, b)| json!([a, b])).collect();
        let between: Vec<Value> = g.between.iter().map(|(n, b)| json!([n, b])).collect();
        json!({ "nodes": nodes, "edges": edges, "groups": groups, "above": above, "between": between,
                "selected": selected })
        .to_string()
    }

    fn pair(v: &Value) -> Option<(f64, f64)> {
        Some((v.get(0)?.as_f64()?, v.get(1)?.as_f64()?))
    }

    /// An arrangement from `lib/arrange.ts`; what can't be read is left out.
    pub fn arrangement(text: &str) -> Result<Arrangement, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let mut a = Arrangement::default();
        let obj = |k: &str| v.get(k).and_then(Value::as_object).cloned().unwrap_or_default();
        for (k, p) in obj("nodes") {
            if let Some(p) = pair(&p) {
                a.nodes.insert(k, p);
            }
        }
        for (k, s) in obj("lists") {
            if let Some(s) = s.as_str() {
                a.lists.insert(k, s.to_string());
            }
        }
        for (k, bends) in obj("edges") {
            let pts: Vec<(f64, f64)> =
                bends.as_array().map(|b| b.iter().filter_map(pair).collect()).unwrap_or_default();
            a.edges.insert(k, pts);
        }
        for (k, r) in obj("groups") {
            if let (Some(x0), Some(y0), Some(x1), Some(y1)) = (
                r.get(0).and_then(Value::as_f64),
                r.get(1).and_then(Value::as_f64),
                r.get(2).and_then(Value::as_f64),
                r.get(3).and_then(Value::as_f64),
            ) {
                a.groups.insert(k, (x0, y0, x1, y1));
            }
        }
        Ok(a)
    }

    /// The figures, with their score.
    pub fn figures(f: &Figures) -> String {
        json!({ "overlaps": f.overlaps, "through": f.through, "crossings": f.crossings, "length": f.length,
                "close": f.close, "close_length": f.close_length,
                "gap": f.gap, "unevenness": f.unevenness, "width": f.width, "height": f.height,
                "moved_out": f.moved_out, "box_overlaps": f.box_overlaps, "score": f.score() })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;
    use crate::session::Session;

    const MANYDOGS: &str = include_str!("../../../../app/test_diagrams/imkerei.erd");
    const GLOBAL: &str = include_str!("../../../../app/test_diagrams/ensemble.gsmd");

    fn doc(text: &str, kind: Kind) -> Document {
        Document::parse(text, kind).unwrap()
    }

    #[test]
    fn the_graph_has_every_element_with_its_lists_and_lines() {
        let g = graph(&doc(MANYDOGS, Kind::Er), LabelLanguage::English);
        assert!(g.nodes.len() > 10);
        let ids: HashSet<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(g.edges.iter().all(|e| ids.contains(e.src.as_str()) && ids.contains(e.dst.as_str())));
        let lists: Vec<&List> = g.nodes.iter().flat_map(|n| &n.lists).filter(|l| !l.chen).collect();
        assert!(!lists.is_empty());
        assert!(lists.iter().all(|l| l.sides.len() == SIDES.len()));
        // the IS_A: the supertype above it, it above its subtype
        assert!(g.above.len() >= 2);
        assert!(g.above.iter().all(|(a, b)| ids.contains(a.as_str()) && ids.contains(b.as_str())));
    }

    #[test]
    fn the_figures_count_overlaps_lines_through_elements_and_crossings() {
        let f = figures(&doc(MANYDOGS, Kind::Er), LabelLanguage::English, None);
        assert_eq!((f.overlaps, f.through, f.crossings), (2, 1, 0));
        assert!(f.score() > 2000.0);
        let f = figures(&doc(GLOBAL, Kind::Gsm), LabelLanguage::English, None);
        assert_eq!((f.overlaps, f.through, f.crossings), (0, 0, 0));
    }

    /// Every node moved 2 cm to the right, nothing else changed.
    fn shifted(d: &Document) -> Arrangement {
        let g = graph(d, LabelLanguage::English);
        Arrangement {
            nodes: g.nodes.iter().map(|n| (n.id.clone(), (n.cx + 2.0, n.cy))).collect(),
            lists: g.nodes.iter().flat_map(|n| &n.lists).map(|l| (l.id.clone(), l.current.clone())).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn trying_an_arrangement_changes_nothing() {
        let d = doc(MANYDOGS, Kind::Er);
        let before = d.to_text();
        let f = try_arrangement(&d, &shifted(&d), LabelLanguage::English);
        assert_eq!(d.to_text(), before);
        // moving everything alike keeps the figures that don't depend on the place
        let f0 = figures(&d, LabelLanguage::English, None);
        assert_eq!((f.overlaps, f.crossings, f.moved_out), (f0.overlaps, f0.crossings, 0));
    }

    #[test]
    fn an_arrangement_is_one_step_that_can_be_undone() {
        let mut ed = Editor::new(Session::open(GLOBAL, Kind::Gsm).unwrap());
        let before = ed.doc().to_text();
        let a = shifted(ed.doc());
        assert!(ed.arrange(&a));
        assert_ne!(ed.doc().to_text(), before);
        assert!(ed.undo());
        assert_eq!(ed.doc().to_text(), before);
    }

    /// Two lines with way points, their middle pieces `apart` cm from each other.
    fn two_lines(apart: f64) -> String {
        format!(
            "\\entity{{a}}(0,0){{A}}\n\\relationship{{r}}(10,0){{r}}\n\\entity{{b}}(0,-6){{B}}\n\\relationship{{s}}(10,-6){{s}}\n\\link[via={{{{(2,-3) (8,-3)}}}}]{{a}}{{r}}{{1}}\n\\link[via={{{{(2,{y}) (8,{y})}}}}]{{b}}{{s}}{{1}}\n",
            y = -3.0 - apart
        )
    }

    #[test]
    fn lines_on_top_of_one_another_are_counted() {
        let f = figures(&doc(&two_lines(0.1), Kind::Er), LabelLanguage::English, None);
        assert_eq!(f.close, 1);
        assert!(f.close_length > 5.0, "{}", f.close_length);
        let f = figures(&doc(&two_lines(1.0), Kind::Er), LabelLanguage::English, None);
        assert_eq!((f.close, f.close_length), (0, 0.0));
        // a bend of one line on the other: they seem to join there
        let touching = "\\entity{a}(0,0){A}\n\\relationship{r}(10,0){r}\n\\entity{b}(5,-6){B}\n\\relationship{s}(5,4){s}\n\\link[via={{(5,-3)}}]{a}{r}{1}\n\\link{b}{s}{1}\n";
        let f = figures(&doc(touching, Kind::Er), LabelLanguage::English, None);
        assert_eq!(f.close, 1);
    }

    /// Two boxes, an entity in each, and a relationship joining them inside the left box.
    const TWO_BOXES: &str = "\\entity{a}(0,0){A}\n\\entity{b}(10,0){B}\n\\relationship{r}(2,-1.5){r}\n\\link{a}{r}{1}\n\\link{r}{b}{n}\n\\entity{c}(0,-3){C}\n\\subdomain{Left}{(-2,2) (4,-4)}\n\\subdomain{Right}{(8,2) (12,-2)}\n";

    #[test]
    fn a_connector_between_boxes_belongs_to_neither() {
        let d = doc(TWO_BOXES, Kind::Er);
        let g = graph(&d, LabelLanguage::English);
        let r = g.nodes.iter().find(|n| n.kind == "relationship").unwrap().id.clone();
        let left = g.groups.iter().find(|x| x.members.len() == 2).unwrap();
        assert_eq!(g.between, vec![(r.clone(), Some(left.id.clone()))]);
        assert!(g.groups.iter().all(|x| !x.members.contains(&r)));
        // moving it out of the left box isn't leaving a box; moving C out is
        let at = |id: &str, x: f64, y: f64| (id.to_string(), (x, y));
        let c = g.nodes.iter().find(|n| (n.cy + 3.0).abs() < 1e-6).unwrap().id.clone();
        let mut a = Arrangement { nodes: [at(&r, 6.0, 0.0)].into_iter().collect(), ..Default::default() };
        assert_eq!(try_arrangement(&d, &a, LabelLanguage::English).moved_out, 0);
        a.nodes.insert(c, (6.0, -3.0));
        assert_eq!(try_arrangement(&d, &a, LabelLanguage::English).moved_out, 1);
    }

    #[test]
    fn the_selection_stands_for_elements() {
        let text = format!("{TWO_BOXES}\\attributes{{c}}{{x}}\n");
        let d = doc(&text, Kind::Er);
        let g = graph(&d, LabelLanguage::English);
        let key = |want: fn(&Body) -> bool| d.elements.iter().find(|e| want(&e.body)).unwrap().key;
        let list = key(|b| matches!(b, Body::Attributes(_)));
        let left = d
            .elements
            .iter()
            .find(|e| matches!(&e.body, Body::Subdomain(s) if s.label == "Left"))
            .unwrap()
            .key;
        let c = d.by_id("c").unwrap().key;
        let a = d.by_id("a").unwrap().key;
        // a list: its element; a box: what is inside it (not the relationship between the boxes)
        assert_eq!(selected(&d, &g, &[list]), vec![name(c)]);
        let mut inside = vec![name(a), name(c)];
        inside.sort();
        assert_eq!(selected(&d, &g, &[left]), inside);
    }

    #[test]
    fn overlapping_boxes_are_counted_nested_ones_not() {
        let f = |boxes: &str| {
            let text = format!("\\entity{{a}}(0,0){{A}}\n{boxes}");
            figures(&doc(&text, Kind::Er), LabelLanguage::English, None).box_overlaps
        };
        assert_eq!(f("\\subdomain{P}{(-2,2) (2,-2)}\n\\subdomain{Q}{(1,1) (5,-3)}\n"), 1);
        assert_eq!(f("\\subdomain{P}{(-4,4) (4,-4)}\n\\subdomain{Q}{(-1,1) (1,-1)}\n"), 0);
        assert_eq!(f("\\subdomain{P}{(-2,2) (2,-2)}\n\\subdomain{Q}{(3,2) (5,-2)}\n"), 0);
    }

    #[test]
    fn a_box_keeps_its_title_at_the_top_left() {
        let text = "\\entity{a}(0,0){A}\n\\subdomain{Box}{(-2,2) (2,-2)}\n";
        let mut d = doc(text, Kind::Er);
        let g = graph(&d, LabelLanguage::English);
        let a = Arrangement {
            groups: [(g.groups[0].id.clone(), (-3.0, -1.0, 3.0, 4.0))].into_iter().collect(),
            ..Default::default()
        };
        apply(&mut d, &a, LabelLanguage::English);
        assert!(d.to_text().contains("{(-3,4) (3,-1)}"), "{}", d.to_text());
    }

    #[test]
    fn a_line_arranged_anew_loses_its_curve() {
        let text = "\\entity{a}(0,0){A}\n\\relationship{r}(10,0){r}\n\\link[bend right=30]{a}{r}{1}\n";
        let mut d = doc(text, Kind::Er);
        let g = graph(&d, LabelLanguage::English);
        assert!(g.edges[0].curved);
        let a = Arrangement {
            edges: [(g.edges[0].id.clone(), vec![(5.0, 2.0)])].into_iter().collect(),
            ..Default::default()
        };
        apply(&mut d, &a, LabelLanguage::English);
        let out = d.to_text();
        assert!(out.contains("via={{(5,2)}}") && !out.contains("bend"), "{out}");
    }

    #[cfg(feature = "json")]
    #[test]
    fn arrangements_come_as_json() {
        let a = json::arrangement(
            r#"{"nodes":{"k1":[1.5,2]},"lists":{"k2":"north"},"edges":{"k3":[[0,0],[1,1]]},"groups":{}}"#,
        )
        .unwrap();
        assert_eq!(a.nodes["k1"], (1.5, 2.0));
        assert_eq!(a.lists["k2"], "north");
        assert_eq!(a.edges["k3"], vec![(0.0, 0.0), (1.0, 1.0)]);
        assert!(json::arrangement("not json").is_err());
        let g = json::graph(&graph(&doc(MANYDOGS, Kind::Er), LabelLanguage::English));
        assert!(g.contains("\"above\"") && g.contains("\"sides\""));
    }
}
