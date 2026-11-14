//! Interactive editing: tools, selection, dragging, connecting, handles, the clipboard.  The
//! port of the Python app's `editor.py`, without the window: the page reports pointer and key
//! events in diagram coordinates (TeX points, y up), the editor changes the document through
//! its [`Session`], and draws what goes on top of the diagram ([`Editor::overlay`]).  What the
//! window should show or say comes back as [`Event`]s.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use crate::document::{Document, Kind, kind_of_text};
use crate::geometry::{CM, PathOp, Pt, Segment, angle_of};
use crate::history::SelKey;
use crate::latex_text::plain_text;
use crate::layout::{Gfx, Handle};
use crate::model::{
    AttrItem, AttributeTree, Body, Coord, Edge, ElementKey, Generalisation, Node, Note, Options, Point,
    Subdomain, endpoint_id, parse_coord_list,
};
use crate::rules::{
    is_double, make_double, make_single, may_be_double, refusal, should_be_double, weak_links,
};
use crate::session::{Laid, Session};
use crate::subdomains::{boxes_of, members};
use crate::text::{py_round, round_half_even};

/// The first line of what's copied: an element such as a note exists in both kinds of
/// diagram.  As a comment it stays valid source, also when pasted into a text editor.
pub const KIND_MARK: &str = "% ERDTeX, from a diagram of kind: ";

/// A tool that places a node: (tool, command, default label, options).
pub type NodeTool =
    (&'static str, &'static str, Option<&'static str>, &'static [(&'static str, Option<&'static str>)]);

/// The tools that place nodes.
pub fn node_tools(kind: Kind) -> &'static [NodeTool] {
    match kind {
        Kind::Er => &[
            ("entity", "entity", Some("Entity"), &[]),
            ("weak entity", "entity", Some("Weak entity"), &[("weak", None)]),
            ("relationship", "relationship", Some("relates"), &[]),
            ("weak relationship", "relationship", Some("has"), &[("weak", None)]),
            ("existence dependency", "relationship", Some("has"), &[("dependency", Some("E"))]),
            ("identification dependency", "relationship", Some("has"), &[("dependency", Some("Id"))]),
            ("part of", "partof", None, &[]),
            ("chen attribute", "attribute", Some("attribute"), &[]),
        ],
        Kind::Gsm => &[
            ("abstract type", "abstract", Some("Abstract"), &[]),
            ("representable type", "representable", Some("Value"), &[]),
            ("subtype", "subtype", Some("Subtype"), &[]),
            ("aggregation", "aggregation", Some("Aggregate"), &[]),
            ("grouping", "grouping", Some("Group"), &[]),
        ],
    }
}

/// The tools that connect: (tool, command).
pub fn edge_tools(kind: Kind) -> &'static [(&'static str, &'static str)] {
    match kind {
        Kind::Er => &[("link", "link")],
        Kind::Gsm => &[
            ("attribute", "attribute"),
            ("multi-valued attribute", "multiattribute"),
            ("component", "component"),
            ("generalization", "generalization"),
            ("specialization", "specialization"),
            ("link", "link"),
        ],
    }
}

/// The other tools.
pub fn special_tools(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::Er => &["is a", "can be", "attribute tree", "point", "note", "subdomain"],
        Kind::Gsm => &["point", "note", "subdomain"],
    }
}

/// The palette: sections with their tools.
pub fn palette(kind: Kind) -> Vec<(&'static str, Vec<&'static str>)> {
    let mut elements: Vec<&str> = node_tools(kind).iter().map(|t| t.0).collect();
    if kind == Kind::Er {
        elements.extend(["is a", "can be"]);
    }
    vec![
        ("Select", vec!["select"]),
        ("Elements", elements),
        ("Connections", edge_tools(kind).iter().map(|t| t.0).collect()),
        ("More", special_tools(kind).iter().copied().filter(|t| *t != "is a" && *t != "can be").collect()),
    ]
}

/// What a tool does, for its tooltip.
pub fn tip(tool: &str) -> &'static str {
    match tool {
        "select" => "Select, move and reshape",
        "entity" => "Entity type",
        "weak entity" => "Weak entity type",
        "relationship" => "Relationship type",
        "weak relationship" => "Weak relationship type",
        "existence dependency" => {
            "Weak relationship with existence dependency (E): the weak entity's own key is unique everywhere"
        }
        "identification dependency" => {
            "Weak relationship with identification dependency (Id), which includes existence dependency: the weak entity's key is only unique together with its owner's"
        }
        "part of" => "Aggregation (PART_OF)",
        "chen attribute" => "Chen-style attribute (click on its node)",
        "is a" => "Generalisation IS_A (click on the supertype)",
        "can be" => "Specialisation CAN_BE (click on the supertype)",
        "attribute tree" => "Attribute list (click on an entity or relationship)",
        "point" => "Junction point",
        "note" => "Note",
        "subdomain" => "Subdomain (drag a rectangle)",
        "link" => "Line (drag from one element to another)",
        "abstract type" => "Abstract type",
        "representable type" => "Representable type",
        "subtype" => "Subtype",
        "aggregation" => "Aggregation",
        "grouping" => "Grouping",
        "attribute" => "Single-valued attribute (drag)",
        "multi-valued attribute" => "Multi-valued attribute (drag)",
        "component" => "Element of a constructed type (drag from aggregation/grouping)",
        "generalization" => "Generalization: disjoint IS-A (drag from subtype)",
        "specialization" => "Specialization: overlapping IS-A (drag from subtype)",
        _ => "",
    }
}

const POSITIONING: &[&str] =
    &["right", "left", "above", "below", "above right", "above left", "below right", "below left", "at"];

/// Ids the editor generates (`entity3`): a label typed later gives a better one.
pub fn is_default_id(id: &str) -> bool {
    const BASES: &[&str] = &[
        "entity",
        "weak",
        "relates",
        "has",
        "partof",
        "attribute",
        "isa",
        "canbe",
        "abstract",
        "value",
        "subtype",
        "aggregate",
        "group",
        "point",
        "attr",
    ];
    let base = id.trim_end_matches(|c: char| c.is_ascii_digit());
    BASES.contains(&base)
}

/// `Blood donor\\(x)` -> `blood`
pub fn slug(text: Option<&str>, fallback: &str) -> String {
    let t = text.unwrap_or("");
    let mut cleaned = String::new();
    let chars: Vec<char> = t.chars().collect();
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        if c == '\\' && chars.get(i + 1).is_some_and(|n| n.is_ascii_alphabetic()) {
            cleaned.push(' ');
            i += 1;
            while chars.get(i).is_some_and(|n| n.is_ascii_alphabetic()) {
                i += 1;
            }
            continue;
        }
        cleaned.push(if matches!(c, '{' | '}' | '$' | '\\') { ' ' } else { c });
        i += 1;
    }
    // the first [A-Za-z][A-Za-z0-9]*
    let mut word = String::new();
    for c in cleaned.chars() {
        if word.is_empty() {
            if c.is_ascii_alphabetic() {
                word.push(c);
            }
        } else if c.is_ascii_alphanumeric() {
            word.push(c);
        } else {
            break;
        }
    }
    if word.is_empty() { fallback.to_string() } else { word.to_lowercase() }
}

/// What the window should show or say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// for the status bar
    Message(String),
    /// something the user should read, not just glance at
    Refused(String),
    /// the label of the (single) selected element should be edited
    EditLabel,
    SelectionChanged,
    ToolChanged,
}

#[derive(Clone, Debug)]
enum Origin {
    Pos(f64, f64),
    Corners(Vec<Option<(f64, f64)>>),
    Via,
}

#[derive(Clone, Debug)]
enum Drag {
    Move {
        start: Pt,
        origins: Vec<(ElementKey, Origin)>,
        primary: ElementKey,
        free: bool,
        via_orig: HashMap<ElementKey, String>,
    },
    Handle {
        name: String,
        element: ElementKey,
        free: bool,
        /// the oval being dragged (its row) and where it was grabbed
        oval: Option<usize>,
        grab: Pt,
    },
    Rubber {
        start: Pt,
        now: Pt,
    },
    Subdomain {
        start: Pt,
        now: Pt,
        free: bool,
    },
    Connect {
        command: String,
        src: ElementKey,
        now: Pt,
        target: Option<ElementKey>,
        ok: bool,
        why: String,
    },
}

/// The pointer's buttons and keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// Distance from a point to a chain of segments (curves sampled).
pub fn dist_point_segments(x: f64, y: f64, segments: &[Segment]) -> f64 {
    let mut best = 1e9f64;
    for s in segments {
        let pts: Vec<Pt> = if s.controls.is_some() {
            (0..=20).map(|k| s.point(f64::from(k) / 20.0)).collect()
        } else {
            vec![s.p0, s.p3]
        };
        for w in pts.windows(2) {
            let [(x1, y1), (x2, y2)] = w else { continue };
            let (dx, dy) = (x2 - x1, y2 - y1);
            let l2 = dx * dx + dy * dy;
            let t = if l2 == 0.0 { 0.0 } else { (((x - x1) * dx + (y - y1) * dy) / l2).clamp(0.0, 1.0) };
            best = best.min((x - (x1 + t * dx)).hypot(y - (y1 + t * dy)));
        }
    }
    best
}

fn type_name(b: &Body) -> &'static str {
    match b {
        Body::Node(n) if n.generalisation.is_some() => "Isa",
        Body::Node(_) => "Node",
        Body::Point(_) => "Point",
        Body::Edge(_) => "Edge",
        Body::Attributes(_) => "AttributeTree",
        Body::Note(_) => "Note",
        Body::Subdomain(_) => "Subdomain",
        Body::Grid(_) => "Grid",
        Body::Setup(_) => "Setup",
        Body::Raw(_) => "Raw",
    }
}

fn item_at_mut(items: &mut [AttrItem], row: usize) -> Option<&mut AttrItem> {
    fn walk<'a>(items: &'a mut [AttrItem], row: usize, k: &mut usize) -> Option<&'a mut AttrItem> {
        for it in items {
            if *k == row {
                return Some(it);
            }
            *k += 1;
            if let Some(found) = walk(&mut it.parts, row, k) {
                return Some(found);
            }
        }
        None
    }
    walk(items, row, &mut 0)
}

