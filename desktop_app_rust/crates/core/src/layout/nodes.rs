//! Nodes, IS_A / CAN_BE, and labels beside shapes.

use super::position::float_opt;
use super::prims::{Gfx, PathPrim, Rgb, Stroke, TextPrim, WHITE};
use super::{Fallible, Handle, LINE_WIDTHS, Layout, length, octant_anchor, plain_length};
use crate::colors::{parse_color, tint};
use crate::generalisation::words;
use crate::geometry::{self as g, CM, COMPASS, PathOp, Shape};
use crate::latex_text::{Fmt, font_switches};
use crate::model::{Node, Options, Point};
use crate::text::py_mod;

/// A label beside a shape: a generalisation's type, or the label of an aggregation / grouping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BesideKind {
    Constraint,
    ConstructorLabel,
}

/// The paint of a node after its options: fill, stroke colour, line width, dash, double.
pub(super) struct Paint {
    pub fill: Option<Rgb>,
    pub draw: Option<Rgb>,
    pub width: f64,
    pub dash: Option<(f64, f64)>,
    pub double: Option<f64>,
}

const NONE_STRIP: &str = "'NoneType' object has no attribute 'strip'";
const NONE_LEN: &str = "object of type 'NoneType' has no len()";

/// `font=...`: a font switch; the option without a value fails as it did in Python.
pub(super) fn font_option(opts: &Options, fmt: Fmt) -> Fallible<Fmt> {
    if !opts.contains("font") {
        return Ok(fmt);
    }
    match opts.get("font") {
        Some(spec) => Ok(font_switches(spec, &fmt)),
        None => Err(NONE_LEN.to_string()),
    }
}

/// Python's `a or b`: `a` unless it's missing or empty.
pub(super) fn or<'x>(a: Option<&'x str>, b: Option<&'x str>) -> Option<&'x str> {
    a.filter(|s| !s.is_empty()).or(b)
}

