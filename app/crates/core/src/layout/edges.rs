//! Edges: their paths and arrow heads, cardinalities and roles (ER), crow's foot symbols, and
//! the labels of GSM edges.

use super::nodes::or;
use super::position::float_opt;
use super::prims::{Arrow, Gfx, PathPrim, Stroke, TextPrim, WHITE};
use super::{DeferredCard, Fallible, LINE_WIDTHS, Layout, choose_side, length, octant_anchor, plain_length};
use crate::cardinality::crow_kind;
use crate::colors::parse_color;
use crate::geometry::{self as g, PathOp, Pt, Segment, Shape, shorten_end};
use crate::latex_text::font_switches;
use crate::model::{Edge, Options, endpoint_id, parse_coord_list};
use crate::text::{py_float, py_int, py_mod, round_half_even, strip};

/// How an edge is drawn.
pub(super) struct EdgeStyle {
    pub stroke: Stroke,
    /// the number of arrow heads at the end
    pub arrows: usize,
    pub arrow_length: f64,
    pub arrow_width: f64,
}

/// One end of an edge: (shape, or a fixed point; the reference point).
type End = (Option<Shape>, Option<Pt>, Pt);

impl Layout<'_> {
    fn edge_style(&self, command: &str, opts: &Options, arrow: bool) -> EdgeStyle {
        let st = &self.style;
        let color = parse_color(
            or(or(opts.get("draw"), opts.get("color")), Some(st.get("edge color"))),
            Some(super::BLACK),
        );
        let mut es = EdgeStyle {
            stroke: Stroke::new(color, st.length("edge line width")),
            arrows: 0,
            arrow_length: 6.0,
            arrow_width: 4.0,
        };
        if self.is_er() {
            if arrow || opts.contains("arrow") || opts.contains("->") {
                es.arrows = 1;
            }
        } else {
            es.arrow_length = st.length("arrow length");
            es.arrow_width = st.length("arrow width");
            if matches!(command, "attribute" | "component" | "generalization" | "specialization") {
                es.arrows = 1;
            } else if command == "multiattribute" {
                es.arrows = 2;
            } else if opts.contains("->") {
                es.arrows = 1;
            }
            match command {
                "component" => es.stroke.dash = Some((3.0, 3.0)),
                "generalization" => es.stroke.width = st.length("generalization line width"),
                "specialization" => es.stroke.double = Some(st.length("specialization double distance")),
                _ => {}
            }
        }
        for (k, v) in LINE_WIDTHS {
            if opts.contains(k) {
                es.stroke.width = *v;
            }
        }
        if opts.contains("line width") {
            es.stroke.width = plain_length(opts.get("line width"), es.stroke.width);
        }
        if opts.contains("dashed") {
            es.stroke.dash = Some((3.0, 3.0));
        } else if opts.contains("dotted") {
            es.stroke.dash = Some((0.4, 2.0));
        }
        if opts.contains("double") && opts.get("double") != Some("false") {
            es.stroke.double = Some(if self.is_er() && !opts.contains("double distance") {
                st.length("link double distance")
            } else {
                plain_length(opts.get("double distance"), 0.6)
            });
        }
        es
    }

    fn endpoint(&self, reference: &str) -> Fallible<End> {
        let (ident, anchor) = reference.split_once('.').unwrap_or((reference, ""));
        let ident = strip(ident);
        let Some(shape) = self.shapes.get(ident) else {
            let p = self.eval_point(reference).ok_or_else(|| format!("unknown node '{ident}'"))?;
            return Ok((None, Some(p), p));
        };
        if !anchor.is_empty() {
            let p = shape.anchor(anchor);
            return Ok((None, Some(p), p));
        }
        Ok((Some(shape.clone()), None, shape.center()))
    }

    /// The segments of an edge.  `a_point` / `b_point`: a fixed point instead of that end
    /// (see `crow_stub`).
    fn edge_segments(
        &self,
        src: &str,
        dst: &str,
        opts: &Options,
        a_point: Option<Pt>,
        b_point: Option<Pt>,
    ) -> Fallible<Vec<Segment>> {
        let (sa, pa, ra) = match a_point {
            Some(p) => (None, Some(p), p),
            None => self.endpoint(src)?,
        };
        let (sb, pb, rb) = match b_point {
            Some(p) => (None, Some(p), p),
            None => self.endpoint(dst)?,
        };
        let fixed = |p: Option<Pt>| p.unwrap_or((0.0, 0.0));
        if let Some(via) = opts.get("via").filter(|v| !v.is_empty()) {
            let mut pts = Vec::new();
            for c in parse_coord_list(via) {
                let p = self
                    .coord(Some(&c))
                    .ok_or_else(|| format!("cannot evaluate way point {}", c.to_tex()))?;
                pts.push(p);
            }
            let out_of_range = || "list index out of range".to_string();
            let start = match &sa {
                Some(s) => {
                    let p = pts.first().ok_or_else(out_of_range)?;
                    s.border_toward(p.0, p.1)
                }
                None => fixed(pa),
            };
            let end = match &sb {
                Some(s) => {
                    let p = pts.last().ok_or_else(out_of_range)?;
                    s.border_toward(p.0, p.1)
                }
                None => fixed(pb),
            };
            let mut chain = vec![start];
            chain.extend(pts);
            chain.push(end);
            return Ok(chain
                .windows(2)
                .filter_map(|w| match w {
                    [a, b] => Some(Segment::line(*a, *b)),
                    _ => None,
                })
                .collect());
        }
        let ortho = opts.get("ortho");
        if matches!(ortho, Some("-|" | "|-")) {
            let corner = if ortho == Some("-|") { (rb.0, ra.1) } else { (ra.0, rb.1) };
            let start = sa.as_ref().map_or(fixed(pa), |s| s.border_toward(corner.0, corner.1));
            let end = sb.as_ref().map_or(fixed(pb), |s| s.border_toward(corner.0, corner.1));
            return Ok(vec![Segment::line(start, corner), Segment::line(corner, end)]);
        }
        // bends
        let mut bend: Option<f64> = None;
        if opts.contains("bend") {
            bend = float_opt(opts.get("bend"), 0.0).ok();
        }
        if opts.contains("bend left") {
            bend = Some(float_opt(opts.get("bend left"), 30.0)?);
        }
        if opts.contains("bend right") {
            bend = Some(-float_opt(opts.get("bend right"), 30.0)?);
        }
        let (out_a, in_a) = (opts.get("out"), opts.get("in"));
        if bend.is_some_and(|b| b != 0.0) || out_a.is_some() || in_a.is_some() {
            let b = bend.unwrap_or(0.0);
            let chord = g::angle_of(rb.0 - ra.0, rb.1 - ra.1);
            let out = match out_a {
                Some(v) => py_float(v)?,
                None => chord + b,
            };
            let inn = match in_a {
                Some(v) => py_float(v)?,
                None => chord + 180.0 - b,
            };
            let start = sa.as_ref().map_or(fixed(pa), |s| s.border_angle(out));
            let end = sb.as_ref().map_or(fixed(pb), |s| s.border_angle(inn));
            let loose = float_opt(opts.get("looseness"), 1.0)?;
            let d = (end.0 - start.0).hypot(end.1 - start.1) * 0.3915 * loose;
            let (ux, uy) = g::unit(out);
            let (vx, vy) = g::unit(inn);
            return Ok(vec![Segment::curve(
                start,
                end,
                (start.0 + d * ux, start.1 + d * uy),
                (end.0 + d * vx, end.1 + d * vy),
            )]);
        }
        let start = sa.as_ref().map_or(fixed(pa), |s| s.border_toward(rb.0, rb.1));
        let end = sb.as_ref().map_or(fixed(pb), |s| s.border_toward(ra.0, ra.1));
        Ok(vec![Segment::line(start, end)])
    }

    pub(super) fn layout_edge(&mut self, e: &Edge, gfx: &mut Gfx) -> Fallible {
        let mut cardend = None;
        if self.is_er() {
            let ta = self.types.get(e.src_id()).map(String::as_str);
            let tb = self.types.get(e.dst_id()).map(String::as_str);
            cardend = Some(if ta == Some("entity") {
                'a'
            } else if tb == Some("entity") {
                'b'
            } else {
                'a'
            });
        }
        self.draw_edge(
            gfx,
            &e.src,
            &e.dst,
            &e.opts,
            e.label.as_deref().unwrap_or(""),
            cardend,
            false,
            &e.command,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_edge(
        &mut self,
        gfx: &mut Gfx,
        src: &str,
        dst: &str,
        opts: &Options,
        label: &str,
        cardend: Option<char>,
        arrow_style: bool,
        command: &str,
    ) -> Fallible {
        let mut segs = self.edge_segments(src, dst, opts, None, None)?;
        let es = self.edge_style(command, opts, arrow_style);
        let mut stub = None;
        if self.is_er() && self.crow_kind_at(label, cardend, src, dst).is_some() {
            let (s, st) = self.crow_stub(src, dst, opts, segs, cardend)?;
            segs = s;
            stub = st;
        }
        let full: Vec<Segment> = match stub {
            Some(st) if cardend == Some('b') => segs.iter().copied().chain([st]).collect(),
            Some(st) => [st].into_iter().chain(segs.iter().copied()).collect(),
            None => segs.clone(),
        };
        let ortho =
            opts.get("ortho").is_some_and(|o| !o.is_empty()) && opts.get("via").is_none_or(|v| v.is_empty());
        // the directions at both ends
        self.edge_no += 1;
        let no = self.edge_no;
        let (Some(first), Some(last)) = (full.first(), full.last()) else { return Ok(()) };
        let (a0, a1) = (first.point(0.0), first.point(0.1));
        let (b0, b1) = (last.point(1.0), last.point(0.9));
        self.angles
            .entry(endpoint_id(src).to_string())
            .or_default()
            .push((no, g::angle_of(a1.0 - a0.0, a1.1 - a0.1)));
        self.angles
            .entry(endpoint_id(dst).to_string())
            .or_default()
            .push((no, g::angle_of(b1.0 - b0.0, b1.1 - b0.1)));
        gfx.segments = segs.clone();
        gfx.stub = stub.into_iter().collect();
        gfx.handles.insert("start".into(), super::Handle::Point(a0));
        gfx.handles.insert("end".into(), super::Handle::Point(b0));
        // arrow heads
        let mut draw = full.clone();
        let mut arrows = Vec::new();
        if es.arrows > 0
            && let Some(tail) = draw.last().copied()
        {
            let (tx, ty) = tail.tangent(1.0);
            let ang = g::angle_of(tx, ty);
            let tip = tail.p3;
            for k in 0..es.arrows {
                let (ux, uy) = g::unit(ang);
                let back = k as f64 * es.arrow_length;
                arrows.push(Arrow {
                    tip: (tip.0 - back * ux, tip.1 - back * uy),
                    angle: ang,
                    length: es.arrow_length,
                    width: es.arrow_width,
                    color: es.stroke.color,
                });
            }
            let cut = es.arrow_length * es.arrows as f64 - if es.stroke.double.is_none() { 0.5 } else { 0.0 };
            if let Some(slot) = draw.last_mut() {
                *slot = shorten_end(&tail, cut);
            }
        }
        let mut path = draw.first().map(|s| vec![PathOp::Move(s.p0)]).unwrap_or_default();
        path.extend(draw.iter().map(Segment::ops));
        gfx.path(PathPrim { segs: path, stroke: Some(es.stroke), fill: None, pattern: None, arrows });
        // labels
        if self.is_er() {
            self.er_edge_labels(gfx, &segs, opts, label, cardend, src, dst, no, ortho, &full)
        } else if !label.is_empty() {
            self.gsm_edge_label(gfx, &segs, opts, label, ortho)
        } else {
            Ok(())
        }
    }

    /// The crow's foot symbol drawn instead of the label, if any.
    fn crow_kind_at(&self, card: &str, cardend: Option<char>, src: &str, dst: &str) -> Option<&'static str> {
        let ent = endpoint_id(if cardend == Some('b') { dst } else { src });
        if card.is_empty()
            || self.style.get("card symbols") != "crowsfoot"
            || self.types.get(ent).map(String::as_str) != Some("entity")
        {
            return None;
        }
        crow_kind(Some(card))
    }

    /// Port of `\er@crowstub`: the edge meets the entity's side at a right angle.  It ends with
    /// a straight piece along the side's normal, which carries the symbol; the rest of the
    /// edge leads to its outer end.
    fn crow_stub(
        &self,
        src: &str,
        dst: &str,
        opts: &Options,
        segs: Vec<Segment>,
        cardend: Option<char>,
    ) -> Fallible<(Vec<Segment>, Option<Segment>)> {
        let b_end = cardend == Some('b');
        let Some(shape) = self.shapes.get(endpoint_id(if b_end { dst } else { src })) else {
            return Ok((segs, None));
        };
        let hit = if b_end { segs.last().map(|s| s.point(1.0)) } else { segs.first().map(|s| s.point(0.0)) };
        let Some(hit) = hit else { return Ok((segs, None)) };
        let (cx, cy) = shape.center();
        let (hw, hh) = (shape.anchor("east").0 - cx, shape.anchor("north").1 - cy);
        let (dx, dy) = (hit.0 - cx, hit.1 - cy);
        if hw <= 0.0 || hh <= 0.0 {
            return Ok((segs, None));
        }
        let (nx, ny) = if dx.abs() / hw >= dy.abs() / hh {
            (if dx >= 0.0 { 1.0 } else { -1.0 }, 0.0)
        } else {
            (0.0, if dy >= 0.0 { 1.0 } else { -1.0 })
        };
        let len = 2.5 * self.style.length("crow length");
        let q = (hit.0 + nx * len, hit.1 + ny * len);
        Ok(if b_end {
            (self.edge_segments(src, dst, opts, None, Some(q))?, Some(Segment::line(q, hit)))
        } else {
            (self.edge_segments(src, dst, opts, Some(q), None)?, Some(Segment::line(hit, q)))
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn er_edge_labels(
        &mut self,
        gfx: &mut Gfx,
        segs: &[Segment],
        opts: &Options,
        card: &str,
        cardend: Option<char>,
        src: &str,
        dst: &str,
        no: u32,
        ortho: bool,
        full: &[Segment],
    ) -> Fallible {
        let st = &self.style;
        let mut placement = st.get("card placement").to_string();
        if opts.contains("inline") {
            placement = "inline".into();
        }
        if opts.contains("beside") {
            placement = "beside".into();
        }
        if let Some(p) = opts.get("card placement").filter(|p| !p.is_empty()) {
            placement = p.to_string();
        }
        let b_end = cardend == Some('b');
        let lseg = if ortho {
            0
        } else if b_end {
            segs.len().saturating_sub(1)
        } else {
            0
        };
        let Some(seg) = segs.get(lseg).copied() else { return Ok(()) };
        // outward direction (entity -> neighbour) and length
        let (vx, vy) = if b_end {
            (seg.p0.0 - seg.p3.0, seg.p0.1 - seg.p3.1)
        } else {
            (seg.p3.0 - seg.p0.0, seg.p3.1 - seg.p0.1)
        };
        let total = if ortho { crate::text::py_sum(segs.iter().map(Segment::length)) } else { vx.hypot(vy) };
        let p = if placement == "inline" {
            0.5
        } else if let Some(cp) = opts.get("card pos").filter(|c| !c.is_empty()) {
            let p = py_float(cp)?;
            if b_end { 1.0 - p } else { p }
        } else {
            let dist = length(Some(or(opts.get("card dist"), Some(st.get("card dist"))).unwrap_or("")), 0.0);
            let p = if total < 1.0 { 0.5 } else { (dist / total).min(0.45) };
            if b_end { 1.0 - p } else { p }
        };
        let side_req = or(opts.get("card side"), Some(st.get("card side"))).unwrap_or("auto").to_string();
        let pref = choose_side(&side_req, vx, vy);
        let at = |t: f64| -> (Pt, Pt) {
            if ortho {
                // a -| path: two halves
                let (s, u) = if t <= 0.5 { (segs.first(), t * 2.0) } else { (segs.get(1), t * 2.0 - 1.0) };
                return s.map_or(((0.0, 0.0), (1.0, 0.0)), |s| (s.point(u), s.tangent(u)));
            }
            (seg.point(t), seg.tangent(t))
        };
        let mut card = card.to_string();
        if let Some(kind) = self.crow_kind_at(&card, cardend, src, dst) {
            self.draw_crowsfoot(gfx, if full.is_empty() { segs } else { full }, cardend, kind);
            card.clear();
        }
        let st = &self.style;
        if !card.is_empty() {
            if placement == "inline" {
                let ((x, y), _) = at(0.5);
                let lay = self.text_box(&card, &st.font("card font", None));
                let color = parse_color(Some(st.get("card text")), Some(super::BLACK));
                gfx.text(TextPrim {
                    layout: lay,
                    cx: x,
                    cy: y,
                    rotation: 0.0,
                    color,
                    bg: Some(WHITE),
                    inner: 1.0,
                });
            } else {
                let pt = if b_end { (p - 0.02).max(0.0) } else { (p + 0.02).min(1.0) };
                let (cp, _) = at(p);
                let (ct, _) = at(pt);
                let entity = endpoint_id(if b_end { dst } else { src }).to_string();
                self.deferred_cards.push(DeferredCard {
                    gfx: self.gfx.len(),
                    text: card.clone(),
                    cp,
                    ct,
                    entity,
                    edge_no: no,
                    pref,
                    forced: side_req != "auto",
                });
            }
        }
        let st = &self.style;
        if let Some(role) = opts.get("role").filter(|r| !r.is_empty()) {
            let rpos_text = or(opts.get("role pos"), Some(st.get("role pos"))).unwrap_or("auto").to_string();
            let rpos = if rpos_text == "auto" {
                if placement == "inline" && !card.is_empty() { if b_end { 0.75 } else { 0.25 } } else { 0.5 }
            } else {
                py_float(&rpos_text)?
            };
            let mut left =
                choose_side(opts.get("role side").filter(|s| !s.is_empty()).unwrap_or("auto"), vx, vy);
            if b_end {
                left = !left;
            }
            let ((x, y), (tx, ty)) = at(rpos);
            let ang = g::angle_of(tx, ty) + if left { 90.0 } else { -90.0 };
            let lay = self.text_box(role, &st.font("role font", None));
            let color = parse_color(Some(st.get("role text")), Some(super::BLACK));
            gfx.text(self.place_text(lay, x, y, octant_anchor(ang + 180.0), 2.0, color, 0.0, None));
        }
        Ok(())
    }

    /// The symbols of `\er@drawcrow`, outwards from the entity's border: how many at most (a
    /// bar or the crow's foot), then how few at least (a bar or a circle).
    fn draw_crowsfoot(&self, gfx: &mut Gfx, segs: &[Segment], cardend: Option<char>, kind: &str) {
        let st = &self.style;
        let ends = if cardend == Some('b') {
            segs.last().map(|s| (s.point(1.0), s.point(0.9)))
        } else {
            segs.first().map(|s| (s.point(0.0), s.point(0.1)))
        };
        let Some((p0, p1)) = ends else { return };
        let (ux, uy) = g::unit(g::angle_of(p1.0 - p0.0, p1.1 - p0.1));
        let len = st.length("crow length");
        let half = st.length("crow width");
        let radius = len / 3.0;
        let stroke = Stroke::new(
            parse_color(Some(st.get("edge color")), Some(super::BLACK)),
            st.length("edge line width"),
        );
        let at = |x: f64, y: f64| (p0.0 + x * ux - y * uy, p0.1 + x * uy + y * ux);
        let mut line = |gfx: &mut Gfx, a: Pt, b: Pt| {
            gfx.path(PathPrim::stroked(vec![PathOp::Move(a), PathOp::Line(b)], stroke.clone()))
        };
        let bar = |gfx: &mut Gfx, line: &mut dyn FnMut(&mut Gfx, Pt, Pt), x: f64| {
            line(gfx, at(x, half), at(x, -half))
        };
        let circle = |gfx: &mut Gfx, x: f64| {
            let (cx, cy) = at(x + radius, 0.0);
            let k = 0.5523 * radius;
            gfx.path(PathPrim {
                segs: vec![
                    PathOp::Move((cx + radius, cy)),
                    PathOp::Curve((cx + radius, cy + k), (cx + k, cy + radius), (cx, cy + radius)),
                    PathOp::Curve((cx - k, cy + radius), (cx - radius, cy + k), (cx - radius, cy)),
                    PathOp::Curve((cx - radius, cy - k), (cx - k, cy - radius), (cx, cy - radius)),
                    PathOp::Curve((cx + k, cy - radius), (cx + radius, cy - k), (cx + radius, cy)),
                    PathOp::Close,
                ],
                stroke: Some(stroke.clone()),
                fill: Some(WHITE),
                pattern: None,
                arrows: Vec::new(),
            });
        };
        if matches!(kind, "n" | "cn") {
            line(gfx, at(len, 0.0), at(0.0, half));
            line(gfx, at(len, 0.0), at(0.0, -half));
        } else {
            bar(gfx, &mut line, len / 2.0);
        }
        match kind {
            "1" => bar(gfx, &mut line, len),
            "c" => circle(gfx, len / 2.0 + len / 4.0),
            "n" => bar(gfx, &mut line, len + len / 3.0),
            _ => circle(gfx, len + len / 6.0),
        }
    }

    /// Port of `\er@placecard` (cardinalities beside the edge).
    pub(super) fn place_card(&mut self, gfx: &mut Gfx, card: &DeferredCard) {
        let st = &self.style;
        let (cp, ct) = (card.cp, card.ct);
        let th = g::angle_of(ct.0 - cp.0, ct.1 - cp.1);
        let (mut gl, mut gr) = (360.0f64, 360.0f64);
        for (k, a) in self.angles.get(&card.entity).map_or(&[][..], Vec::as_slice) {
            if *k == card.edge_no {
                continue;
            }
            gl = gl.min(py_mod(a - th, 360.0));
            gr = gr.min(py_mod(th - a, 360.0));
        }
        let left = if card.forced || gl.min(gr) >= 90.0 {
            card.pref
        } else if gl > gr {
            true
        } else if gl < gr {
            false
        } else {
            card.pref
        };
        let nth = if left { th + 90.0 } else { th - 90.0 };
        let sgn = |v: f64| {
            let r = round_half_even(100.0 * v);
            (r > 0.0) as i32 - (r < 0.0) as i32
        };
        let sx = -sgn(nth.to_radians().cos() + 0.15 * th.to_radians().cos());
        let sy = -sgn(nth.to_radians().sin() + 0.15 * th.to_radians().sin());
        let anchor = match (sx, sy) {
            (-1, -1) => "south west",
            (-1, 0) => "west",
            (-1, 1) => "north west",
            (0, -1) => "south",
            (0, 0) => "center",
            (0, 1) => "north",
            (1, -1) => "south east",
            (1, 0) => "east",
            _ => "north east",
        };
        let lay = self.text_box(&card.text, &st.font("card font", None));
        let w = lay.width / 2.0 + 1.5;
        let h = (lay.height + lay.depth) / 2.0 + 1.5;
        let entity = self.shapes.get(&card.entity).map(|s| {
            let (ex, ey) = s.anchor("south west");
            let (fx, fy) = s.anchor("north east");
            (ex, ey, fx, fy)
        });
        let (cx, cy) = (-f64::from(sx), -f64::from(sy));
        let mm = 2.845274;
        let (mut qx, mut qy) = cp;
        let mut boxed = (0.0, 0.0, 0.0, 0.0);
        for k in 0..13 {
            let step = f64::from(k) * mm;
            qx = cp.0 + step * th.to_radians().cos();
            qy = cp.1 + step * th.to_radians().sin();
            boxed = (qx + cx * w - w, qy + cy * h - h, qx + cx * w + w, qy + cy * h + h);
            let hit = |b: &(f64, f64, f64, f64)| {
                boxed.0 < b.2 + 1.0 && boxed.2 > b.0 - 1.0 && boxed.1 < b.3 + 1.0 && boxed.3 > b.1 - 1.0
            };
            let clear = !entity.as_ref().is_some_and(hit) && !self.card_boxes.iter().any(hit);
            if clear {
                break;
            }
        }
        self.card_boxes.push(boxed);
        let color = parse_color(Some(st.get("card text")), Some(super::BLACK));
        gfx.text(self.place_text(lay, qx, qy, anchor, 1.5, color, 0.0, None));
    }

    fn gsm_edge_label(
        &self,
        gfx: &mut Gfx,
        segs: &[Segment],
        opts: &Options,
        label: &str,
        ortho: bool,
    ) -> Fallible {
        let st = &self.style;
        let n = segs.len() as i64;
        let mut lseg = match opts.get("label segment").filter(|s| !s.is_empty()) {
            Some(s) => py_int(s)? - 1,
            // the middle segment, rounded like TeX's \numexpr (n+1)/2
            None => ((n + 1) as f64 / 2.0 + 0.5) as i64 - 1,
        };
        if ortho {
            lseg = 0;
        }
        let lseg = lseg.min(n - 1).max(0) as usize;
        let Some(seg) = segs.get(lseg).copied() else { return Ok(()) };
        let pos = py_float(or(opts.get("label pos"), Some(st.get("label pos"))).unwrap_or("0.5"))?;
        let ((x, y), (tx, ty)) = if ortho {
            let (s, u) = if pos <= 0.5 { (segs.first(), pos * 2.0) } else { (segs.get(1), pos * 2.0 - 1.0) };
            s.map_or(((0.0, 0.0), (1.0, 0.0)), |s| (s.point(u), s.tangent(u)))
        } else {
            (seg.point(pos), seg.tangent(pos))
        };
        let (vx, vy) = (seg.p3.0 - seg.p0.0, seg.p3.1 - seg.p0.1);
        let left =
            choose_side(or(opts.get("label side"), Some(st.get("label side"))).unwrap_or("auto"), vx, vy);
        let mut sloped = st.flag("label sloped");
        if opts.contains("sloped") {
            sloped = opts.get("sloped") != Some("false");
        }
        if opts.contains("horizontal") {
            sloped = false;
        }
        let mut fmt = st.font("label font", None);
        let mut color = parse_color(
            Some(or(opts.get("label color"), Some(st.get("label color"))).unwrap_or("black")),
            Some(super::BLACK),
        );
        if let Some(style) = opts.get("label style").filter(|s| !s.is_empty()) {
            for part in style.split(',') {
                let mut kv = part.splitn(2, '=');
                let key = strip(kv.next().unwrap_or(""));
                if let Some(value) = kv.next() {
                    if key == "font" {
                        fmt = font_switches(value, &fmt);
                    } else if key == "text" {
                        color = parse_color(Some(value), Some(super::BLACK));
                    }
                }
            }
        }
        let lay = self.text_box(label, &fmt);
        if sloped {
            let ang = g::angle_of(tx, ty);
            let flip = vx < -0.01;
            let rot = if flip { ang + 180.0 } else { ang };
            let anchor = if left == flip { "north" } else { "south" };
            gfx.text(self.place_text(lay, x, y, anchor, 2.0, color, rot, None));
        } else {
            let ang = g::angle_of(tx, ty) + if left { 90.0 } else { -90.0 };
            gfx.text(self.place_text(lay, x, y, octant_anchor(ang + 180.0), 2.0, color, 0.0, None));
        }
        Ok(())
    }
}
