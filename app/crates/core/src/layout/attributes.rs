//! Attribute lists: compact (port of `\er@attributes`) and as ovals (port of `\er@placechen`,
//! which must make the same decisions).

use super::nodes::font_option;
use super::prims::{Gfx, Handle, Oval, PathPrim, Prim, Stroke};
use super::{Fallible, Layout, length};
use crate::colors::parse_color;
use crate::geometry::{CM, PathOp, Pt};
use crate::latex_text::typeset;
use crate::model::{AttrItem, AttributeTree, Coord, Node, Options, flatten_items};
use crate::text::{is_space, py_float, py_mod};

/// Python's truthiness of an option value: present and not empty.
fn nonempty(v: Option<&str>) -> Option<&str> {
    v.filter(|s| !s.is_empty())
}

impl Layout<'_> {
    /// `: integer` if data types are shown (`show types`).
    pub(super) fn type_suffix(&self, ty: Option<&str>) -> String {
        let Some(ty) = ty.filter(|t| !t.is_empty()) else { return String::new() };
        if !self.style.values.iter().any(|(k, _)| *k == "show types") || !self.style.flag("show types") {
            return String::new();
        }
        format!("{}{{{}{ty}}}", self.style.get("type sep"), self.style.get("type font"))
    }

    /// Attribute lists as ovals, once all edges are known.  The free directions around the
    /// node are given out clockwise from the top, in the order of the list; an oval that would
    /// overlap another moves further out.  The parts of a composite attribute fan out behind it.
    pub(super) fn layout_chen(&mut self, tree: &AttributeTree, index: usize, gfx: &mut Gfx) -> Fallible {
        let Some(owner) = self.shapes.get(&tree.node).cloned() else {
            gfx.warnings.push(format!("unknown node '{}'", tree.node));
            return Ok(());
        };
        let mut rows: Vec<(&AttrItem, Option<usize>)> = Vec::new();
        fn walk<'t>(
            items: &'t [AttrItem],
            parent: Option<usize>,
            rows: &mut Vec<(&'t AttrItem, Option<usize>)>,
        ) {
            for it in items {
                rows.push((it, parent));
                let me = rows.len() - 1;
                walk(&it.parts, Some(me), rows);
            }
        }
        walk(&tree.items, None, &mut rows);
        let font = self.style.font("attr font", None);
        let min_w = self.style.length("attribute minimum width");
        let min_h = self.style.length("attribute minimum height");
        let mut labels = Vec::new();
        let mut sizes: Vec<(f64, f64)> = Vec::new();
        for (it, _) in &rows {
            let suffix = self.type_suffix(it.r#type.as_deref());
            let lay = self.text_box(&format!("{}{suffix}", it.text), &font);
            let (tw, th) = (lay.width / 2.0 + 2.0, (lay.height + lay.depth) / 2.0 + 2.0);
            // 1.41421 as the package and the Python app write it, not exactly the square root of 2
            #[allow(clippy::approx_constant)]
            let k = if it.parts.is_empty() { 1.41421 } else { 1.0 };
            sizes.push(((k * tw).max(min_w / 2.0), (k * th).max(min_h / 2.0)));
            let text = if it.key { self.key_label(&it.text, &tree.node) } else { it.text.clone() };
            labels.push(format!("{text}{suffix}"));
        }
        let size = |k: usize| sizes.get(k).copied().unwrap_or((1.0, 1.0));
        let radius = |k: usize, ang: f64| {
            let (hw, hh) = size(k);
            let (c, s) = (ang.to_radians().cos(), ang.to_radians().sin());
            if rows.get(k).is_some_and(|(it, _)| !it.parts.is_empty()) {
                hw * hh / (hh * c.abs()).max(hw * s.abs())
            } else {
                hw * hh / (hh * c).hypot(hw * s)
            }
        };
        let mut used: Vec<f64> =
            self.angles.get(&tree.node).map_or(vec![], |v| v.iter().map(|(_, a)| *a).collect());
        let mut boxes: Vec<(f64, f64, f64, f64)> = Vec::new();
        let mut auto = 0;
        let mut parts_of: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        for (k, (it, parent)) in rows.iter().enumerate() {
            if let Some((px, py)) = it.pos {
                if parent.is_none() {
                    used.push(py_mod(py.atan2(px).to_degrees(), 360.0));
                    let (hw, hh) = size(k);
                    boxes.push((owner.cx + px * CM, owner.cy + py * CM, hw, hh));
                }
            } else if let Some(p) = parent {
                *parts_of.entry(*p).or_insert(0) += 1;
            } else {
                auto += 1;
            }
        }
        let step_angle = |step: i64| (90 - 15 * step).rem_euclid(360);
        let mut chosen = [0usize; 24];
        for _ in 0..auto {
            let (mut best, mut best_score) = (0i64, -1.0f64);
            for step in 0..24 {
                let cand = step_angle(step) as f64;
                let score = used
                    .iter()
                    .fold(180.0f64, |s, a| s.min(py_mod(cand - a, 360.0).min(py_mod(a - cand, 360.0))));
                if score > best_score + 0.01 {
                    best = step;
                    best_score = score;
                }
            }
            used.push(step_angle(best) as f64);
            if let Some(c) = chosen.get_mut(best as usize) {
                *c += 1;
            }
        }
        let mut given: std::collections::VecDeque<i64> = (0..24)
            .flat_map(|step| {
                std::iter::repeat_n(step_angle(step), chosen.get(step as usize).copied().unwrap_or(0))
            })
            .collect();
        let stroke = Stroke::new(
            parse_color(Some(self.style.get("edge color")), Some(super::BLACK)),
            self.style.length("edge line width"),
        );
        let dist = self.style.length("chen dist");
        let mut centres: Vec<Pt> = Vec::new();
        let mut angles: Vec<f64> = Vec::new();
        let mut shapes = Vec::new();
        let mut next_part: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        let mut path = Vec::new();
        let mut ovals = Vec::new();
        let key = gfx.key.map_or(index as u64, |k| k.0);
        for (k, (it, parent)) in rows.iter().enumerate() {
            let (ox, oy) = match parent {
                None => (owner.cx, owner.cy),
                Some(p) => centres.get(*p).copied().unwrap_or((owner.cx, owner.cy)),
            };
            let (hw, hh) = size(k);
            let (cx, cy, ang);
            if let Some((px, py)) = it.pos {
                cx = ox + px * CM;
                cy = oy + py * CM;
                ang = py_mod(py.atan2(px).to_degrees(), 360.0);
                if parent.is_some() {
                    boxes.push((cx, cy, hw, hh));
                }
            } else {
                let (sx, sy);
                match parent {
                    None => {
                        let a = given.pop_front().ok_or_else(|| "pop from empty list".to_string())?;
                        ang = a as f64;
                        (sx, sy) = owner.anchor(&a.to_string());
                    }
                    Some(p) => {
                        let j = next_part.get(p).copied().unwrap_or(0);
                        next_part.insert(*p, j + 1);
                        let count = parts_of.get(p).copied().unwrap_or(0) as f64;
                        ang =
                            angles.get(*p).copied().unwrap_or(0.0) + (j as f64 - (count - 1.0) / 2.0) * 40.0;
                        let r = radius(*p, ang);
                        sx = ox + r * ang.to_radians().cos();
                        sy = oy + r * ang.to_radians().sin();
                    }
                }
                let mut t = dist + radius(k, ang);
                let (mut x, mut y) = (sx, sy);
                for tries in 0..25 {
                    x = sx + t * ang.to_radians().cos();
                    y = sy + t * ang.to_radians().sin();
                    if !boxes.iter().any(|b| (x - b.0).abs() < hw + b.2 && (y - b.1).abs() < hh + b.3) {
                        break;
                    }
                    if tries < 24 {
                        t += 0.3 * CM;
                    }
                }
                cx = x;
                cy = y;
                boxes.push((cx, cy, hw, hh));
            }
            centres.push((cx, cy));
            angles.push(ang);
            let mut opts = Options::new();
            for (flag, name) in [
                (it.derived, "derived"),
                (it.multivalued, "multivalued"),
                (!it.parts.is_empty(), "composite"),
            ] {
                if flag {
                    opts.set(name, None);
                }
            }
            let mut sub = Gfx { key: gfx.key, ..Default::default() };
            let node = Node {
                command: "attribute".into(),
                id: format!("er@oval{key}@{k}"),
                pos: Some(Coord::numeric(cx / CM, cy / CM)),
                label: labels.get(k).cloned(),
                opts,
                generalisation: None,
            };
            self.layout_node(&node, &mut sub, false)?;
            gfx.prims.append(&mut sub.prims);
            let Some(shape) = sub.shape else { continue };
            let before = match parent {
                None => owner.clone(),
                Some(p) => shapes.get(*p).cloned().unwrap_or_else(|| owner.clone()),
            };
            let a = before.border_toward(cx, cy);
            let b = shape.border_toward(before.cx, before.cy);
            path.push(PathOp::Move(a));
            path.push(PathOp::Line(b));
            ovals.push(Oval { row: k, item: (*it).clone(), shape: shape.clone(), origin: (ox, oy) });
            shapes.push(shape);
        }
        gfx.prims.insert(0, Prim::Path(PathPrim::stroked(path, stroke)));
        gfx.handles.insert("ovals".into(), Handle::Ovals(ovals));
        Ok(())
    }

    pub(super) fn layout_attributes(
        &mut self,
        tree: &AttributeTree,
        index: usize,
        gfx: &mut Gfx,
    ) -> Fallible {
        let st = &self.style;
        if st.get("attribute style") == "chen" {
            self.deferred_chen.push(index);
            return Ok(());
        }
        let o = &tree.opts;
        let Some(shape) = self.shapes.get(&tree.node).cloned() else {
            gfx.warnings.push(format!("unknown node '{}'", tree.node));
            return Ok(());
        };
        let anchor = crate::text::strip(nonempty(o.get("anchor")).unwrap_or("east")).to_string();
        let mut direction = nonempty(o.get("dir")).map(str::to_string);
        let mut side = nonempty(o.get("side")).map(str::to_string);
        let align = nonempty(o.get("align")).unwrap_or("center").to_string();
        let spread = length(Some(nonempty(o.get("spread")).unwrap_or(st.get("attr spread"))), 0.0);
        let gap_text = nonempty(o.get("gap")).or(nonempty(Some(st.get("attr gap"))));
        let gap = gap_text.map_or(spread, |v| length(Some(v), 0.0));
        let tick = length(Some(nonempty(o.get("tick")).unwrap_or(st.get("attr tick"))), 0.0);
        let stem = length(Some(nonempty(o.get("stem")).unwrap_or(st.get("attr stem"))), 0.0);
        let has = |w: &str| anchor.contains(w);
        let any_compass = has("north") || has("south") || has("west") || has("east");
        // \s*[-+]?[\d.]+\s*
        let numeric = !any_compass && {
            let t = anchor.trim_matches(is_space);
            let body = t.strip_prefix(['+', '-']).unwrap_or(t);
            !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        };
        if direction.is_none() {
            direction = Some(
                if numeric {
                    let a = py_float(&anchor)?;
                    let (s, c) = (a.to_radians().sin(), a.to_radians().cos());
                    if s.abs() > c.abs() {
                        if s > 0.0 { "up" } else { "down" }
                    } else if c < 0.0 {
                        "left"
                    } else {
                        "right"
                    }
                } else if has("north") {
                    "up"
                } else if has("south") {
                    "down"
                } else if has("west") {
                    "left"
                } else {
                    "right"
                }
                .to_string(),
            );
        }
        if side.is_none() {
            side = Some(
                if numeric {
                    if py_float(&anchor)?.to_radians().cos() < -0.01 { "left" } else { "right" }
                } else if has("west") {
                    "left"
                } else {
                    "right"
                }
                .to_string(),
            );
        }
        let direction = direction.unwrap_or_default();
        let side = side.unwrap_or_default();
        let (mut x0, mut y0) = shape.anchor(&anchor);
        if let Some(v) = nonempty(o.get("xshift")) {
            x0 += length(Some(v), 0.0);
        }
        if let Some(v) = nonempty(o.get("yshift")) {
            y0 += length(Some(v), 0.0);
        }
        let dir_angle = match direction.as_str() {
            "up" => 90.0,
            "down" => 270.0,
            "left" => 180.0,
            _ => 0.0,
        };
        self.angles.entry(tree.node.clone()).or_default().push((0, dir_angle));
        let rows = flatten_items(&tree.items);
        let n = rows.len();
        gfx.handles.insert("start".into(), Handle::Point((x0, y0)));
        if n == 0 {
            return Ok(());
        }
        let st = &self.style;
        let stroke = Stroke::new(
            parse_color(Some(st.get("edge color")), Some(super::BLACK)),
            st.length("edge line width"),
        );
        let fmt = st.font("attr font", None);
        let fmt = if nonempty(o.get("font")).is_some() { font_option(o, fmt)? } else { fmt };
        let color =
            parse_color(Some(nonempty(o.get("text")).unwrap_or(st.get("attr text"))), Some(super::BLACK));
        let ex = 0.43 * fmt.size;
        let labels: Vec<_> = rows
            .iter()
            .map(|(it, _)| {
                let shown = if it.key { self.key_label(&it.shown(), &tree.node) } else { it.shown() };
                let src = format!("{shown}{}", self.type_suffix(it.r#type.as_deref()));
                self.text_layout(&typeset(&src, &fmt), true, Some(1.5 * ex), Some(0.25 * ex))
            })
            .collect();
        let last_main =
            rows.iter().enumerate().filter(|(_, (_, d))| *d == 0).map(|(k, _)| k + 1).max().unwrap_or(1);
        let mut path = Vec::new();
        let nf = n as f64;
        let (origin, row_side, lanchor);
        let row_y: Box<dyn Fn(usize) -> f64>;
        if direction == "up" || direction == "down" {
            let s = if direction == "up" { 1.0 } else { -1.0 };
            let (ss, la) = if side == "left" { (-1.0, "east") } else { (1.0, "west") };
            lanchor = la;
            let top = s * (gap + (if s == 1.0 { nf - 1.0 } else { last_main as f64 - 1.0 }) * spread);
            path.push(PathOp::Move((x0, y0)));
            path.push(PathOp::Line((x0, y0 + top)));
            origin = (x0, y0);
            row_side = ss;
            row_y = Box::new(move |k: usize| {
                let j = if s == 1.0 { nf - k as f64 } else { k as f64 - 1.0 };
                s * (gap + j * spread)
            });
        } else {
            let s = if direction == "right" { 1.0 } else { -1.0 };
            lanchor = if s == 1.0 { "west" } else { "east" };
            let bx = x0 + s * stem;
            path.push(PathOp::Move((x0, y0)));
            path.push(PathOp::Line((bx, y0)));
            let align = align.clone();
            let f = move |k: usize| -> f64 {
                let k = k as f64;
                match align.as_str() {
                    "top" => -(k - 1.0) * spread,
                    "bottom" => (nf - k) * spread,
                    _ => (nf - 1.0) * spread / 2.0 - (k - 1.0) * spread,
                }
            };
            // parts at the end may leave the stem below the last row that isn't one
            path.push(PathOp::Move((bx, y0 + f(1))));
            path.push(PathOp::Line((bx, y0 + f(last_main).min(0.0))));
            origin = (bx, y0);
            row_side = s;
            row_y = Box::new(f);
        }
        let indent = self.style.length("attr indent");
        let mut ends: Vec<f64> = Vec::new(); // where the tick of a row ends
        let mut stack: Vec<(usize, f64)> = Vec::new(); // the composite attributes around this row
        let mut parts_left: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        for (k0, ((it, depth), lay)) in rows.iter().zip(labels).enumerate() {
            let k = k0 + 1;
            stack.truncate(*depth);
            let base = if *depth > 0 {
                stack.last().and_then(|(o, _)| ends.get(o - 1)).copied().unwrap_or(0.0) + row_side * indent
            } else {
                0.0
            };
            let y = origin.1 + row_y(k);
            let end = base + row_side * tick;
            ends.push(end);
            path.push(PathOp::Move((origin.0 + base, y)));
            path.push(PathOp::Line((origin.0 + end, y)));
            gfx.text(self.place_text(lay, origin.0 + end, y, lanchor, 1.0, color, 0.0, None));
            if *depth > 0
                && let Some(&(owner, owner_y)) = stack.last()
            {
                let left = parts_left.entry(owner).or_insert(0);
                *left = left.saturating_sub(1);
                if *left == 0 {
                    path.push(PathOp::Move((origin.0 + base, owner_y - 0.45 * spread)));
                    path.push(PathOp::Line((origin.0 + base, y)));
                }
            }
            stack.push((k, y));
            parts_left.insert(k, it.parts.len());
        }
        gfx.prims.insert(0, Prim::Path(PathPrim::stroked(path, stroke)));
        Ok(())
    }
}