impl Layout<'_> {
    pub(super) fn common_paint(&self, opts: &Options, fill: &str, draw: &str, lw: f64) -> Fallible<Paint> {
        let mut fill_c: Option<String> = Some(fill.to_string());
        if opts.contains("color") || opts.contains("colour") {
            let value = or(opts.get("color"), opts.get("colour")).ok_or_else(|| NONE_STRIP.to_string())?;
            fill_c = Some(tint(value));
        }
        if opts.contains("fill") {
            fill_c = or(opts.get("fill"), fill_c.as_deref()).map(str::to_string);
        }
        let draw_c = or(opts.get("draw"), Some(draw)).unwrap_or(draw);
        let mut width = lw;
        for (k, v) in LINE_WIDTHS {
            if opts.contains(k) {
                width = *v;
            }
        }
        if opts.contains("line width") {
            width = plain_length(opts.get("line width"), width);
        }
        let dash = if opts.contains("dashed") {
            Some((3.0, 3.0))
        } else if opts.contains("dotted") {
            Some((0.4, 2.0))
        } else {
            None
        };
        let double = opts.contains("double").then(|| plain_length(opts.get("double distance"), 0.6));
        Ok(Paint {
            fill: parse_color(fill_c.as_deref(), Some(WHITE)),
            draw: parse_color(Some(draw_c), Some(super::BLACK)),
            width,
            dash,
            double,
        })
    }

    pub(super) fn register(&mut self, ident: &str, shape: Shape, kind: &str) {
        self.shapes.insert(ident.to_string(), shape);
        self.types.insert(ident.to_string(), kind.to_string());
    }

    pub(super) fn layout_node(&mut self, e: &Node, gfx: &mut Gfx, isa: bool) -> Fallible {
        let er = self.is_er();
        let k = e.command.as_str();
        let opts = &e.opts;
        // --- what kind of shape
        let pre = match k {
            "entity" if er => "entity",
            "relationship" | "partof" if er => "rel",
            "attribute" if er => "attribute",
            "abstract" => "abstract",
            "representable" => "repr",
            "subtype" => "sub",
            _ => "aggr", // aggregation / grouping
        };
        // --- the label
        let mut label = e.label.clone().unwrap_or_default();
        if k == "partof" {
            label = self.style.keyword("partof label", self.language);
        }
        let (font_key, text_key) = if er && pre == "attribute" {
            if opts.contains("key") {
                label = self.key_label(&label, &self.owner_of_attribute(&e.id));
            }
            label += &self.type_suffix(opts.get("type"));
            (Some("attr font".to_string()), Some("attr text".to_string()))
        } else if pre == "aggr" {
            (None, None)
        } else {
            (Some(format!("{pre} font")), Some(format!("{pre} text")))
        };
        let dep = opts.get("dependency");
        if let Some(dep) = dep {
            label = format!(
                "{{\\color{{{}}}{} {dep}}}\\\\[-2pt]{label}",
                self.style.try_get("dependency color")?,
                self.style.try_get("dependency font")?
            );
        }
        let fmt = match &font_key {
            Some(key) => self.style.font(key, None),
            None => Fmt::default(),
        };
        let fmt = font_option(opts, fmt)?;
        let text_default = text_key.as_deref().map_or("black", |k| self.style.get(k));
        let text_color = parse_color(
            Some(or(opts.get("text"), Some(text_default)).unwrap_or(text_default)),
            Some(super::BLACK),
        );
        let lay = self.text_box(if pre != "aggr" { &label } else { "" }, &fmt);
        let (tw, th) = (lay.width, lay.height + lay.depth);
        // --- paint
        let (fill, draw, lw0) = match pre {
            "aggr" => (
                self.style.get("aggr fill"),
                self.style.get("aggr draw"),
                self.style.length("aggr line width"),
            ),
            "attribute" => (
                self.style.get("attribute fill"),
                self.style.get("attribute draw"),
                self.style.length("edge line width"),
            ),
            _ => (
                self.style.get(&format!("{pre} fill")),
                self.style.get(&format!("{pre} draw")),
                self.style.length(&format!("{pre} line width")),
            ),
        };
        let (fill, draw) = (fill.to_string(), draw.to_string());
        let mut paint = self.common_paint(opts, &fill, &draw, lw0)?;
        if opts.is_on("weak") || opts.contains("dependency") && dep.is_some() {
            paint.double = Some(if er { self.style.length("weak double distance") } else { 2.0 });
        }
        if er && pre == "attribute" {
            if opts.contains("derived") {
                paint.dash = Some((3.0, 3.0));
            }
            if opts.contains("multivalued") {
                paint.double = Some(1.5);
            }
        }
        // --- size
        let inner = opts.contains("inner sep").then(|| plain_length(opts.get("inner sep"), 0.0));
        let st = &self.style;
        let mlen = |key: &str, default_key: &str| -> f64 {
            if opts.contains(key) {
                length(opts.get(key), 0.0)
            } else if opts.contains("minimum size") {
                length(opts.get("minimum size"), 0.0)
            } else if default_key.is_empty() {
                0.0
            } else {
                st.length(default_key)
            }
        };
        let lw = paint.width;
        // only a diamond reads `aspect` (an IS_A drawn as a hexagon ignores it, even if invalid)
        let hexagon = isa && st.get("isa shape") == "hexagon";
        let aspect = if pre == "rel" && !hexagon {
            float_opt(opts.get("aspect"), st.number("rel aspect"))?
        } else {
            0.0
        };
        let make = |cx: f64, cy: f64| -> Shape {
            let inner_or = |d: f64| inner.unwrap_or(d);
            match pre {
                "entity" => g::rect_shape(
                    cx,
                    cy,
                    tw,
                    th,
                    inner.unwrap_or_else(|| st.length("entity inner sep")),
                    mlen("minimum width", "entity minimum width"),
                    mlen("minimum height", "entity minimum height"),
                    lw,
                    if opts.contains("rounded corners") {
                        length(opts.get("rounded corners"), 4.0)
                    } else {
                        0.0
                    },
                ),
                "rel" if isa && st.get("isa shape") == "hexagon" => g::hexagon_shape(
                    cx,
                    cy,
                    tw,
                    th,
                    inner_or(2.0),
                    mlen("minimum width", "isa minimum width"),
                    mlen("minimum height", "isa minimum height"),
                    st.number("isa slant"),
                    lw,
                ),
                "rel" => g::diamond_shape(
                    cx,
                    cy,
                    tw,
                    th,
                    inner_or(2.0),
                    mlen("minimum width", "rel minimum width"),
                    mlen("minimum height", "rel minimum height"),
                    aspect,
                    lw,
                ),
                "attribute" if opts.contains("composite") => g::rect_shape(
                    cx,
                    cy,
                    tw,
                    th,
                    inner_or(2.0),
                    mlen("minimum width", "attribute minimum width"),
                    mlen("minimum height", "attribute minimum height"),
                    lw,
                    0.3 * CM,
                ),
                "attribute" => g::ellipse_shape(
                    cx,
                    cy,
                    tw,
                    th,
                    inner_or(2.0),
                    mlen("minimum width", "attribute minimum width"),
                    mlen("minimum height", "attribute minimum height"),
                    lw,
                ),
                "abstract" => g::triangle_shape(
                    cx,
                    cy,
                    tw,
                    th,
                    inner_or(1.0),
                    mlen("minimum width", "abstract minimum width"),
                    mlen("minimum height", "abstract minimum height"),
                    st.number("abstract apex angle"),
                    lw,
                ),
                "repr" => g::rect_shape(
                    cx,
                    cy,
                    tw,
                    th,
                    inner_or(5.0),
                    mlen("minimum width", "repr minimum width"),
                    mlen("minimum height", "repr minimum height"),
                    lw,
                    st.length("repr corner radius"),
                ),
                "sub" => g::circle_shape(
                    cx,
                    cy,
                    tw,
                    th,
                    inner_or(2.0),
                    mlen("minimum size", "sub minimum size"),
                    lw,
                ),
                _ => {
                    let size = if opts.contains("minimum size") {
                        length(opts.get("minimum size"), 0.0)
                    } else {
                        st.length("aggr size")
                    };
                    g::circle_shape(cx, cy, 0.0, 0.0, 0.0, size, lw)
                }
            }
        };
        let probe = make(0.0, 0.0);
        let (cx, cy) = self.node_position(e.pos.as_ref(), opts, &probe, gfx);
        let shape = make(cx, cy);
        self.register(&e.id, shape.clone(), k);
        gfx.shape = Some(shape.clone());
        gfx.handles.insert("center".into(), Handle::Point((cx, cy)));
        let stroke =
            paint.draw.map(|c| Stroke { color: Some(c), width: lw, dash: paint.dash, double: paint.double });
        gfx.path(PathPrim {
            segs: shape.outline(),
            stroke,
            fill: paint.fill,
            pattern: None,
            arrows: Vec::new(),
        });
        if pre == "aggr" {
            let line = Stroke::new(parse_color(Some(self.style.get("aggr draw")), Some(super::BLACK)), lw);
            let (a1, a2) = (shape.anchor("north west"), shape.anchor("south east"));
            let (b1, b2) = (shape.anchor("north east"), shape.anchor("south west"));
            gfx.path(PathPrim::stroked(
                vec![PathOp::Move(a1), PathOp::Line(a2), PathOp::Move(b1), PathOp::Line(b2)],
                line.clone(),
            ));
            if k == "grouping" {
                let (w1, w2) = (shape.anchor("west"), shape.anchor("east"));
                gfx.path(PathPrim::stroked(vec![PathOp::Move(w1), PathOp::Line(w2)], line));
            }
            if let Some(l) = e.label.as_deref().filter(|l| !l.is_empty()) {
                let lpos = or(opts.get("label pos"), Some(self.style.get("aggr label pos")))
                    .unwrap_or("auto")
                    .to_string();
                if lpos == "auto" {
                    self.deferred_conlabels.push((e.id.clone(), l.to_string(), self.gfx.len()));
                } else {
                    self.constructor_label(&e.id, l, &lpos, gfx);
                }
            }
        } else {
            // TikZ centres the text box (height + depth) on the node's centre
            gfx.text(TextPrim::at(lay, cx, cy, text_color));
        }
        if er
            && pre == "attribute"
            && let Some(of) = opts.get("of").filter(|o| !o.is_empty())
        {
            self.draw_edge(gfx, of, &e.id, &Options::new(), "", None, false, "link")?;
        }
        Ok(())
    }

    fn constructor_label(&self, ident: &str, text: &str, lpos: &str, gfx: &mut Gfx) {
        let Some(shape) = self.shapes.get(ident) else { return };
        let lay = self.text_box(text, &self.style.font("aggr label font", None));
        let (x, y) = shape.anchor(lpos);
        let a = COMPASS.iter().find(|(n, _)| *n == lpos).map_or(0.0, |(_, a)| *a);
        let color = parse_color(Some(self.style.get("aggr label color")), Some(super::BLACK));
        gfx.text(self.place_text(lay, x, y, octant_anchor(a + 180.0), 2.0, color, 0.0, None));
    }

    pub(super) fn layout_point(&mut self, e: &Point, gfx: &mut Gfx) {
        let p = self.coord(e.pos.as_ref()).unwrap_or((0.0, 0.0));
        let shape = g::point_shape(p.0, p.1);
        self.register(&e.id, shape.clone(), "point");
        gfx.shape = Some(shape);
        gfx.handles.insert("center".into(), Handle::Point(p));
    }

    // ------------------------------------------------------------------ generalisation
    pub(super) fn layout_isa(&mut self, e: &Node, gfx: &mut Gfx) -> Fallible {
        let Some(gen_) = &e.generalisation else { return Ok(()) };
        let key = if e.command == "isa" { "isa label" } else { "canbe label" };
        let mut label = self.style.keyword(key, self.language);
        let mut cpos = or(e.opts.get("constraint pos"), Some(self.style.get("constraint pos")))
            .unwrap_or("")
            .to_string();
        let mut constraint = gen_.constraint.clone();
        if self.style.get("constraint style") == "words" {
            // words don't fit inside
            constraint = words(constraint.as_deref());
            if cpos == "inside" {
                cpos = "auto".into();
            }
        }
        let constraint = constraint.filter(|c| !c.is_empty());
        if let Some(c) = &constraint
            && cpos == "inside"
        {
            label = format!(
                "{label}\\\\[-2pt]{{\\color{{{}}}{} {c}}}",
                self.style.get("constraint color"),
                self.style.get("constraint font")
            );
        }
        let node = Node {
            command: "relationship".into(),
            id: e.id.clone(),
            pos: e.pos.clone(),
            label: Some(label),
            opts: e.opts.clone(),
            generalisation: None,
        };
        self.layout_node(&node, gfx, true)?;
        self.types.insert(e.id.clone(), e.command.clone());
        let shape = gfx.shape.clone();
        // the arrow to the supertype, the cardinality near the supertype
        let sup = e.opts.get("super card").unwrap_or("").to_string();
        self.draw_edge(gfx, &e.id, &gen_.supertype, &Options::new(), &sup, Some('b'), true, "link")?;
        if let Some(c) = &constraint
            && cpos != "inside"
        {
            if cpos == "auto" {
                self.deferred_constraints.push((e.id.clone(), c.clone(), self.gfx.len()));
            } else {
                self.beside_label(&e.id, c, &cpos, gfx, BesideKind::Constraint);
            }
        }
        if let Some(sub) = e.opts.get("sub card").filter(|s| !s.is_empty())
            && let Some(shape) = shape
        {
            let lay = self.text_box(sub, &self.style.font("card font", None));
            let (x, y) = shape.anchor("south");
            let color = parse_color(Some(self.style.get("card text")), Some(super::BLACK));
            gfx.text(self.place_text(lay, x + 1.0, y, "north west", 1.5, color, 0.0, None));
        }
        Ok(())
    }

    fn beside_label(&self, ident: &str, text: &str, pos: &str, gfx: &mut Gfx, kind: BesideKind) {
        let Some(shape) = self.shapes.get(ident) else { return };
        let (font, color) = match kind {
            BesideKind::Constraint => ("constraint font", "constraint color"),
            BesideKind::ConstructorLabel => ("aggr label font", "aggr label color"),
        };
        let lay = self.text_box(text, &self.style.font(font, None));
        let (x, y) = shape.anchor(pos);
        let a = COMPASS.iter().find(|(n, _)| *n == pos).map_or(0.0, |(_, a)| *a);
        let color = parse_color(Some(self.style.get(color)), Some(super::BLACK));
        gfx.text(self.place_text(lay, x, y, octant_anchor(a + 180.0), 2.0, color, 0.0, None));
    }

    pub(super) fn place_beside(&self, ident: &str, text: &str, gfx: &mut Gfx, kind: BesideKind) -> Fallible {
        // words are wide: centred above or below a shape, they would straddle its edges
        let sides_only = kind == BesideKind::Constraint && self.style.get("constraint style") == "words";
        let dir = self.best_dir(ident, sides_only);
        self.beside_label(ident, text, dir, gfx, kind);
        Ok(())
    }

    /// Port of `\er@bestdir`: the free direction around a node.
    pub(super) fn best_dir(&self, ident: &str, sides_only: bool) -> &'static str {
        let mut best = "east";
        let mut best_score = -1.0;
        let candidates = [
            (0.0, "east"),
            (45.0, "north east"),
            (315.0, "south east"),
            (90.0, "north"),
            (270.0, "south"),
            (180.0, "west"),
            (135.0, "north west"),
            (225.0, "south west"),
        ];
        for (cand, name) in candidates {
            if sides_only && (cand == 90.0 || cand == 270.0) {
                continue;
            }
            let mut score = 180.0f64;
            for (_, a) in self.angles.get(ident).map_or(&[][..], Vec::as_slice) {
                score = score.min(py_mod(cand - a, 360.0).min(py_mod(a - cand, 360.0)));
            }
            if score > best_score {
                best = name;
                best_score = score;
            }
        }
        best
    }
}
