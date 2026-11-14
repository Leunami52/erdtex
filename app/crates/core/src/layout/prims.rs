//! What the layout draws: paths and text, collected per element of the document.

use std::collections::BTreeMap;

use crate::geometry::{PathOp, Pt, Segment, Shape};
use crate::latex_text::Run;
use crate::model::{AttrItem, ElementKey};

pub type Rgb = (f64, f64, f64);
pub const WHITE: Rgb = (1.0, 1.0, 1.0);
pub const BLACK: Rgb = (0.0, 0.0, 0.0);

#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    /// `None`: not drawn (`draw=none`)
    pub color: Option<Rgb>,
    pub width: f64,
    /// (on, off) in pt
    pub dash: Option<(f64, f64)>,
    /// the gap of a double line
    pub double: Option<f64>,
}

impl Stroke {
    pub fn new(color: Option<Rgb>, width: f64) -> Self {
        Stroke { color, width, dash: None, double: None }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arrow {
    pub tip: Pt,
    /// direction of travel at the tip, in degrees
    pub angle: f64,
    pub length: f64,
    pub width: f64,
    pub color: Option<Rgb>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathPrim {
    pub segs: Vec<PathOp>,
    pub stroke: Option<Stroke>,
    pub fill: Option<Rgb>,
    /// the colour of a dotted background
    pub pattern: Option<Rgb>,
    pub arrows: Vec<Arrow>,
}

impl PathPrim {
    pub fn stroked(segs: Vec<PathOp>, stroke: Stroke) -> Self {
        PathPrim { segs, stroke: Some(stroke), fill: None, pattern: None, arrows: Vec::new() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineLayout {
    /// baseline offset from the first baseline
    pub y: f64,
    /// left offset inside the box
    pub x: f64,
    pub width: f64,
    /// (x offset, run, width)
    pub runs: Vec<(f64, Run, f64)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayout {
    pub width: f64,
    /// above the first baseline
    pub height: f64,
    /// below the first baseline (including the other lines)
    pub depth: f64,
    pub lines: Vec<LineLayout>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextPrim {
    pub layout: TextLayout,
    /// the centre of the text box
    pub cx: f64,
    pub cy: f64,
    /// degrees, around the centre
    pub rotation: f64,
    /// the colour of runs without `\color`
    pub color: Option<Rgb>,
    /// a background box (with `inner` around the text)
    pub bg: Option<Rgb>,
    pub inner: f64,
}

impl TextPrim {
    pub fn at(layout: TextLayout, cx: f64, cy: f64, color: Option<Rgb>) -> Self {
        TextPrim { layout, cx, cy, rotation: 0.0, color, bg: None, inner: 0.0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Prim {
    Path(PathPrim),
    Text(TextPrim),
}

/// One oval of an attribute list drawn in Chen style, for dragging it.
#[derive(Clone, Debug, PartialEq)]
pub struct Oval {
    /// the item's place in the list, counted depth first (as `flatten_items`)
    pub row: usize,
    pub item: AttrItem,
    pub shape: Shape,
    /// what its position is relative to
    pub origin: Pt,
}

/// Named points and outlines for editing.
#[derive(Clone, Debug, PartialEq)]
pub enum Handle {
    Point(Pt),
    Rect(f64, f64, f64, f64),
    Segments(Vec<Segment>),
    Ovals(Vec<Oval>),
}

/// Everything drawn for one element of the document.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Gfx {
    pub key: Option<ElementKey>,
    pub prims: Vec<Prim>,
    pub z: f64,
    /// a node's shape (selection, connections)
    pub shape: Option<Shape>,
    /// an edge's geometry
    pub segments: Vec<Segment>,
    /// crow's foot: the straight piece at the entity
    pub stub: Vec<Segment>,
    pub handles: BTreeMap<String, Handle>,
    pub warnings: Vec<String>,
}

impl Gfx {
    pub fn new(key: ElementKey, z: f64) -> Self {
        Gfx { key: Some(key), z, ..Default::default() }
    }

    pub fn path(&mut self, p: PathPrim) {
        self.prims.push(Prim::Path(p));
    }

    pub fn text(&mut self, t: TextPrim) {
        self.prims.push(Prim::Text(t));
    }
}