fn cm(p: Coord) -> Option<(f64, f64)> {
    match p {
        Coord::Numeric { x, y } => Some((x, y)),
        Coord::Raw(_) => None,
    }
}

fn shifted(via: &str, dx: f64, dy: f64) -> String {
    parse_coord_list(via)
        .into_iter()
        .map(|p| match p {
            Coord::Numeric { x, y } => Coord::numeric(py_round(x + dx, 3), py_round(y + dy, 3)).to_tex(),
            raw => raw.to_tex(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

const SELECT: &str = "#2878ff";
const HOVER: &str = "#8db7ff";

/// The editing state of one open diagram.
#[derive(Clone, Debug)]
pub struct Editor {
    session: Session,
    pub tool: String,
    pub selection: Vec<ElementKey>,
    /// grid snapping in cm; 0 for none
    pub snap_cm: f64,
    /// TeX points per screen pixel
    px: f64,
    drag: Option<Drag>,
    hover: Option<ElementKey>,
    events: Vec<Event>,
    /// whether hints are shown (badges, tooltips, the Problems panel)
    pub show_hints: bool,
    /// the problems of a revision of the document (see `problems::collect`)
    pub(crate) problem_cache: Option<(u64, Vec<crate::rules::Problem>)>,
}

impl Editor {
    pub fn new(session: Session) -> Editor {
        Editor {
            session,
            tool: "select".into(),
            selection: vec![],
            snap_cm: 0.25,
            px: 0.75,
            drag: None,
            hover: None,
            events: vec![],
            show_hints: true,
            problem_cache: None,
        }
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    pub fn doc(&self) -> &Document {
        self.session.doc()
    }

    /// What happened since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    fn message(&mut self, text: impl Into<String>) {
        self.events.push(Event::Message(text.into()));
    }

    /// TeX points per screen pixel, for the sizes of handles and how near a click must be.
    pub fn set_scale(&mut self, pt_per_px: f64) {
        self.px = pt_per_px.max(1e-6);
    }

    // -- the document, undo ---------------------------------------------------------------

    fn sel_keys(&self) -> Vec<SelKey> {
        let doc = self.doc();
        self.selection
            .iter()
            .filter_map(|k| {
                let el = doc.get(*k)?;
                if let Some(id) = el.id() {
                    return Some(SelKey::Id(id.to_string()));
                }
                let name = type_name(&el.body);
                let n =
                    doc.elements.iter().filter(|e| type_name(&e.body) == name).position(|e| e.key == *k)?;
                Some(SelKey::Nth(name, n))
            })
            .collect()
    }

    fn resolve(&mut self, keys: Vec<SelKey>) {
        let doc = self.doc();
        let found: Vec<ElementKey> = keys
            .iter()
            .filter_map(|k| match k {
                SelKey::Id(id) => doc.by_id(id).map(|e| e.key),
                SelKey::Nth(name, n) => {
                    doc.elements.iter().filter(|e| type_name(&e.body) == *name).nth(*n).map(|e| e.key)
                }
            })
            .collect();
        self.selection = found;
        self.events.push(Event::SelectionChanged);
    }

    fn begin_edit(&mut self) {
        let keys = self.sel_keys();
        self.session.begin(keys);
    }

    fn end_edit(&mut self, label: &str) -> bool {
        let keys = self.sel_keys();
        self.session.end(label, keys)
    }

    fn cancel_edit(&mut self) {
        if let Some(keys) = self.session.cancel() {
            self.resolve(keys);
        }
    }

    /// Runs `f` as one undoable edit called `label`.
    pub fn apply(&mut self, label: &str, f: impl FnOnce(&mut Editor)) -> bool {
        self.settle(); // a drag in progress is one step, this edit another
        self.begin_edit();
        f(self);
        self.end_edit(label)
    }

    /// Ends a drag that was interrupted (a tool chosen, undo, a press without a release) as if
    /// the button had been released, so that its change is one step that can be undone.
    fn settle(&mut self) {
        match self.drag.take() {
            Some(Drag::Move { .. }) => {
                self.end_edit("Move");
            }
            Some(Drag::Handle { element, .. }) => {
                let edge = matches!(self.doc().get(element).map(|e| &e.body), Some(Body::Edge(_)));
                self.end_edit(if edge { "Edit edge" } else { "Reshape" });
            }
            _ => {}
        }
    }

    pub fn undo(&mut self) -> bool {
        self.settle();
        match self.session.undo() {
            Some(keys) => {
                self.resolve(keys);
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        self.settle();
        match self.session.redo() {
            Some(keys) => {
                self.resolve(keys);
                true
            }
            None => false,
        }
    }

    /// Another document (reloaded): nothing selected, nothing being dragged.
    pub fn replace(&mut self, doc: Document) {
        self.session.replace(doc);
        self.selection.clear();
        self.drag = None;
        self.hover = None;
    }

    // -- selection and tools ------------------------------------------------------------

    pub fn set_selection(&mut self, keys: Vec<ElementKey>) {
        let doc = self.doc();
        self.selection = keys.into_iter().filter(|k| doc.get(*k).is_some()).collect();
        self.events.push(Event::SelectionChanged);
    }

    pub fn select_all(&mut self) {
        let keys = self.doc().elements.iter().filter(|e| e.editable()).map(|e| e.key).collect();
        self.set_selection(keys);
    }

    pub fn set_tool(&mut self, tool: &str) {
        self.settle();
        self.tool = tool.to_string();
        self.events.push(Event::ToolChanged);
    }

    fn snap(&self, v_cm: f64, free: bool) -> f64 {
        if free || self.snap_cm == 0.0 {
            return py_round(v_cm, 3);
        }
        py_round(round_half_even(v_cm / self.snap_cm) * self.snap_cm, 3)
    }

    // -- hit testing --------------------------------------------------------------------

    fn oval_at(g: &Gfx, x: f64, y: f64) -> Option<&crate::layout::Oval> {
        match g.handles.get("ovals") {
            Some(Handle::Ovals(ovals)) => ovals.iter().rev().find(|o| o.shape.contains(x, y)),
            _ => None,
        }
    }

    /// The element under (x, y), the top one first.
    pub fn element_at(&mut self, x: f64, y: f64) -> Option<ElementKey> {
        let tol = 4.0 * self.px;
        let (doc, laid) = self.session.parts();
        let mut order: Vec<&Gfx> = laid.gfx.iter().collect();
        order.sort_by(|a, b| b.z.total_cmp(&a.z));
        for g in order {
            let Some(key) = g.key else { continue };
            let Some(el) = doc.get(key) else { continue };
            let hit = match &el.body {
                Body::Point(_) => match g.handles.get("center") {
                    Some(Handle::Point((cx, cy))) => (x - cx).hypot(y - cy) <= (tol * 1.5).max(3.0),
                    _ => false,
                },
                body @ (Body::Node(_) | Body::Note(_)) => match &g.shape {
                    Some(shape) if shape.contains(x, y) => true,
                    Some(shape) if matches!(body, Body::Node(_)) => {
                        let (bx, by) = shape.border_toward(x, y);
                        (x - bx).hypot(y - by) <= tol
                            && (x - shape.cx).hypot(y - shape.cy)
                                <= (bx - shape.cx).hypot(by - shape.cy) + tol
                    }
                    _ => false,
                },
                Body::Edge(_) if !g.segments.is_empty() => {
                    let all: Vec<Segment> = g.segments.iter().chain(&g.stub).copied().collect();
                    dist_point_segments(x, y, &all) <= tol
                }
                Body::Attributes(_) if g.handles.contains_key("ovals") => Self::oval_at(g, x, y).is_some(),
                Body::Attributes(_) => crate::svg::bbox(g)
                    .is_some_and(|b| b.0 - tol <= x && x <= b.2 + tol && b.1 - tol <= y && y <= b.3 + tol),
                Body::Subdomain(_) => {
                    let in_label = matches!(g.handles.get("labelbox"),
                        Some(Handle::Rect(x0, y0, x1, y1)) if *x0 <= x && x <= *x1 && *y0 <= y && y <= *y1);
                    let on_outline = matches!(g.handles.get("outline"),
                        Some(Handle::Segments(s)) if dist_point_segments(x, y, s) <= tol);
                    in_label || on_outline
                }
                _ => false,
            };
            if hit {
                return Some(key);
            }
        }
        None
    }

    /// The drag handles of the (single) selected element.
    pub fn handles(&mut self) -> Vec<(String, Pt)> {
        let [key] = self.selection.as_slice() else { return vec![] };
        let key = *key;
        let (doc, laid) = self.session.parts();
        let (Some(el), Some(g)) = (doc.get(key), laid.gfx_for(key)) else { return vec![] };
        let mut out = Vec::new();
        match &el.body {
            Body::Edge(e) if !g.segments.is_empty() => {
                let (Some(first), Some(last)) = (g.segments.first(), g.segments.last()) else { return out };
                let at = |name: &str, default: Pt| match g.handles.get(name) {
                    Some(Handle::Point(p)) => *p,
                    _ => default,
                };
                out.push(("start".to_string(), at("start", first.p0))); // at the entity, also
                out.push(("end".to_string(), at("end", last.p3))); // with a crow's foot stub
                if e.opts.get("via").is_some_and(|v| !v.is_empty()) {
                    for (k, s) in g.segments.iter().take(g.segments.len() - 1).enumerate() {
                        out.push((format!("wp{k}"), s.p3));
                    }
                } else if e.opts.get("ortho").is_none_or(str::is_empty) {
                    out.push(("bend".into(), first.point(0.5)));
                }
            }
            Body::Subdomain(_) => {
                let mut corners: Vec<(usize, Pt)> = g
                    .handles
                    .iter()
                    .filter_map(|(k, v)| {
                        match (k.strip_prefix('p').and_then(|n| n.parse::<usize>().ok()), v) {
                            (Some(n), Handle::Point(p)) => Some((n, *p)),
                            _ => None,
                        }
                    })
                    .collect();
                corners.sort_by_key(|c| c.0);
                // Python iterates the handles in their order of insertion: p0, p1, ...
                out.extend(corners.into_iter().map(|(n, p)| (format!("p{n}"), p)));
                // round handles in the middle of each side: drag to add a corner
                if let Some(Handle::Segments(sides)) = g.handles.get("outline") {
                    for (k, s) in sides.iter().enumerate() {
                        out.push((format!("m{k}"), s.point(0.5)));
                    }
                }
            }
            _ => {}
        }
        out
    }

    fn handle_at(&mut self, x: f64, y: f64) -> Option<String> {
        let r = 6.0 * self.px;
        self.handles()
            .into_iter()
            .find(|(_, (hx, hy))| (x - hx).abs() <= r && (y - hy).abs() <= r)
            .map(|h| h.0)
    }

    // -- model helpers ------------------------------------------------------------------

    fn center_cm(&mut self, key: ElementKey) -> Option<(f64, f64)> {
        let laid = self.session.laid();
        match laid.gfx_for(key)?.handles.get("center") {
            Some(Handle::Point((x, y))) => Some((x / CM, y / CM)),
            _ => None,
        }
    }

    fn body_mut(&mut self, key: ElementKey) -> Option<&mut crate::model::Element> {
        self.session.doc_mut().get_mut(key)
    }

    /// Writes an arrangement into the diagram (`arrange::apply`), as one step that can be undone.
    pub fn arrange(&mut self, a: &crate::arrange::Arrangement) -> bool {
        self.apply("Arrange diagram", |ed| {
            let language = ed.session.language();
            crate::arrange::apply(ed.session.doc_mut(), a, language);
        })
    }

    /// Turns `right=of x` or calc positions into plain (x,y).
    pub fn make_absolute(&mut self, key: ElementKey) {
        let Some(el) = self.doc().get(key) else { return };
        let (pos, opts) = match &el.body {
            Body::Point(_) => return, // has no options, and its position stays as written
            Body::Node(n) => (n.pos.clone(), n.opts.clone()),
            Body::Note(n) => (n.pos.clone(), n.opts.clone()),
            _ => return,
        };
        let is_node = matches!(el.body, Body::Node(_));
        if pos.as_ref().is_some_and(Coord::is_numeric) && !POSITIONING.iter().any(|k| opts.contains(k)) {
            return;
        }
        let Some((cx, cy)) = self.center_cm(key) else { return };
        let mut at = (cx, cy);
        if !is_node && let Some(anchor) = opts.get("anchor").filter(|a| !a.is_empty()) {
            // notes keep their anchor: convert the anchor point
            let laid = self.session.laid();
            if let Some(shape) = laid.gfx_for(key).and_then(|g| g.shape.as_ref()) {
                let (px, py) = shape.anchor(anchor);
                at = (px / CM, py / CM);
            }
        }
        let coord = Coord::numeric(py_round(at.0, 3), py_round(at.1, 3));
        if let Some(el) = self.body_mut(key) {
            match &mut el.body {
                Body::Node(n) => {
                    for k in POSITIONING {
                        n.opts.remove(k);
                    }
                    n.opts.remove("anchor");
                    n.pos = Some(coord);
                }
                Body::Note(n) => {
                    for k in POSITIONING {
                        n.opts.remove(k);
                    }
                    n.pos = Some(coord);
                }
                _ => {}
            }
            el.touch();
        }
    }

    /// The position (cm) used for moving: pos for nodes, points and notes.
    fn movable_pos(&mut self, key: ElementKey) -> Option<(f64, f64)> {
        self.make_absolute(key);
        match &self.doc().get(key)?.body {
            Body::Node(Node { pos, .. }) | Body::Point(Point { pos, .. }) | Body::Note(Note { pos, .. }) => {
                pos.clone().and_then(cm)
            }
            _ => None,
        }
    }

    fn set_pos(&mut self, key: ElementKey, x: f64, y: f64) {
        if let Some(el) = self.body_mut(key) {
            match &mut el.body {
                Body::Node(Node { pos, .. })
                | Body::Point(Point { pos, .. })
                | Body::Note(Note { pos, .. }) => {
                    *pos = Some(Coord::numeric(x, y));
                }
                _ => return,
            }
            el.touch();
        }
    }

    fn node_ids(&self, keys: &[ElementKey]) -> HashSet<String> {
        keys.iter()
            .filter_map(|k| match &self.doc().get(*k)?.body {
                Body::Node(n) => Some(n.id.clone()),
                Body::Point(p) => Some(p.id.clone()),
                _ => None,
            })
            .collect()
    }

    // -- mouse ----------------------------------------------------------------------------

    /// The left button pressed at (x, y).
    pub fn press(&mut self, x: f64, y: f64, mods: Mods) {
        self.settle();
        let free = mods.alt;
        if self.tool == "select" {
            if let Some(mut h) = self.handle_at(x, y) {
                self.begin_edit();
                let Some(&target) = self.selection.first() else { return };
                let is_subdomain =
                    matches!(self.doc().get(target).map(|e| &e.body), Some(Body::Subdomain(_)));
                if is_subdomain && let Some(k) = h.strip_prefix('m').and_then(|n| n.parse::<usize>().ok()) {
                    let (cx, cy) = (self.snap(x / CM, free), self.snap(y / CM, free));
                    h = format!("p{}", self.insert_corner(target, k, cx, cy));
                }
                self.drag =
                    Some(Drag::Handle { name: h, element: target, free, oval: None, grab: (0.0, 0.0) });
                return;
            }
            if let Some(e) = self.element_at(x, y) {
                if mods.shift || mods.ctrl {
                    let mut sel = self.selection.clone();
                    match sel.iter().position(|k| *k == e) {
                        Some(i) => {
                            sel.remove(i);
                        }
                        None => sel.push(e),
                    }
                    self.set_selection(sel);
                    return;
                }
                if !self.selection.contains(&e) {
                    self.set_selection(vec![e]);
                }
                self.start_move(e, x, y, free);
                return;
            }
            if !mods.shift {
                self.set_selection(vec![]);
            }
            self.drag = Some(Drag::Rubber { start: (x, y), now: (x, y) });
            return;
        }
        let kind = self.doc().kind;
        if let Some((_, command)) = edge_tools(kind).iter().find(|(t, _)| *t == self.tool) {
            let src = self.element_at(x, y).filter(|k| {
                matches!(self.doc().get(*k).map(|e| &e.body), Some(Body::Node(_) | Body::Point(_)))
            });
            match src {
                Some(src) => {
                    self.drag = Some(Drag::Connect {
                        command: command.to_string(),
                        src,
                        now: (x, y),
                        target: None,
                        ok: false,
                        why: String::new(),
                    })
                }
                None => self.message("Press on an element and drag to the element to connect."),
            }
            return;
        }
        if self.tool == "subdomain" {
            self.drag = Some(Drag::Subdomain { start: (x, y), now: (x, y), free });
            return;
        }
        let tool = self.tool.clone();
        self.place(&tool, x, y, free);
    }

    /// The pointer moved to (x, y), with or without a button.
    pub fn move_to(&mut self, x: f64, y: f64, mods: Mods) {
        let Some(drag) = self.drag.clone() else {
            self.hover = if self.tool == "select" { self.element_at(x, y) } else { None };
            return;
        };
        match drag {
            Drag::Move { free, .. } => self.drag_move(x, y, free || mods.alt),
            Drag::Handle { free, .. } => self.drag_handle(x, y, free || mods.alt),
            Drag::Rubber { start, .. } => self.drag = Some(Drag::Rubber { start, now: (x, y) }),
            Drag::Subdomain { start, free, .. } => {
                self.drag = Some(Drag::Subdomain { start, now: (x, y), free })
            }
            Drag::Connect { command, src, .. } => {
                let t = self.element_at(x, y).filter(|t| *t != src).filter(|t| {
                    matches!(self.doc().get(*t).map(|e| &e.body), Some(Body::Node(_) | Body::Point(_)))
                });
                let (mut ok, mut why) = (false, String::new());
                if let Some(t) = t {
                    let ids =
                        (self.doc().get(src).and_then(|e| e.id()), self.doc().get(t).and_then(|e| e.id()));
                    if let (Some(a), Some(b)) = ids {
                        // the refusal names its rule in the rule books (the status bar links it)
                        (ok, why) = match refusal(self.doc(), &command, a, b) {
                            Some((rule, why)) => (false, format!("{why} ({rule})")),
                            None => (true, String::new()),
                        };
                    }
                    if !ok {
                        self.message(format!("Not allowed: {why}"));
                    }
                }
                self.drag = Some(Drag::Connect { command, src, now: (x, y), target: t, ok, why });
            }
        }
    }

    /// The left button released at (x, y).
    pub fn release(&mut self, x: f64, y: f64, mods: Mods) {
        let Some(drag) = self.drag.take() else { return };
        match drag {
            Drag::Move { .. } => {
                self.end_edit("Move");
            }
            Drag::Handle { element, .. } => {
                let edge = matches!(self.doc().get(element).map(|e| &e.body), Some(Body::Edge(_)));
                self.end_edit(if edge { "Edit edge" } else { "Reshape" });
            }
            Drag::Rubber { start, .. } => self.rubber_select(start, (x, y), mods.shift),
            Drag::Subdomain { start, free, .. } => self.create_subdomain(start, (x, y), free),
            Drag::Connect { command, src, target, ok, why, .. } => match target {
                Some(t) if ok => self.create_edge(&command, src, t),
                Some(_) => self.message(format!("Not connected: {why}")),
                None => {}
            },
        }
    }

    /// A double click at (x, y); returns whether it did something.
    pub fn double_click(&mut self, x: f64, y: f64) -> bool {
        if self.tool != "select" {
            return false;
        }
        if let Some(h) = self.handle_at(x, y)
            && let Some(&target) = self.selection.first()
        {
            if let Some(k) = h.strip_prefix("wp").and_then(|n| n.parse::<usize>().ok()) {
                self.remove_waypoint(target, k);
                return true;
            }
            let is_subdomain = matches!(self.doc().get(target).map(|e| &e.body), Some(Body::Subdomain(_)));
            if is_subdomain && let Some(k) = h.strip_prefix('p').and_then(|n| n.parse::<usize>().ok()) {
                self.remove_corner(target, k);
                return true;
            }
        }
        match self.element_at(x, y) {
            Some(e) if matches!(self.doc().get(e).map(|el| &el.body), Some(Body::Edge(_))) => {
                self.add_waypoint(e, x, y);
                true
            }
            Some(e) => {
                self.set_selection(vec![e]);
                self.events.push(Event::EditLabel);
                true
            }
            None => false,
        }
    }

    /// A command from the keyboard: `cancel`, `delete`, `nudge left` ...  Returns whether it
    /// was used.
    pub fn key(&mut self, command: &str, mods: Mods) -> bool {
        if command == "cancel" {
            if let Some(d) = self.drag.take() {
                if matches!(d, Drag::Move { .. } | Drag::Handle { .. }) {
                    self.cancel_edit();
                }
            } else if self.tool != "select" {
                self.set_tool("select");
            } else {
                self.set_selection(vec![]);
            }
            return true;
        }
        if !self.selection.is_empty() && command == "delete" {
            self.delete_selection();
            return true;
        }
        let dir = match command {
            "nudge left" => (-1.0, 0.0),
            "nudge right" => (1.0, 0.0),
            "nudge up" => (0.0, 1.0),
            "nudge down" => (0.0, -1.0),
            _ => return false,
        };
        if self.selection.is_empty() {
            return false;
        }
        let mut step = if self.snap_cm != 0.0 { self.snap_cm } else { 0.25 };
        if mods.shift {
            step = 1.0;
        }
        if mods.alt {
            step = 0.05;
        }
        let sel = self.selection.clone();
        self.apply("Nudge", |ed| ed.translate(&sel, dir.0 * step, dir.1 * step));
        true
    }

    /// What the pointer should look like at (x, y).
    pub fn cursor(&mut self, x: f64, y: f64) -> &'static str {
        if self.tool != "select" {
            return "crosshair";
        }
        if self.drag.is_some() {
            return "move";
        }
        if self.handle_at(x, y).is_some() {
            "pointer"
        } else if self.element_at(x, y).is_some() {
            "move"
        } else {
            "default"
        }
    }

    // -- moving -------------------------------------------------------------------------

    /// A box moves with what's inside it.
    fn with_contents(&mut self, keys: &[ElementKey]) -> Vec<ElementKey> {
        let mut out = keys.to_vec();
        let extra: Vec<ElementKey> = self.session.with_layout(|doc, layout| {
            let boxes = boxes_of(doc, layout);
            boxes
                .iter()
                .filter(|b| keys.contains(&b.element))
                .flat_map(|b| members(b, doc, layout, &boxes))
                .collect()
        });
        for k in extra {
            if !out.contains(&k) {
                out.push(k);
            }
        }
        out
    }

    fn start_move(&mut self, primary: ElementKey, x: f64, y: f64, free: bool) {
        self.begin_edit();
        let sel = self.selection.clone();
        let mut origins = Vec::new();
        for k in self.with_contents(&sel) {
            let Some(el) = self.doc().get(k) else { continue };
            match &el.body {
                Body::Node(_) | Body::Point(_) | Body::Note(_) => {
                    if let Some(p) = self.movable_pos(k) {
                        origins.push((k, Origin::Pos(p.0, p.1)));
                    }
                }
                Body::Subdomain(s) => {
                    let corners = s.points.iter().map(|c| cm(c.clone())).collect();
                    origins.push((k, Origin::Corners(corners)));
                }
                Body::Edge(e) if e.opts.get("via").is_some_and(|v| !v.is_empty()) => {
                    origins.push((k, Origin::Via))
                }
                _ => {}
            }
        }
        self.drag = Some(Drag::Move { start: (x, y), origins, primary, free, via_orig: HashMap::new() });
        let tree = matches!(self.doc().get(primary).map(|e| &e.body), Some(Body::Attributes(_)));
        if tree && sel.len() == 1 {
            let laid = self.session.laid();
            let (oval, grab) = match laid.gfx_for(primary) {
                Some(g) if g.handles.contains_key("ovals") => match Self::oval_at(g, x, y) {
                    Some(o) => (Some(o.row), (x - o.shape.cx, y - o.shape.cy)),
                    None => (None, (0.0, 0.0)),
                },
                _ => (None, (0.0, 0.0)),
            };
            let is_ovals = laid.gfx_for(primary).is_some_and(|g| g.handles.contains_key("ovals"));
            self.drag = Some(Drag::Handle {
                name: if is_ovals { "oval".into() } else { "tree".into() },
                element: primary,
                free,
                oval,
                grab,
            });
        }
    }

    fn drag_move(&mut self, x: f64, y: f64, free: bool) {
        let Some(Drag::Move { start, origins, primary, free: f0, mut via_orig }) = self.drag.clone() else {
            return;
        };
        let (sx, sy) = start;
        let (mut dx, mut dy) = ((x - sx) / CM, (y - sy) / CM);
        // snap the primary element, move the rest by the same amount
        match origins.iter().find(|(k, _)| *k == primary).map(|o| &o.1) {
            Some(Origin::Pos(ox, oy)) => {
                dx = self.snap(ox + dx, free) - ox;
                dy = self.snap(oy + dy, free) - oy;
            }
            _ if !free && self.snap_cm != 0.0 => {
                dx = self.snap(dx, false);
                dy = self.snap(dy, false);
            }
            _ => {}
        }
        let mut moved: HashSet<String> = HashSet::new();
        for (k, orig) in &origins {
            match orig {
                Origin::Pos(ox, oy) => {
                    self.set_pos(*k, py_round(ox + dx, 3), py_round(oy + dy, 3));
                    if let Some(id) = self.doc().get(*k).and_then(|e| e.id()) {
                        moved.insert(id.to_string());
                    }
                }
                Origin::Corners(corners) => {
                    if let Some(el) = self.body_mut(*k) {
                        if let Body::Subdomain(s) = &mut el.body {
                            s.points = corners
                                .iter()
                                .zip(&s.points)
                                .map(|(p, c)| match p {
                                    Some((px, py)) => {
                                        Coord::numeric(py_round(px + dx, 3), py_round(py + dy, 3))
                                    }
                                    None => c.clone(),
                                })
                                .collect();
                        }
                        el.touch();
                    }
                }
                Origin::Via => {}
            }
        }
        // way points of edges whose both ends move along
        let edges: Vec<(ElementKey, String, String, String)> = self
            .doc()
            .elements
            .iter()
            .filter_map(|el| match &el.body {
                Body::Edge(e) => e
                    .opts
                    .get("via")
                    .filter(|v| !v.is_empty())
                    .map(|v| (el.key, v.to_string(), e.src_id().to_string(), e.dst_id().to_string())),
                _ => None,
            })
            .collect();
        for (k, via, src, dst) in edges {
            if moved.contains(&src) && moved.contains(&dst) {
                let orig = via_orig.entry(k).or_insert(via).clone();
                let new = shifted(&orig, dx, dy);
                if let Some(el) = self.body_mut(k) {
                    if let Body::Edge(e) = &mut el.body {
                        e.opts.set("via", Some(&new));
                    }
                    el.touch();
                }
            }
        }
        self.drag = Some(Drag::Move { start, origins, primary, free: f0, via_orig });
    }

    /// Moves elements (and what's in the boxes among them) by (dx, dy) cm.
    pub fn translate(&mut self, keys: &[ElementKey], dx: f64, dy: f64) {
        let mut ids = HashSet::new();
        for k in self.with_contents(keys) {
            let Some(el) = self.doc().get(k) else { continue };
            match &el.body {
                Body::Node(_) | Body::Point(_) | Body::Note(_) => {
                    let is_note = matches!(el.body, Body::Note(_));
                    let id = el.id().map(str::to_string);
                    let Some(p) = self.movable_pos(k) else { continue };
                    self.set_pos(k, py_round(p.0 + dx, 3), py_round(p.1 + dy, 3));
                    if !is_note && let Some(id) = id {
                        ids.insert(id);
                    }
                }
                Body::Subdomain(_) => {
                    if let Some(el) = self.body_mut(k) {
                        if let Body::Subdomain(s) = &mut el.body {
                            for c in &mut s.points {
                                if let Coord::Numeric { x, y } = c {
                                    *c = Coord::numeric(py_round(*x + dx, 3), py_round(*y + dy, 3));
                                }
                            }
                        }
                        el.touch();
                    }
                }
                _ => {}
            }
        }
        let edges: Vec<ElementKey> = self
            .doc()
            .elements
            .iter()
            .filter(|el| match &el.body {
                Body::Edge(e) => {
                    e.opts.get("via").is_some_and(|v| !v.is_empty())
                        && ids.contains(e.src_id())
                        && ids.contains(e.dst_id())
                }
                _ => false,
            })
            .map(|el| el.key)
            .collect();
        for k in edges {
            if let Some(el) = self.body_mut(k) {
                if let Body::Edge(e) = &mut el.body {
                    let new = shifted(e.opts.get("via").unwrap_or(""), dx, dy);
                    e.opts.set("via", Some(&new));
                }
                el.touch();
            }
        }
    }

    // -- handles ------------------------------------------------------------------------

    fn drag_oval(&mut self, key: ElementKey, x: f64, y: f64, free: bool, oval: Option<usize>, grab: Pt) {
        let Some(row) = oval else { return };
        let ovals = match self.session.laid().gfx_for(key).and_then(|g| g.handles.get("ovals")) {
            Some(Handle::Ovals(o)) => o.clone(),
            _ => return,
        };
        let Some(this) = ovals.iter().find(|o| o.row == row).cloned() else { return };
        let (gx, gy) = grab;
        let (cx, cy) = (self.snap((x - gx) / CM, free), self.snap((y - gy) / CM, free));
        let pos = (py_round(cx - this.origin.0 / CM, 2), py_round(cy - this.origin.1 / CM, 2));
        let Some(el) = self.body_mut(key) else { return };
        let Body::Attributes(t) = &mut el.body else { return };
        // the others would move to fill the gap: they stay where they are from now on
        for o in &ovals {
            if let Some(it) = item_at_mut(&mut t.items, o.row)
                && it.pos.is_none()
            {
                it.pos = Some((
                    py_round((o.shape.cx - o.origin.0) / CM, 2),
                    py_round((o.shape.cy - o.origin.1) / CM, 2),
                ));
            }
        }
        if let Some(it) = item_at_mut(&mut t.items, row)
            && it.pos != Some(pos)
        {
            it.pos = Some(pos);
        }
        el.touch();
    }

    fn drag_handle(&mut self, x: f64, y: f64, free: bool) {
        let Some(Drag::Handle { name, element, oval, grab, .. }) = self.drag.clone() else { return };
        let (cx, cy) = (self.snap(x / CM, free), self.snap(y / CM, free));
        let body = match self.doc().get(element) {
            Some(el) => el.body.clone(),
            None => return,
        };
        match (&body, name.as_str()) {
            (Body::Attributes(_), "oval") => self.drag_oval(element, x, y, free, oval, grab),
            (Body::Attributes(t), "tree") => {
                let Some(node) = self.session.laid().shapes.get(&t.node).cloned() else { return };
                let ang = angle_of(x - node.cx, y - node.cy);
                let compass = [
                    "east",
                    "north east",
                    "north",
                    "north west",
                    "west",
                    "south west",
                    "south",
                    "south east",
                ];
                let i =
                    (round_half_even(crate::text::py_mod(ang, 360.0) / 45.0) as i64).rem_euclid(8) as usize;
                let compass = compass.get(i).copied().unwrap_or("east");
                let current = t.opts.get("anchor").filter(|a| !a.is_empty()).unwrap_or("east");
                if current != compass
                    && let Some(el) = self.body_mut(element)
                    && let Body::Attributes(t) = &mut el.body
                {
                    t.opts.set("anchor", Some(compass));
                    for k in ["dir", "side", "xshift", "yshift", "align"] {
                        t.opts.remove(k);
                    }
                    if compass == "east" {
                        t.opts.remove("anchor");
                    }
                    el.touch();
                }
            }
            (Body::Subdomain(s), n) if n.starts_with('p') => {
                if let Some(k) = n.get(1..).and_then(|k| k.parse::<usize>().ok())
                    && k < s.points.len()
                    && let Some(el) = self.body_mut(element)
                    && let Body::Subdomain(s) = &mut el.body
                {
                    if let Some(p) = s.points.get_mut(k) {
                        *p = Coord::numeric(cx, cy);
                    }
                    el.touch();
                }
            }
            (Body::Edge(e), n) if n.starts_with("wp") => {
                let mut pts = parse_coord_list(e.opts.get("via").unwrap_or(""));
                if let Some(k) = n.get(2..).and_then(|k| k.parse::<usize>().ok())
                    && let Some(p) = pts.get_mut(k)
                {
                    *p = Coord::numeric(cx, cy);
                    let via = pts.iter().map(Coord::to_tex).collect::<Vec<_>>().join(" ");
                    if let Some(el) = self.body_mut(element) {
                        if let Body::Edge(e) = &mut el.body {
                            e.opts.set("via", Some(&via));
                        }
                        el.touch();
                    }
                }
            }
            (Body::Edge(_), "bend") => self.set_bend_from_point(element, x, y),
            (Body::Edge(e), end @ ("start" | "end")) => {
                let t = self.element_at(x, y);
                let target = t.and_then(|t| match &self.doc().get(t)?.body {
                    Body::Node(n) => Some(n.id.clone()),
                    Body::Point(p) => Some(p.id.clone()),
                    _ => None,
                });
                let other = if end == "start" { e.dst.clone() } else { e.src.clone() };
                if let Some(tid) = target
                    && tid != endpoint_id(&other)
                {
                    let (src, dst) = if end == "start" {
                        (tid.clone(), e.dst.clone())
                    } else {
                        (e.src.clone(), tid.clone())
                    };
                    let cur = if end == "start" { &e.src } else { &e.dst };
                    if endpoint_id(cur) != tid {
                        let refused = refusal(self.doc(), &e.command, &src, &dst);
                        if refused.is_none() {
                            if let Some(el) = self.body_mut(element) {
                                if let Body::Edge(e) = &mut el.body {
                                    if end == "start" {
                                        e.src = tid;
                                    } else {
                                        e.dst = tid;
                                    }
                                }
                                el.touch();
                            }
                            self.sync_double(element);
                        } else if let Some((rule, why)) = refused {
                            self.message(format!("Not allowed: {why} ({rule})"));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn reference_point(laid: &Laid, reference: &str) -> Option<Pt> {
        let (ident, anchor) = reference.split_once('.').unwrap_or((reference, ""));
        let shape = laid.shapes.get(crate::text::strip(ident))?;
        Some(if anchor.is_empty() { shape.center() } else { shape.anchor(anchor) })
    }

    /// Chooses the bend angle so that the curve passes near (x, y).
    fn set_bend_from_point(&mut self, key: ElementKey, x: f64, y: f64) {
        let Some(Body::Edge(e)) = self.doc().get(key).map(|el| el.body.clone()) else { return };
        let laid = self.session.laid();
        let (Some(sa), Some(sb)) = (laid.shapes.get(e.src_id()), laid.shapes.get(e.dst_id())) else { return };
        let (Some(a), Some(b)) = (Self::reference_point(laid, &e.src), Self::reference_point(laid, &e.dst))
        else {
            return;
        };
        let p0 = if e.src.contains('.') { a } else { sa.border_toward(b.0, b.1) };
        let p3 = if e.dst.contains('.') { b } else { sb.border_toward(a.0, a.1) };
        let l = (p3.0 - p0.0).hypot(p3.1 - p0.1);
        if l < 1.0 {
            return;
        }
        let (mx, my) = ((p0.0 + p3.0) / 2.0, (p0.1 + p3.1) / 2.0);
        let (nx, ny) = (-(p3.1 - p0.1) / l, (p3.0 - p0.0) / l); // left normal
        let h = (x - mx) * nx + (y - my) * ny;
        let s = (h / (0.2936 * l)).clamp(-1.0, 1.0);
        let bend = round_half_even(s.asin().to_degrees()) as i64;
        if let Some(el) = self.body_mut(key) {
            if let Body::Edge(e) = &mut el.body {
                for k in ["bend left", "bend right", "out", "in", "looseness"] {
                    e.opts.remove(k);
                }
                if bend.abs() < 3 {
                    e.opts.remove("bend");
                } else {
                    e.opts.set("bend", Some(&bend.to_string()));
                }
            }
            el.touch();
        }
    }

    /// Double-lines a reconnected link if it leads to the only weak entity of a weak
    /// relationship.  With two of them, whatever double line there is keeps saying which one
    /// depends on the other.
    pub fn sync_double(&mut self, key: ElementKey) {
        let Some(Body::Edge(e)) = self.doc().get(key).map(|el| el.body.clone()) else { return };
        if should_be_double(self.doc(), &e) {
            if !is_double(&e) {
                make_double(self.session.doc_mut(), key);
            }
        } else if is_double(&e) && !may_be_double(self.doc(), &e) {
            make_single(self.session.doc_mut(), key);
        }
    }

    /// After a node stopped being weak: its double lines are not allowed.
    pub fn drop_invalid_doubles(&mut self, ident: &str) {
        let doomed: Vec<ElementKey> = self
            .doc()
            .elements
            .iter()
            .filter_map(|el| match &el.body {
                Body::Edge(e)
                    if (e.src_id() == ident || e.dst_id() == ident)
                        && is_double(e)
                        && !may_be_double(self.doc(), e) =>
                {
                    Some(el.key)
                }
                _ => None,
            })
            .collect();
        for k in doomed {
            make_single(self.session.doc_mut(), k);
        }
    }

    fn add_waypoint(&mut self, key: ElementKey, x: f64, y: f64) {
        let p = Coord::numeric(self.snap(x / CM, false), self.snap(y / CM, false));
        let segments = self.session.laid().gfx_for(key).map(|g| g.segments.clone());
        self.apply("Add way point", |ed| {
            let Some(el) = ed.body_mut(key) else { return };
            let Body::Edge(e) = &mut el.body else { return };
            let mut pts = parse_coord_list(e.opts.get("via").unwrap_or(""));
            let mut k = pts.len();
            if !pts.is_empty()
                && let Some(segs) = &segments
            {
                // into the nearest segment
                let dists: Vec<f64> =
                    segs.iter().map(|s| dist_point_segments(x, y, std::slice::from_ref(s))).collect();
                let best = dists.iter().copied().fold(f64::INFINITY, f64::min);
                k = dists.iter().position(|d| *d == best).unwrap_or(k);
            }
            pts.insert(k.min(pts.len()), p);
            for key in ["bend", "bend left", "bend right", "out", "in", "looseness", "ortho"] {
                e.opts.remove(key);
            }
            let via = pts.iter().map(Coord::to_tex).collect::<Vec<_>>().join(" ");
            e.opts.set("via", Some(&via));
            el.touch();
        });
    }

    fn remove_waypoint(&mut self, key: ElementKey, k: usize) {
        self.apply("Remove way point", |ed| {
            let Some(el) = ed.body_mut(key) else { return };
            let Body::Edge(e) = &mut el.body else { return };
            let mut pts = parse_coord_list(e.opts.get("via").unwrap_or(""));
            if k < pts.len() {
                pts.remove(k);
            }
            if pts.is_empty() {
                e.opts.remove("via");
            } else {
                let via = pts.iter().map(Coord::to_tex).collect::<Vec<_>>().join(" ");
                e.opts.set("via", Some(&via));
            }
            el.touch();
        });
    }

    // -- subdomain outlines ---------------------------------------------------------------

    /// The corners of the outline (a two-point rectangle has four).
    fn subdomain_ring(s: &Subdomain) -> Vec<Coord> {
        if let [Coord::Numeric { x: ax, y: ay }, Coord::Numeric { x: bx, y: by }] = s.points.as_slice() {
            return vec![
                Coord::numeric(*ax, *ay),
                Coord::numeric(*bx, *ay),
                Coord::numeric(*bx, *by),
                Coord::numeric(*ax, *by),
            ];
        }
        s.points.clone()
    }

    /// Inserts a corner after corner k (rectangles become polygons); returns its index.
    fn insert_corner(&mut self, key: ElementKey, k: usize, x: f64, y: f64) -> usize {
        if let Some(el) = self.body_mut(key)
            && let Body::Subdomain(s) = &mut el.body
        {
            let mut ring = Self::subdomain_ring(s);
            ring.insert((k + 1).min(ring.len()), Coord::numeric(x, y));
            s.points = ring;
            el.touch();
        }
        k + 1
    }

    fn remove_corner(&mut self, key: ElementKey, k: usize) {
        self.apply("Remove corner", |ed| {
            let ring = match ed.doc().get(key).map(|e| &e.body) {
                Some(Body::Subdomain(s)) => Self::subdomain_ring(s),
                _ => return,
            };
            if ring.len() <= 3 {
                ed.message("A subdomain needs at least three corners.");
                return;
            }
            if let Some(el) = ed.body_mut(key)
                && let Body::Subdomain(s) = &mut el.body
            {
                let mut ring = ring;
                if k < ring.len() {
                    ring.remove(k);
                }
                s.points = ring;
                el.touch();
            }
        });
    }

    /// A subdomain back to a rectangle around its corners.
    pub fn make_rectangle(&mut self, key: ElementKey) {
        self.apply("Make rectangle", |ed| {
            let Some(el) = ed.body_mut(key) else { return };
            let Body::Subdomain(s) = &mut el.body else { return };
            let pts: Vec<(f64, f64)> = s.points.iter().filter_map(|c| cm(c.clone())).collect();
            if pts.len() < 2 {
                return;
            }
            let x0 = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
            let x1 = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
            let y0 = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            let y1 = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
            s.points = vec![Coord::numeric(x0, y1), Coord::numeric(x1, y0)]; // label at the top left
            el.touch();
        });
    }

    // -- creating -----------------------------------------------------------------------

    fn add(&mut self, body: Body, after: Option<ElementKey>) -> ElementKey {
        self.session.doc_mut().add(body, after)
    }

    /// Places what `tool` makes at (x, y).
    pub fn place(&mut self, tool: &str, x: f64, y: f64, free: bool) {
        let kind = self.doc().kind;
        let pos = Coord::numeric(self.snap(x / CM, free), self.snap(y / CM, free));
        let target = self.element_at(x, y);
        let target_node = target.and_then(|t| self.doc().get(t)).and_then(|e| e.as_node()).cloned();
        if let Some((_, command, label, opts)) = node_tools(kind).iter().find(|t| t.0 == tool) {
            let base = match *command {
                "partof" => "partof".to_string(),
                "attribute" => "attr".to_string(),
                c => slug(*label, c),
            };
            let mut node = Node {
                command: command.to_string(),
                id: self.doc().unique_id(&base),
                pos: Some(pos),
                label: label.map(str::to_string),
                opts: Options {
                    items: opts.iter().map(|(k, v)| (k.to_string(), v.map(str::to_string))).collect(),
                },
                generalisation: None,
            };
            if tool == "chen attribute"
                && let (Some(t), Some(tn)) = (target, &target_node)
                && (tn.command == "entity" || tn.command == "relationship")
                && tn.generalisation.is_none()
                && let Some(c) = self.center_cm(t)
            {
                node.pos = Some(Coord::numeric(self.snap(c.0 + 2.0, false), self.snap(c.1 + 1.25, false)));
                node.opts.set("of", Some(&tn.id));
            }
            let mut key = None;
            self.apply(&format!("Add {tool}"), |ed| key = Some(ed.add(Body::Node(node), None)));
            self.set_selection(key.into_iter().collect());
            self.events.push(Event::EditLabel);
        } else if tool == "is a" || tool == "can be" {
            let Some((t, tn)) =
                target.zip(target_node).filter(|(_, n)| n.command == "entity" && n.generalisation.is_none())
            else {
                self.message("Click on the supertype entity.");
                return;
            };
            let Some(c) = self.center_cm(t) else { return };
            let command = if tool == "is a" { "isa" } else { "canbe" };
            let node = Node {
                command: command.into(),
                id: self.doc().unique_id(command),
                pos: Some(Coord::numeric(self.snap(c.0, false), self.snap(c.1 - 1.8, false))),
                label: None,
                opts: Options::new(),
                generalisation: Some(Generalisation { supertype: tn.id.clone(), constraint: None }),
            };
            let mut key = None;
            self.apply(&format!("Add {}", tool.to_uppercase()), |ed| {
                key = Some(ed.add(Body::Node(node), None))
            });
            self.set_selection(key.into_iter().collect());
            self.message("Now connect the subtypes to the diamond with the link tool.");
        } else if tool == "attribute tree" {
            let Some((t, tn)) = target.zip(target_node).filter(|(_, n)| {
                matches!(n.command.as_str(), "entity" | "relationship" | "partof")
                    && n.generalisation.is_none()
            }) else {
                self.message("Click on the entity or relationship that gets the attributes.");
                return;
            };
            let existing = self
                .doc()
                .elements
                .iter()
                .find(|e| matches!(&e.body, Body::Attributes(a) if a.node == tn.id))
                .map(|e| e.key);
            let mut key = existing;
            match existing {
                Some(tree) => {
                    self.apply("Add attribute", |ed| {
                        if let Some(el) = ed.body_mut(tree) {
                            if let Body::Attributes(a) = &mut el.body {
                                a.items.push(AttrItem::new("attribute"));
                            }
                            el.touch();
                        }
                    });
                }
                None => {
                    let tree = AttributeTree {
                        node: tn.id.clone(),
                        items: vec![AttrItem::new("attribute")],
                        opts: Options::new(),
                    };
                    self.apply("Add attributes", |ed| key = Some(ed.add(Body::Attributes(tree), Some(t))));
                }
            }
            self.set_selection(key.into_iter().collect());
            self.events.push(Event::EditLabel);
        } else if tool == "point" {
            let p = Point { id: self.doc().unique_id("point"), pos: Some(pos) };
            let mut key = None;
            self.apply("Add point", |ed| key = Some(ed.add(Body::Point(p), None)));
            self.set_selection(key.into_iter().collect());
        } else if tool == "note" {
            let n = Note {
                pos: Some(pos),
                content: "Note".into(),
                opts: Options { items: vec![("anchor".into(), Some("west".into()))] },
            };
            let mut key = None;
            self.apply("Add note", |ed| key = Some(ed.add(Body::Note(n), None)));
            self.set_selection(key.into_iter().collect());
            self.events.push(Event::EditLabel);
        }
    }

    fn create_subdomain(&mut self, a: Pt, b: Pt, free: bool) {
        let x1 = self.snap(a.0.min(b.0) / CM, free);
        let y1 = self.snap(a.1.max(b.1) / CM, free);
        let (mut x2, mut y2) = (self.snap(a.0.max(b.0) / CM, free), self.snap(a.1.min(b.1) / CM, free));
        if x2 - x1 < 0.5 || y1 - y2 < 0.5 {
            (x2, y2) = (x1 + 4.0, y1 - 3.0);
        }
        let s = Subdomain {
            label: "Subdomain".into(),
            points: vec![Coord::numeric(x1, y1), Coord::numeric(x2, y2)],
            opts: Options { items: vec![("color".into(), Some("blue!60".into()))] },
        };
        let mut key = None;
        self.apply("Add subdomain", |ed| key = Some(ed.add(Body::Subdomain(s), None)));
        self.set_selection(key.into_iter().collect());
        self.events.push(Event::EditLabel);
    }

    fn create_edge(&mut self, command: &str, src: ElementKey, dst: ElementKey) {
        let id = |k: ElementKey| self.doc().get(k).and_then(|e| e.id()).map(str::to_string);
        let (Some(s), Some(d)) = (id(src), id(dst)) else { return };
        let edge = Edge { command: command.into(), src: s, dst: d, label: None, opts: Options::new() };
        let mut undecided: Option<ElementKey> = None;
        let mut key = None;
        self.apply("Connect", |ed| {
            let k = ed.add(Body::Edge(edge.clone()), None);
            key = Some(k);
            if should_be_double(ed.doc(), &edge) {
                // weak relationship -- weak entity
                make_double(ed.session.doc_mut(), k);
            } else if may_be_double(ed.doc(), &edge) {
                // a second weak entity: only the user knows
                let src_is_rel =
                    ed.doc().get(src).and_then(|e| e.as_node()).is_some_and(|n| n.command == "relationship");
                let rel = if src_is_rel { src } else { dst };
                let rel_id = ed.doc().get(rel).and_then(|e| e.id()).map(str::to_string).unwrap_or_default();
                let links: Vec<ElementKey> = weak_links(ed.doc(), &rel_id).iter().map(|(k, _)| *k).collect();
                for l in links {
                    make_single(ed.session.doc_mut(), l);
                }
                undecided = Some(rel);
            }
        });
        if let Some(rel) = undecided {
            // the panel of the relationship has the choice
            self.set_selection(vec![rel]);
            let label = self
                .doc()
                .get(rel)
                .and_then(|e| e.as_node())
                .and_then(|n| n.label.clone())
                .unwrap_or_default();
            self.message(format!(
                "{} joins two weak entities: choose under Dependent which one depends on the other.",
                plain_text(&label)
            ));
        } else {
            self.set_selection(key.into_iter().collect());
        }
        self.events.push(Event::EditLabel);
    }

    // -- deleting, clipboard, duplicating -------------------------------------------------

    /// Elements plus everything that would dangle without them.
    fn expand_dependents(&self, keys: &[ElementKey]) -> Vec<ElementKey> {
        let mut out = keys.to_vec();
        let mut ids = self.node_ids(keys);
        let mut changed = true;
        while changed {
            changed = false;
            for el in &self.doc().elements {
                if out.contains(&el.key) {
                    continue;
                }
                let dangling = match &el.body {
                    Body::Edge(e) => ids.contains(e.src_id()) || ids.contains(e.dst_id()),
                    Body::Attributes(t) => ids.contains(&t.node),
                    Body::Node(n) => {
                        n.generalisation.as_ref().is_some_and(|g| ids.contains(endpoint_id(&g.supertype)))
                            || n.opts.get("of").is_some_and(|o| ids.contains(o))
                    }
                    _ => false,
                };
                if dangling {
                    out.push(el.key);
                    if let Some(id) = el.id() {
                        ids.insert(id.to_string());
                    }
                    changed = true;
                }
            }
        }
        out
    }

    pub fn delete_selection(&mut self) {
        let editable: Vec<ElementKey> = self
            .selection
            .iter()
            .copied()
            .filter(|k| self.doc().get(*k).is_some_and(|e| e.editable()))
            .collect();
        let doomed = self.expand_dependents(&editable);
        self.apply("Delete", |ed| {
            for k in doomed {
                if ed.doc().get(k).is_some() {
                    ed.session.doc_mut().remove(k);
                }
            }
            ed.selection.clear();
        });
        self.events.push(Event::SelectionChanged);
    }

    /// The selection as diagram source (nodes bring their attribute lists and the edges
    /// between selected nodes).
    pub fn clipboard_text(&self) -> String {
        let doc = self.doc();
        let sel: Vec<ElementKey> =
            self.selection.iter().copied().filter(|k| doc.get(*k).is_some_and(|e| e.editable())).collect();
        let ids = self.node_ids(&sel);
        let chosen: Vec<&crate::model::Element> = doc
            .elements
            .iter()
            .filter(|el| {
                sel.contains(&el.key)
                    || match &el.body {
                        Body::Attributes(t) => ids.contains(&t.node),
                        Body::Edge(e) => ids.contains(e.src_id()) && ids.contains(e.dst_id()),
                        _ => false,
                    }
            })
            .filter(|el| match &el.body {
                // nothing whose ends are not part of the copy
                Body::Edge(e) => ids.contains(e.src_id()) && ids.contains(e.dst_id()),
                Body::Attributes(t) => ids.contains(&t.node),
                Body::Node(n) => {
                    n.generalisation.as_ref().is_none_or(|g| ids.contains(endpoint_id(&g.supertype)))
                }
                _ => true,
            })
            .collect();
        chosen.iter().map(|e| e.text()).collect::<Vec<_>>().join("\n") + "\n"
    }

    /// What Copy puts on the clipboard (empty: nothing selected).
    pub fn copy(&mut self) -> String {
        if self.selection.is_empty() {
            return String::new();
        }
        self.message("Copied (as diagram source).");
        format!("{KIND_MARK}{}\n{}", self.doc().kind.name(), self.clipboard_text())
    }

    pub fn cut(&mut self) -> String {
        let text = self.copy();
        self.delete_selection();
        text
    }

    /// Pastes `text`: with `at` (cm), its middle goes there (from the clipboard); without, it
    /// moves by `offset` cm (duplicating).
    pub fn paste(&mut self, text: &str, at: Option<(f64, f64)>, offset: Option<(f64, f64)>) {
        // the mark says where it comes from; plain text by its commands
        let word: Option<String> = text
            .strip_prefix(KIND_MARK)
            .map(|rest| rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<String>())
            .filter(|w| !w.is_empty());
        let kind = match word {
            Some(w) => Kind::from_name(&w),
            None => kind_of_text(text),
        };
        let own = self.doc().kind;
        if let Some(k) = kind
            && k != own
        {
            let names = |k: Kind| if k == Kind::Er { "an ER" } else { "a GSM" };
            self.events.push(Event::Refused(format!(
                "This comes from {} diagram and can't be pasted into {} diagram, because they have different elements.",
                names(k),
                names(own)
            )));
            return;
        }
        let incoming = match Document::parse(text, own) {
            Ok(d) => d,
            Err(e) => {
                self.message(format!("Cannot paste: {e}"));
                return;
            }
        };
        let mut bodies: Vec<Body> =
            incoming.elements.into_iter().filter(|e| e.editable()).map(|e| e.body).collect();
        // connections need both their ends: in what's pasted, or already in the diagram
        let mut known: HashSet<String> = self.doc().ids().into_iter().map(str::to_string).collect();
        for b in &bodies {
            if let Body::Node(Node { id, .. }) | Body::Point(Point { id, .. }) = b {
                known.insert(id.clone());
            }
        }
        let before = bodies.len();
        bodies.retain(|b| match b {
            Body::Edge(e) => known.contains(e.src_id()) && known.contains(e.dst_id()),
            Body::Attributes(t) => known.contains(&t.node),
            _ => true,
        });
        let left_out = before - bodies.len();
        if bodies.is_empty() {
            self.message("Nothing to paste.");
            return;
        }
        let offset = match (offset, at) {
            (Some(o), _) => o,
            (None, Some(target)) => self.placing_offset(&bodies, target),
            (None, None) => (1.0, -1.0),
        };
        let taken: HashSet<String> = self.doc().ids().into_iter().map(str::to_string).collect();
        let mut rename: HashMap<String, String> = HashMap::new();
        for b in &bodies {
            if let Body::Node(Node { id, .. }) | Body::Point(Point { id, .. }) = b {
                let mut new = id.clone();
                let mut k = 1;
                while taken.contains(&new) || rename.values().any(|v| *v == new) {
                    k += 1;
                    new = format!("{}{k}", id.trim_end_matches(|c: char| c.is_ascii_digit()));
                }
                rename.insert(id.clone(), new);
            }
        }
        let refer = |r: &str| match r.split_once('.') {
            Some((ident, anchor)) => {
                format!("{}.{anchor}", rename.get(ident).cloned().unwrap_or_else(|| ident.to_string()))
            }
            None => rename.get(r).cloned().unwrap_or_else(|| r.to_string()),
        };
        let mv = |c: &Coord| match c {
            Coord::Numeric { x, y } => Coord::numeric(py_round(x + offset.0, 3), py_round(y + offset.1, 3)),
            raw => raw.clone(),
        };
        let mut added = Vec::new();
        self.apply("Paste", |ed| {
            for mut b in bodies {
                match &mut b {
                    Body::Node(n) => {
                        n.id = rename.get(&n.id).cloned().unwrap_or_else(|| n.id.clone());
                        if let Some(g) = &mut n.generalisation {
                            g.supertype = refer(&g.supertype);
                        }
                        if let Some(of) = n.opts.get("of").map(str::to_string) {
                            let new = rename.get(&of).cloned().unwrap_or(of);
                            n.opts.set("of", Some(&new));
                        }
                        n.pos = n.pos.as_ref().map(mv);
                    }
                    Body::Point(p) => {
                        p.id = rename.get(&p.id).cloned().unwrap_or_else(|| p.id.clone());
                        p.pos = p.pos.as_ref().map(mv);
                    }
                    Body::Edge(e) => {
                        e.src = refer(&e.src);
                        e.dst = refer(&e.dst);
                        if let Some(via) = e.opts.get("via").filter(|v| !v.is_empty()).map(str::to_string) {
                            // Python: Coord(p.x + dx, p.y + dy) without rounding
                            let moved: Vec<String> = parse_coord_list(&via)
                                .iter()
                                .map(|p| match p {
                                    Coord::Numeric { x, y } => {
                                        Coord::numeric(x + offset.0, y + offset.1).to_tex()
                                    }
                                    raw => raw.to_tex(),
                                })
                                .collect();
                            e.opts.set("via", Some(&moved.join(" ")));
                        }
                    }
                    Body::Attributes(t) => {
                        t.node = rename.get(&t.node).cloned().unwrap_or_else(|| t.node.clone())
                    }
                    Body::Note(n) => n.pos = n.pos.as_ref().map(mv),
                    Body::Subdomain(s) => {
                        s.points = s
                            .points
                            .iter()
                            .map(|c| match c {
                                Coord::Numeric { x, y } => Coord::numeric(x + offset.0, y + offset.1),
                                raw => raw.clone(),
                            })
                            .collect();
                    }
                    _ => {}
                }
                added.push(ed.add(b, None));
            }
            ed.selection = added.clone();
        });
        self.events.push(Event::SelectionChanged);
        if left_out > 0 {
            self.message(format!(
                "Pasted; left out {left_out} connection(s) whose ends are not in this diagram."
            ));
        }
    }

    /// How far to move these so that their middle is at the target.  If that would put
    /// something exactly on top of what's there, a little further.
    fn placing_offset(&self, bodies: &[Body], target: (f64, f64)) -> (f64, f64) {
        let pos_of = |b: &Body| match b {
            Body::Node(Node { pos, .. }) | Body::Point(Point { pos, .. }) | Body::Note(Note { pos, .. }) => {
                pos.clone().and_then(cm)
            }
            _ => None,
        };
        let mut pts: Vec<(f64, f64)> = Vec::new();
        for b in bodies {
            pts.extend(pos_of(b));
            if let Body::Subdomain(s) = b {
                pts.extend(s.points.iter().filter_map(|c| cm(c.clone())));
            }
        }
        if pts.is_empty() {
            return (0.0, 0.0);
        }
        let min = |f: fn(&(f64, f64)) -> f64| pts.iter().map(f).fold(f64::INFINITY, f64::min);
        let max = |f: fn(&(f64, f64)) -> f64| pts.iter().map(f).fold(f64::NEG_INFINITY, f64::max);
        let cx = (min(|p| p.0) + max(|p| p.0)) / 2.0;
        let cy = (min(|p| p.1) + max(|p| p.1)) / 2.0;
        let (mut dx, mut dy) = (self.snap(target.0, false) - cx, self.snap(target.1, false) - cy);
        let key = |x: f64, y: f64| {
            ((py_round(x, 3) * 1000.0).round() as i64, (py_round(y, 3) * 1000.0).round() as i64)
        };
        let taken: HashSet<(i64, i64)> =
            self.doc().elements.iter().filter_map(|e| pos_of(&e.body)).map(|(x, y)| key(x, y)).collect();
        let moved: Vec<(f64, f64)> = bodies.iter().filter_map(pos_of).collect();
        let mut guard = 0;
        while moved.iter().any(|(x, y)| taken.contains(&key(x + dx, y + dy))) && guard < 1000 {
            dx += 0.5;
            dy -= 0.5;
            guard += 1;
        }
        (dx, dy)
    }

    pub fn duplicate(&mut self) {
        if !self.selection.is_empty() {
            let text = self.clipboard_text();
            self.paste(&text, None, None);
        }
    }

    // -- selecting by rectangle -------------------------------------------------------------

    fn rubber_select(&mut self, a: Pt, b: Pt, add: bool) {
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        if x1 - x0 < 2.0 && y1 - y0 < 2.0 {
            return;
        }
        let (doc, laid) = self.session.parts();
        let hits: Vec<ElementKey> = laid
            .gfx
            .iter()
            .filter_map(|g| {
                let key = g.key?;
                let bb = crate::svg::bbox(g)?;
                (doc.get(key).is_some_and(|e| e.editable())
                    && x0 <= bb.0
                    && bb.2 <= x1
                    && y0 <= bb.1
                    && bb.3 <= y1)
                    .then_some(key)
            })
            .collect();
        let mut sel = if add { self.selection.clone() } else { vec![] };
        for h in hits {
            if !sel.contains(&h) {
                sel.push(h);
            }
        }
        self.set_selection(sel);
    }

    // -- context menu -------------------------------------------------------------------------

    /// A right click at (x, y): the element under it becomes the selection (unless it is in
    /// it already).  Returns what the menu offers: `edge`, `node` (one that can have attributes),
    /// `one`, `many`, or `none`.
    pub fn context(&mut self, x: f64, y: f64) -> &'static str {
        if let Some(e) = self.element_at(x, y)
            && !self.selection.contains(&e)
        {
            self.set_selection(vec![e]);
        }
        match self.selection.as_slice() {
            [] => "none",
            [one] => match self.doc().get(*one).map(|e| &e.body) {
                Some(Body::Edge(_)) => "edge",
                Some(Body::Node(n))
                    if self.doc().kind == Kind::Er
                        && n.generalisation.is_none()
                        && matches!(n.command.as_str(), "entity" | "relationship" | "partof") =>
                {
                    "node"
                }
                _ => "one",
            },
            _ => "many",
        }
    }

    /// The selected edge without bends and way points.
    pub fn straighten(&mut self) {
        let Some(&key) = self.selection.first() else { return };
        self.apply("Straighten", |ed| {
            if let Some(el) = ed.body_mut(key) {
                if let Body::Edge(e) = &mut el.body {
                    for k in ["bend", "bend left", "bend right", "out", "in", "looseness", "via", "ortho"] {
                        e.opts.remove(k);
                    }
                }
                el.touch();
            }
        });
    }

    /// An attribute list for the selected node.
    pub fn add_attribute_list(&mut self) {
        let Some(&key) = self.selection.first() else { return };
        let id = self.doc().get(key).and_then(|e| e.id()).map(str::to_string);
        let Some(shape) = id.and_then(|i| self.session.laid().shapes.get(&i).cloned()) else { return };
        self.place("attribute tree", shape.cx, shape.cy, true);
    }

    // -- the overlay: selection, handles, previews --------------------------------------------

    fn outline_of(&mut self, key: ElementKey) -> Option<String> {
        let (doc, laid) = self.session.parts();
        let g = laid.gfx_for(key)?;
        let el = doc.get(key)?;
        if matches!(el.body, Body::Node(_) | Body::Point(_))
            && let Some(shape) = &g.shape
        {
            let ops = shape.outline();
            if !ops.is_empty() {
                return Some(path_d(&ops));
            }
        }
        if !g.segments.is_empty() {
            let mut ops: Vec<PathOp> = Vec::new();
            if let Some(first) = g.segments.first() {
                ops.push(PathOp::Move(first.p0));
            }
            ops.extend(g.segments.iter().map(Segment::ops));
            for s in &g.stub {
                ops.push(PathOp::Move(s.p0));
                ops.push(PathOp::Line(s.p3));
            }
            return Some(format!("band:{}", path_d(&ops)));
        }
        let (x0, y0, x1, y1) = crate::svg::bbox(g)?;
        Some(format!(
            "M{} {} L{} {} L{} {} L{} {} Z",
            n(x0 - 1.0),
            n(y0 - 1.0),
            n(x1 + 1.0),
            n(y0 - 1.0),
            n(x1 + 1.0),
            n(y1 + 1.0),
            n(x0 - 1.0),
            n(y1 + 1.0)
        ))
    }

    fn mark(out: &mut String, d: &str, color: &str, width: f64, dashed: bool, band_pt: f64) {
        let dash = if dashed { r#" stroke-dasharray="4 3""# } else { "" };
        if let Some(path) = d.strip_prefix("band:") {
            let _ = write!(
                out,
                r#"<path d="{path}" fill="none" stroke="{color}" stroke-opacity="0.35" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>"#,
                n(band_pt)
            );
        } else {
            let _ = write!(
                out,
                r#"<path d="{d}" fill="none" stroke="{color}" stroke-width="{width}" vector-effect="non-scaling-stroke"{dash}/>"#
            );
        }
    }

    /// Elements with problems: a dashed outline in the colour of the worst (not for hints), and
    /// a round marker with ! or i in the top right corner.
    fn badges(&mut self, out: &mut String) {
        let px = self.px;
        for (key, severity) in crate::problems::worst(self, self.show_hints) {
            let color = crate::problems::severity_color(severity);
            if severity != crate::rules::Severity::Hint
                && let Some(d) = self.outline_of(key)
            {
                Self::mark(out, &d, color, 1.5, true, 6.0);
            }
            let laid = self.session.laid();
            let Some((x0, y0, x1, y1)) = laid.gfx_for(key).and_then(crate::svg::bbox) else { continue };
            let (x0, y0, x1, y1) = (x0 - 4.0, y0 - 4.0, x1 + 4.0, y1 + 4.0);
            let r = (7.0 * px).min((y1 - y0) / 2.0).min((x1 - x0) / 2.0);
            let (cx, cy) = (x1 - r, y1 - r);
            let mark = if severity == crate::rules::Severity::Hint { "i" } else { "!" };
            let _ = write!(
                out,
                r##"<g class="badge"><circle cx="{}" cy="{}" r="{}" fill="{color}"/><text transform="translate({} {}) scale({} {})" font-family="sans-serif" font-weight="bold" font-size="{}" text-anchor="middle" dominant-baseline="central" fill="#fff">{mark}</text></g>"##,
                n(cx),
                n(cy),
                n(r),
                n(cx),
                n(cy),
                n(px),
                n(-px),
                n(r * 1.5 / px)
            );
        }
    }

    /// What goes on top of the diagram, in its coordinates (y up): the selection, the element
    /// under the pointer, handles, the rubber band and the connection being drawn.
    pub fn overlay(&mut self) -> String {
        let mut out = String::new();
        let px = self.px;
        self.badges(&mut out);
        if let Some(h) = self.hover.filter(|h| !self.selection.contains(h))
            && let Some(d) = self.outline_of(h)
        {
            Self::mark(&mut out, &d, HOVER, 1.0, true, 6.0);
        }
        for k in self.selection.clone() {
            if let Some(d) = self.outline_of(k) {
                Self::mark(&mut out, &d, SELECT, 2.0, false, 6.0);
            }
        }
        match self.drag.clone() {
            Some(
                Drag::Rubber { start: (ax, ay), now: (bx, by) }
                | Drag::Subdomain { start: (ax, ay), now: (bx, by), .. },
            ) => {
                let _ = write!(
                    out,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{SELECT}" fill-opacity="0.1" stroke="{SELECT}" stroke-width="1" stroke-dasharray="4 3" vector-effect="non-scaling-stroke"/>"#,
                    n(ax.min(bx)),
                    n(ay.min(by)),
                    n((ax - bx).abs()),
                    n((ay - by).abs())
                );
            }
            Some(Drag::Connect { src, now, target, ok, .. }) => {
                let src_id = self.doc().get(src).and_then(|e| e.id()).map(str::to_string);
                let (doc, laid) = self.session.parts();
                let _ = doc;
                if let Some(s) = src_id.and_then(|i| laid.shapes.get(&i)) {
                    let mut to = now;
                    let color = match target {
                        Some(_) if ok => "#00aa3c",
                        Some(_) => "#dc1e1e",
                        None => SELECT,
                    };
                    if let Some(t) = target
                        && let Some(g) = laid.gfx_for(t)
                        && let Some(shape) = &g.shape
                    {
                        let ops = shape.outline();
                        let d = if ops.is_empty() {
                            format!(
                                "M{} {} L{} {}",
                                n(shape.cx - 3.0),
                                n(shape.cy - 3.0),
                                n(shape.cx + 3.0),
                                n(shape.cy + 3.0)
                            )
                        } else {
                            path_d(&ops)
                        };
                        let _ = write!(
                            out,
                            r#"<path d="{d}" fill="{color}" fill-opacity="0.16" stroke="{color}" stroke-width="3" vector-effect="non-scaling-stroke"/>"#
                        );
                        to = shape.center();
                    }
                    let (sx, sy) = s.border_toward(to.0, to.1);
                    let _ = write!(
                        out,
                        r#"<path d="M{} {} L{} {}" stroke="{color}" stroke-width="1.5" stroke-dasharray="5 3" vector-effect="non-scaling-stroke"/>"#,
                        n(sx),
                        n(sy),
                        n(to.0),
                        n(to.1)
                    );
                }
            }
            _ => {}
        }
        let r = 4.5 * px;
        for (name, (hx, hy)) in self.handles() {
            let common = format!(r#"stroke="{SELECT}" stroke-width="1" vector-effect="non-scaling-stroke""#);
            if name == "bend" {
                let _ = write!(
                    out,
                    r#"<circle cx="{}" cy="{}" r="{}" fill="{SELECT}" {common}/>"#,
                    n(hx),
                    n(hy),
                    n(r)
                );
            } else if name.starts_with('m') {
                // add-a-corner handle
                let _ = write!(
                    out,
                    r##"<circle cx="{}" cy="{}" r="{}" fill="#fff" {common}/>"##,
                    n(hx),
                    n(hy),
                    n(r * 0.8)
                );
            } else {
                let fill = if name == "start" || name == "end" { "#fff" } else { SELECT };
                let _ = write!(
                    out,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{fill}" {common}/>"#,
                    n(hx - r),
                    n(hy - r),
                    n(2.0 * r),
                    n(2.0 * r)
                );
            }
        }
        out
    }
}

fn n(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

fn path_d(ops: &[PathOp]) -> String {
    let mut d = String::new();
    for op in ops {
        let _ = match op {
            PathOp::Move(p) => write!(d, "M{} {} ", n(p.0), n(p.1)),
            PathOp::Line(p) => write!(d, "L{} {} ", n(p.0), n(p.1)),
            PathOp::Curve(a, b, p) => {
                write!(d, "C{} {} {} {} {} {} ", n(a.0), n(a.1), n(b.0), n(b.1), n(p.0), n(p.1))
            }
            PathOp::Close => write!(d, "Z "),
        };
    }
    d.trim_end().to_string()
}
