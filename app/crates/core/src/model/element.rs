//! The elements of a diagram file: one per command.
//!
//! Every element remembers the exact text it was read from.  As long as it isn't changed, it
//! is written back verbatim, so comments, spacing and hand-made formatting survive loading
//! and saving.  A changed element is written in the canonical one-line form.

use super::{AttrItem, Coord, Options};

/// `student.north` -> `student`.
pub fn endpoint_id(reference: &str) -> &str {
    crate::text::strip(reference.split('.').next().unwrap_or(""))
}

/// A stable handle for an element of a document (survives inserting and removing others).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ElementKey(pub u64);

/// The text around an element and whether it changed.
#[derive(Clone, Debug, PartialEq)]
pub struct Meta {
    /// the original text; `None` for new elements
    pub source: Option<String>,
    pub dirty: bool,
    /// white space and comment lines before the element
    pub leading: String,
    /// the rest of its own line: spaces, an end-of-line comment and the line break
    pub trailing: String,
}

impl Default for Meta {
    fn default() -> Self {
        Meta { source: None, dirty: true, leading: String::new(), trailing: "\n".into() }
    }
}

/// Every element with an id and a shape: entity, relationship, (Chen) attribute, PART_OF,
/// abstract / representable type, subtype, aggregation, grouping, and IS_A / CAN_BE.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Node {
    pub command: String,
    pub id: String,
    pub pos: Option<Coord>,
    /// `None` for PART_OF, IS_A and CAN_BE, which have no label argument
    pub label: Option<String>,
    pub opts: Options,
    /// set for IS_A / CAN_BE
    pub generalisation: Option<Generalisation>,
}

/// What an IS_A / CAN_BE adds to a node: the arrow to its supertype, and its type.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Generalisation {
    pub supertype: String,
    pub constraint: Option<String>,
}

/// An invisible junction or way point.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Point {
    pub id: String,
    pub pos: Option<Coord>,
}

/// `\link` (ER and GSM) and GSM's edge commands.  `label` is the cardinality (ER) or the edge
/// label (GSM).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edge {
    pub command: String,
    pub src: String,
    pub dst: String,
    pub label: Option<String>,
    pub opts: Options,
}

impl Edge {
    pub fn src_id(&self) -> &str {
        endpoint_id(&self.src)
    }

    pub fn dst_id(&self) -> &str {
        endpoint_id(&self.dst)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AttributeTree {
    pub node: String,
    pub items: Vec<AttrItem>,
    pub opts: Options,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Note {
    pub pos: Option<Coord>,
    pub content: String,
    pub opts: Options,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Subdomain {
    pub label: String,
    pub points: Vec<Coord>,
    pub opts: Options,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub a: Coord,
    pub b: Coord,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Setup {
    pub opts: Options,
}

/// Anything the editor doesn't understand (plain TikZ, unknown commands, stray text): kept
/// verbatim and never changed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Raw {
    pub command: String,
    pub raw: String,
}

/// What an element is.
#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Node(Node),
    Point(Point),
    Edge(Edge),
    Attributes(AttributeTree),
    Note(Note),
    Subdomain(Subdomain),
    Grid(Grid),
    Setup(Setup),
    Raw(Raw),
}

impl Body {
    pub fn command(&self) -> &str {
        match self {
            Body::Node(n) => &n.command,
            Body::Point(_) => "point",
            Body::Edge(e) => &e.command,
            Body::Attributes(_) => "attributes",
            Body::Note(_) => "note",
            Body::Subdomain(_) => "subdomain",
            Body::Grid(_) => "grid",
            Body::Setup(_) => "setup",
            Body::Raw(r) => &r.command,
        }
    }

    /// The canonical form.
    pub fn to_tex(&self) -> String {
        match self {
            Body::Node(n) => node_tex(n),
            Body::Point(p) => {
                format!("\\point{{{}}}{}", p.id, p.pos.as_ref().map_or(String::new(), Coord::to_tex))
            }
            Body::Edge(e) => {
                let mut s = format!("\\{}{}{{{}}}{{{}}}", e.command, e.opts.to_tex(), e.src, e.dst);
                if let Some(l) = &e.label {
                    s += &format!("{{{l}}}");
                }
                s
            }
            Body::Attributes(t) => {
                let items: Vec<String> = t.items.iter().map(AttrItem::to_tex).collect();
                format!("\\attributes{}{{{}}}{{{}}}", t.opts.to_tex(), t.node, items.join(", "))
            }
            Body::Note(n) => format!(
                "\\note{}{}{{{}}}",
                n.opts.to_tex(),
                n.pos.as_ref().map_or(String::new(), Coord::to_tex),
                n.content
            ),
            Body::Subdomain(s) => {
                let pts: Vec<String> = s.points.iter().map(Coord::to_tex).collect();
                format!("\\subdomain{}{{{}}}{{{}}}", s.opts.to_tex(), s.label, pts.join(" "))
            }
            Body::Grid(g) => format!("\\grid{}{}", g.a.to_tex(), g.b.to_tex()),
            Body::Setup(s) => {
                let o = s.opts.to_tex();
                let inner = o.get(1..o.len().saturating_sub(1)).unwrap_or("");
                format!("\\setup{{{inner}}}")
            }
            Body::Raw(r) => r.raw.clone(),
        }
    }
}

fn node_tex(n: &Node) -> String {
    let mut s = format!("\\{}{}{{{}}}", n.command, n.opts.to_tex(), n.id);
    if let Some(p) = &n.pos {
        s += &p.to_tex();
    }
    match &n.generalisation {
        Some(g) => {
            s += &format!("{{{}}}", g.supertype);
            if let Some(c) = g.constraint.as_deref().filter(|c| !c.is_empty()) {
                s += &format!("{{{c}}}");
            }
        }
        None => {
            if let Some(l) = &n.label {
                s += &format!("{{{l}}}");
            }
        }
    }
    s
}

/// An element of a document.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub key: ElementKey,
    pub meta: Meta,
    pub body: Body,
}

impl Element {
    pub fn command(&self) -> &str {
        self.body.command()
    }

    /// Marks the element as changed: it will be written in canonical form.
    pub fn touch(&mut self) {
        self.meta.dirty = true;
    }

    pub fn to_tex(&self) -> String {
        self.body.to_tex()
    }

    /// The text written to the file.
    pub fn text(&self) -> String {
        match (&self.meta.source, self.meta.dirty) {
            (Some(src), false) => src.clone(),
            _ => self.to_tex(),
        }
    }

    /// Whether the editor may change it (unknown commands are kept as they are).
    pub fn editable(&self) -> bool {
        !matches!(self.body, Body::Raw(_))
    }

    pub fn as_node(&self) -> Option<&Node> {
        match &self.body {
            Body::Node(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_edge(&self) -> Option<&Edge> {
        match &self.body {
            Body::Edge(e) => Some(e),
            _ => None,
        }
    }

    /// The id of a node or point.
    pub fn id(&self) -> Option<&str> {
        match &self.body {
            Body::Node(n) => Some(&n.id),
            Body::Point(p) => Some(&p.id),
            _ => None,
        }
    }
}
