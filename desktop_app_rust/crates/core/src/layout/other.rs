//! Notes, subdomains and the grid.

use super::nodes::{font_option, or};
use super::prims::{Gfx, Handle, PathPrim, Stroke, TextPrim, WHITE};
use super::{Fallible, Layout};
use crate::colors::parse_color;
use crate::geometry::{CM, PathOp, Pt, Segment, Shape, ShapeKind};
use crate::latex_text::{Fmt, typeset};
use crate::model::{Grid, Note, Subdomain};

fn rect(cx: f64, cy: f64, hw: f64, hh: f64) -> Shape {
    Shape { cx, cy, outer: 0.0, kind: ShapeKind::Rect { hw, hh, radius: 0.0 } }
}

impl Layout<'_> {
    pub(super) fn layout_note(&mut self, e: &Note, gfx: &mut Gfx) -> Fallible {
        let st = &self.style;
        let o = &e.opts;
        let fmt = st.font("note font", None);
        let fmt = if o.get("font").is_some_and(|f| !f.is_empty()) { font_option(o, fmt)? } else { fmt };
        let color = parse_color(
            Some(or(or(o.get("color"), o.get("text")), Some(st.get("note color"))).unwrap_or("")),
            Some(super::BLACK),
        );
        let lay = self.text_layout(&typeset(&e.content, &fmt), false, None, None);
        let (hw, hh) = (lay.width / 2.0, (lay.height + lay.depth) / 2.0);
        let probe = rect(0.0, 0.0, hw, hh);
        let (cx, cy) = self.node_position(e.pos.as_ref(), o, &probe, gfx);
        gfx.text(TextPrim::at(lay, cx, cy, color));
        gfx.shape = Some(rect(cx, cy, hw, hh));
        gfx.handles.insert("center".into(), Handle::Point((cx, cy)));
        Ok(())
    }

    pub(super) fn layout_subdomain(&mut self, e: &Subdomain, gfx: &mut Gfx) {
        let st = &self.style;
        let o = &e.opts;
        let pts: Option<Vec<Pt>> = e.points.iter().map(|c| self.coord(Some(c))).collect();
        let Some(pts) = pts.filter(|p| p.len() >= 2) else {
            gfx.warnings.push("cannot evaluate the subdomain outline".into());
            return;
        };
        let color = parse_color(
            Some(or(or(o.get("color"), o.get("colour")), Some(st.get("subdomain color"))).unwrap_or("")),
            Some(super::BLACK),
        );
        let lw = st.length("subdomain line width");
        let outline: Vec<PathOp> = match pts.as_slice() {
            [(x1, y1), (x2, y2)] => vec![
                PathOp::Move((*x1, *y1)),
                PathOp::Line((*x2, *y1)),
                PathOp::Line((*x2, *y2)),
                PathOp::Line((*x1, *y2)),
                PathOp::Close,
            ],
            _ => {
                let mut ops: Vec<PathOp> = pts
                    .iter()
                    .enumerate()
                    .map(|(k, p)| if k == 0 { PathOp::Move(*p) } else { PathOp::Line(*p) })
                    .collect();
                ops.push(PathOp::Close);
                ops
            }
        };
        if let Some(fill) = o.get("fill").filter(|f| !f.is_empty()) {
            gfx.path(PathPrim {
                segs: outline.clone(),
                stroke: None,
                fill: parse_color(Some(fill), Some(super::BLACK)),
                pattern: None,
                arrows: vec![],
            });
        }
        if o.contains("dotted") && o.get("dotted") != Some("false") {
            gfx.path(PathPrim {
                segs: outline.clone(),
                stroke: None,
                fill: None,
                pattern: color,
                arrows: vec![],
            });
        }
        let dash = o.contains("dashed").then_some((3.0, 3.0));
        gfx.path(PathPrim::stroked(outline, Stroke { color, width: lw, dash, double: None }));
        // the label box with its tucked-in corner
        let lay = self.text_box(&e.label, &st.font("subdomain label font", None));
        let (hp, vp, tuck) = (
            st.length("subdomain label hpad"),
            st.length("subdomain label vpad"),
            st.length("subdomain tuck"),
        );
        let (x0, y0) = pts.first().copied().unwrap_or((0.0, 0.0));
        let w = lay.width + 2.0 * hp;
        let h = lay.height + lay.depth + 2.0 * vp;
        let label_box = vec![
            PathOp::Move((x0, y0)),
            PathOp::Line((x0 + w, y0)),
            PathOp::Line((x0 + w, y0 - h + tuck)),
            PathOp::Line((x0 + w - tuck, y0 - h)),
            PathOp::Line((x0, y0 - h)),
            PathOp::Close,
        ];
        gfx.path(PathPrim {
            segs: label_box,
            stroke: Some(Stroke::new(color, lw)),
            fill: Some(WHITE),
            pattern: None,
            arrows: vec![],
        });
        gfx.text(TextPrim::at(lay, x0 + w / 2.0, y0 - h / 2.0, color));
        for (k, p) in pts.iter().enumerate() {
            gfx.handles.insert(format!("p{k}"), Handle::Point(*p));
        }
        // for selecting: the label box and the outline (not the inside)
        gfx.handles.insert("labelbox".into(), Handle::Rect(x0, y0 - h, x0 + w, y0));
        let ring: Vec<Pt> = match pts.as_slice() {
            [a, b] => vec![*a, (b.0, a.1), *b, (a.0, b.1)],
            _ => pts.clone(),
        };
        let segments =
            ring.iter().zip(ring.iter().cycle().skip(1)).map(|(a, b)| Segment::line(*a, *b)).collect();
        gfx.handles.insert("outline".into(), Handle::Segments(segments));
    }

    pub(super) fn layout_grid(&mut self, e: &Grid, gfx: &mut Gfx) {
        let (Some(a), Some(b)) = (self.coord(Some(&e.a)), self.coord(Some(&e.b))) else { return };
        let cell = |v: f64| (v / CM + 1e-6).floor() as i64;
        let (xa, xb) = (cell(a.0).min(cell(b.0)), cell(a.0).max(cell(b.0)));
        let (ya, yb) = (cell(a.1).min(cell(b.1)), cell(a.1).max(cell(b.1)));
        let mut path = Vec::new();
        for x in xa..=xb {
            path.push(PathOp::Move((x as f64 * CM, ya as f64 * CM)));
            path.push(PathOp::Line((x as f64 * CM, yb as f64 * CM)));
        }
        for y in ya..=yb {
            path.push(PathOp::Move((xa as f64 * CM, y as f64 * CM)));
            path.push(PathOp::Line((xb as f64 * CM, y as f64 * CM)));
        }
        let st = &self.style;
        gfx.path(PathPrim::stroked(
            path,
            Stroke::new(parse_color(Some(st.get("grid color")), Some(super::BLACK)), 0.2),
        ));
        let fmt = Fmt { size: 5.0, ..Fmt::default() };
        let col = parse_color(Some(st.get("grid label color")), Some(super::BLACK));
        for x in xa..=xb {
            for y in ya..=yb {
                gfx.text(TextPrim::at(
                    self.text_box(&format!("{x},{y}"), &fmt),
                    x as f64 * CM,
                    y as f64 * CM,
                    col,
                ));
            }
        }
    }
}
