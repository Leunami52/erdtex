//! Node shapes as TikZ computes them: sizes, border points and anchors.  All lengths are TeX
//! points; y points up (TikZ's convention).

use crate::text::parse_float;

/// TeX points per cm.
pub const CM: f64 = 28.45274;

pub type Pt = (f64, f64);

/// The compass anchors and their angles.
pub const COMPASS: &[(&str, f64)] = &[
    ("east", 0.0),
    ("north east", 45.0),
    ("north", 90.0),
    ("north west", 135.0),
    ("west", 180.0),
    ("south west", 225.0),
    ("south", 270.0),
    ("south east", 315.0),
];

fn compass_angle(name: &str) -> Option<f64> {
    COMPASS.iter().find(|(n, _)| *n == name).map(|(_, a)| *a)
}

pub fn unit(angle_deg: f64) -> Pt {
    let a = angle_deg.to_radians();
    (a.cos(), a.sin())
}

pub fn angle_of(dx: f64, dy: f64) -> f64 {
    dy.atan2(dx).to_degrees()
}

/// A step of an outline or edge path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathOp {
    Move(Pt),
    Line(Pt),
    Curve(Pt, Pt, Pt),
    Close,
}

/// The first point where the ray from (cx, cy) in direction (dx, dy) meets the closed polygon
/// `pts`; the centre if it meets none.
fn ray_polygon(cx: f64, cy: f64, dx: f64, dy: f64, pts: &[Pt]) -> Pt {
    let mut best: Option<f64> = None;
    let n = pts.len();
    for (k, &(x1, y1)) in pts.iter().enumerate() {
        let Some(&(x2, y2)) = pts.get((k + 1) % n) else { continue };
        let (ex, ey) = (x2 - x1, y2 - y1);
        let den = dx * ey - dy * ex;
        if den.abs() < 1e-12 {
            continue;
        }
        let t = ((x1 - cx) * ey - (y1 - cy) * ex) / den;
        let u = ((x1 - cx) * dy - (y1 - cy) * dx) / den;
        if t >= -1e-9 && (-1e-9..=1.0 + 1e-9).contains(&u) && best.is_none_or(|b| t < b) {
            best = Some(t);
        }
    }
    best.map_or((cx, cy), |t| (cx + t * dx, cy + t * dy))
}

/// A node's shape.  `outer` is TikZ's outer sep (half the line width).
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub cx: f64,
    pub cy: f64,
    pub outer: f64,
    pub kind: ShapeKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShapeKind {
    Point,
    Rect {
        hw: f64,
        hh: f64,
        radius: f64,
    },
    Diamond {
        hx: f64,
        hy: f64,
    },
    Ellipse {
        a: f64,
        b: f64,
    },
    /// isosceles, apex up
    Triangle {
        apex: Pt,
        left: Pt,
        right: Pt,
    },
    /// the package's own `er hexagon`: flat top and bottom, pointed sides
    Hexagon {
        hx: f64,
        hy: f64,
        slant: f64,
    },
}

impl Shape {
    pub fn center(&self) -> Pt {
        (self.cx, self.cy)
    }

    fn rect_pts(&self, hw: f64, hh: f64, o: f64) -> Vec<Pt> {
        let (w, h) = (hw + o, hh + o);
        vec![
            (self.cx - w, self.cy - h),
            (self.cx + w, self.cy - h),
            (self.cx + w, self.cy + h),
            (self.cx - w, self.cy + h),
        ]
    }

    fn diamond_pts(&self, hx: f64, hy: f64, o: f64) -> Vec<Pt> {
        let (x, y) = (hx + o, hy + o); // PGF adds the outer sep to the vertices
        vec![(self.cx + x, self.cy), (self.cx, self.cy + y), (self.cx - x, self.cy), (self.cx, self.cy - y)]
    }

    fn hexagon_pts(&self, hx: f64, hy: f64, s: f64, o: f64) -> Vec<Pt> {
        let (x, y) = (hx + o, hy + o);
        let (cx, cy) = (self.cx, self.cy);
        vec![
            (cx + x, cy),
            (cx + x - s, cy + y),
            (cx - x + s, cy + y),
            (cx - x, cy),
            (cx - x + s, cy - y),
            (cx + x - s, cy - y),
        ]
    }

    /// The path that draws the shape.
    pub fn outline(&self) -> Vec<PathOp> {
        let poly = |p: Vec<Pt>| {
            let mut ops: Vec<PathOp> = Vec::new();
            for (k, q) in p.into_iter().enumerate() {
                ops.push(if k == 0 { PathOp::Move(q) } else { PathOp::Line(q) });
            }
            ops.push(PathOp::Close);
            ops
        };
        match &self.kind {
            ShapeKind::Point => vec![],
            ShapeKind::Rect { hw, hh, radius } => {
                let (x0, x1, y0, y1) = (self.cx - hw, self.cx + hw, self.cy - hh, self.cy + hh);
                let r = radius.min(*hw).min(*hh);
                if r <= 0.0 {
                    return poly(vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
                }
                let k = 0.5523 * r;
                vec![
                    PathOp::Move((x0 + r, y0)),
                    PathOp::Line((x1 - r, y0)),
                    PathOp::Curve((x1 - r + k, y0), (x1, y0 + r - k), (x1, y0 + r)),
                    PathOp::Line((x1, y1 - r)),
                    PathOp::Curve((x1, y1 - r + k), (x1 - r + k, y1), (x1 - r, y1)),
                    PathOp::Line((x0 + r, y1)),
                    PathOp::Curve((x0 + r - k, y1), (x0, y1 - r + k), (x0, y1 - r)),
                    PathOp::Line((x0, y0 + r)),
                    PathOp::Curve((x0, y0 + r - k), (x0 + r - k, y0), (x0 + r, y0)),
                    PathOp::Close,
                ]
            }
            ShapeKind::Diamond { hx, hy } => poly(self.diamond_pts(*hx, *hy, 0.0)),
            ShapeKind::Ellipse { a, b } => {
                let k = 0.5523;
                let (cx, cy) = (self.cx, self.cy);
                vec![
                    PathOp::Move((cx + a, cy)),
                    PathOp::Curve((cx + a, cy + k * b), (cx + k * a, cy + b), (cx, cy + b)),
                    PathOp::Curve((cx - k * a, cy + b), (cx - a, cy + k * b), (cx - a, cy)),
                    PathOp::Curve((cx - a, cy - k * b), (cx - k * a, cy - b), (cx, cy - b)),
                    PathOp::Curve((cx + k * a, cy - b), (cx + a, cy - k * b), (cx + a, cy)),
                    PathOp::Close,
                ]
            }
            ShapeKind::Triangle { apex, left, right } => poly(vec![*left, *right, *apex]),
            ShapeKind::Hexagon { hx, hy, slant } => poly(self.hexagon_pts(*hx, *hy, *slant, 0.0)),
        }
    }

    /// Where the ray from the centre in direction (dx, dy) leaves the border.
    pub fn border_dir(&self, dx: f64, dy: f64) -> Pt {
        let o = self.outer;
        match &self.kind {
            ShapeKind::Point => (self.cx, self.cy),
            ShapeKind::Rect { hw, hh, .. } => {
                ray_polygon(self.cx, self.cy, dx, dy, &self.rect_pts(*hw, *hh, o))
            }
            ShapeKind::Diamond { hx, hy } => {
                ray_polygon(self.cx, self.cy, dx, dy, &self.diamond_pts(*hx, *hy, o))
            }
            ShapeKind::Ellipse { a, b } => {
                let (a, b) = (a + o, b + o);
                let n = dx.hypot(dy);
                let (dx, dy) = (dx / n, dy / n);
                let t = if a != 0.0 && b != 0.0 {
                    1.0 / ((dx / a).powi(2) + (dy / b).powi(2)).sqrt()
                } else {
                    0.0
                };
                (self.cx + t * dx, self.cy + t * dy)
            }
            ShapeKind::Triangle { apex, left, right } => {
                ray_polygon(self.cx, self.cy, dx, dy, &[*left, *right, *apex])
            }
            ShapeKind::Hexagon { hx, hy, slant } => {
                ray_polygon(self.cx, self.cy, dx, dy, &self.hexagon_pts(*hx, *hy, *slant, o))
            }
        }
    }

    pub fn bbox(&self) -> (f64, f64, f64, f64) {
        let (cx, cy) = (self.cx, self.cy);
        match &self.kind {
            ShapeKind::Point => (cx, cy, cx, cy),
            ShapeKind::Rect { hw, hh, .. } => (cx - hw, cy - hh, cx + hw, cy + hh),
            ShapeKind::Diamond { hx, hy } | ShapeKind::Hexagon { hx, hy, .. } => {
                (cx - hx, cy - hy, cx + hx, cy + hy)
            }
            ShapeKind::Ellipse { a, b } => (cx - a, cy - b, cx + a, cy + b),
            ShapeKind::Triangle { apex, left, right } => {
                let xs = [left.0, right.0, apex.0];
                let ys = [left.1, right.1, apex.1];
                let min = |v: [f64; 3]| v.iter().copied().fold(f64::INFINITY, f64::min);
                let max = |v: [f64; 3]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                (min(xs), min(ys), max(xs), max(ys))
            }
        }
    }

    pub fn border_toward(&self, px: f64, py: f64) -> Pt {
        let (dx, dy) = (px - self.cx, py - self.cy);
        if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
            return (self.cx, self.cy);
        }
        self.border_dir(dx, dy)
    }

    pub fn border_angle(&self, angle: f64) -> Pt {
        let (dx, dy) = unit(angle);
        self.border_dir(dx, dy)
    }

    /// A named anchor (`north east`, `center`, an angle such as `60`, a triangle's `apex`);
    /// the centre for unknown names.
    pub fn anchor(&self, name: &str) -> Pt {
        let name = crate::text::strip(name);
        if let ShapeKind::Triangle { apex, left, right } = &self.kind {
            match name {
                "apex" => return *apex,
                "left corner" => return *left,
                "right corner" => return *right,
                _ => {}
            }
        }
        if matches!(name, "center" | "text" | "base" | "mid") {
            return (self.cx, self.cy);
        }
        if let Some(a) = compass_angle(name) {
            return self.compass(a);
        }
        parse_float(name).map_or((self.cx, self.cy), |a| self.border_angle(a))
    }

    fn compass(&self, a: f64) -> Pt {
        let o = self.outer;
        match &self.kind {
            ShapeKind::Point => (self.cx, self.cy),
            ShapeKind::Rect { hw, hh, .. } => {
                let (w, h) = (hw + o, hh + o);
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
                (self.cx + sx * w, self.cy + sy * h)
            }
            ShapeKind::Diamond { hx, hy } => {
                if a % 90.0 == 0.0 {
                    return self.border_angle(a);
                }
                let sx = if a == 45.0 || a == 315.0 { 1.0 } else { -1.0 };
                let sy = if a == 45.0 || a == 135.0 { 1.0 } else { -1.0 };
                self.border_dir(sx * hx, sy * hy)
            }
            ShapeKind::Hexagon { hx, hy, slant } => {
                let p = self.hexagon_pts(*hx, *hy, *slant, o);
                let at = |k: usize| p.get(k).copied().unwrap_or((self.cx, self.cy));
                match a as i64 {
                    0 => at(0),
                    45 => at(1),
                    90 => (self.cx, at(1).1),
                    135 => at(2),
                    180 => at(3),
                    225 => at(4),
                    270 => (self.cx, at(4).1),
                    _ => at(5),
                }
            }
            ShapeKind::Ellipse { .. } | ShapeKind::Triangle { .. } => self.border_angle(a),
        }
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        match &self.kind {
            ShapeKind::Diamond { hx, hy } => {
                *hx != 0.0 && *hy != 0.0 && (x - self.cx).abs() / hx + (y - self.cy).abs() / hy <= 1.0
            }
            ShapeKind::Ellipse { a, b } => {
                *a != 0.0 && *b != 0.0 && ((x - self.cx) / a).powi(2) + ((y - self.cy) / b).powi(2) <= 1.0
            }
            ShapeKind::Triangle { apex, left, right } => {
                let side = |(ax, ay): Pt, (bx, by): Pt| (bx - ax) * (y - ay) - (by - ay) * (x - ax);
                let (s1, s2, s3) = (side(*left, *right), side(*right, *apex), side(*apex, *left));
                (s1 >= 0.0 && s2 >= 0.0 && s3 >= 0.0) || (s1 <= 0.0 && s2 <= 0.0 && s3 <= 0.0)
            }
            ShapeKind::Hexagon { hx, hy, slant } => {
                let (ax, ay) = ((x - self.cx).abs(), (y - self.cy).abs());
                if ay > *hy || ax > *hx {
                    return false;
                }
                *hy != 0.0 && ax <= hx - slant * ay / hy
            }
            ShapeKind::Point | ShapeKind::Rect { .. } => {
                let (x0, y0, x1, y1) = self.bbox();
                x0 <= x && x <= x1 && y0 <= y && y <= y1
            }
        }
    }
}

// ----------------------------------------------------------------- shape sizes (from PGF)
pub fn point_shape(cx: f64, cy: f64) -> Shape {
    Shape { cx, cy, outer: 0.0, kind: ShapeKind::Point }
}

#[allow(clippy::too_many_arguments)]
pub fn rect_shape(
    cx: f64,
    cy: f64,
    tw: f64,
    th: f64,
    inner: f64,
    min_w: f64,
    min_h: f64,
    lw: f64,
    radius: f64,
) -> Shape {
    let hw = (tw / 2.0 + inner).max(min_w / 2.0);
    let hh = (th / 2.0 + inner).max(min_h / 2.0);
    Shape { cx, cy, outer: lw / 2.0, kind: ShapeKind::Rect { hw, hh, radius } }
}

#[allow(clippy::too_many_arguments)]
pub fn diamond_shape(
    cx: f64,
    cy: f64,
    tw: f64,
    th: f64,
    inner: f64,
    min_w: f64,
    min_h: f64,
    aspect: f64,
    lw: f64,
) -> Shape {
    let (xi, yi) = (tw / 2.0 + inner, th / 2.0 + inner);
    let x = xi + aspect * yi;
    let y = yi + xi / aspect;
    Shape {
        cx,
        cy,
        outer: lw / 2.0,
        kind: ShapeKind::Diamond { hx: x.max(min_w / 2.0), hy: y.max(min_h / 2.0) },
    }
}

/// See `\er@hexcalc`: wide enough for the corners of the text under the slanted sides.
#[allow(clippy::too_many_arguments)]
pub fn hexagon_shape(
    cx: f64,
    cy: f64,
    tw: f64,
    th: f64,
    inner: f64,
    min_w: f64,
    min_h: f64,
    slant: f64,
    lw: f64,
) -> Shape {
    let hy = (th / 2.0 + inner).max(min_h / 2.0);
    let s = slant * hy;
    let hx = (tw / 2.0 + inner + s * (th / 2.0 + inner) / hy).max(min_w / 2.0);
    Shape { cx, cy, outer: lw / 2.0, kind: ShapeKind::Hexagon { hx, hy, slant: s } }
}

pub fn circle_shape(cx: f64, cy: f64, tw: f64, th: f64, inner: f64, min_size: f64, lw: f64) -> Shape {
    let r = (tw / 2.0 + inner).hypot(th / 2.0 + inner).max(min_size / 2.0);
    Shape { cx, cy, outer: lw / 2.0, kind: ShapeKind::Ellipse { a: r, b: r } }
}

#[allow(clippy::too_many_arguments)]
pub fn ellipse_shape(
    cx: f64,
    cy: f64,
    tw: f64,
    th: f64,
    inner: f64,
    min_w: f64,
    min_h: f64,
    lw: f64,
) -> Shape {
    let a = (std::f64::consts::SQRT_2 * (tw / 2.0 + inner)).max(min_w / 2.0);
    let b = (std::f64::consts::SQRT_2 * (th / 2.0 + inner)).max(min_h / 2.0);
    Shape { cx, cy, outer: lw / 2.0, kind: ShapeKind::Ellipse { a, b } }
}

/// pgflibraryshapes.geometric: an isosceles triangle rotated by 90 degrees (apex up), not
/// stretching.  (cx, cy) is the centre of the text.
#[allow(clippy::too_many_arguments)]
pub fn triangle_shape(
    cx: f64,
    cy: f64,
    tw: f64,
    th: f64,
    inner: f64,
    min_w: f64,
    min_h: f64,
    apex_angle: f64,
    lw: f64,
) -> Shape {
    let half = (apex_angle / 2.0).to_radians();
    let (tan, cot) = (half.tan(), 1.0 / half.tan());
    // the text's half sizes; the rotation swaps them
    let (x0, y0) = (tw / 2.0 + inner, th / 2.0 + inner);
    let (x, y) = (y0, x0);
    let mut xa = 2.0 * x;
    let mut ya = tan * xa;
    xa += cot * y;
    ya += y;
    if ya < min_w / 2.0 {
        ya = min_w / 2.0;
        xa = cot * ya;
    }
    if xa < min_h {
        xa = min_h;
        ya = tan * xa;
    }
    let offset = ((ya - y) * half.cos() - 2.0 * x * half.sin()) / (1.0 + half.sin()) + x;
    Shape {
        cx,
        cy,
        outer: lw / 2.0,
        kind: ShapeKind::Triangle {
            apex: (cx, cy + xa - offset),
            left: (cx - ya, cy - offset),
            right: (cx + ya, cy - offset),
        },
    }
}

// ----------------------------------------------------------------- paths
pub fn bezier(p0: Pt, p1: Pt, p2: Pt, p3: Pt, t: f64) -> Pt {
    let u = 1.0 - t;
    let f = |a: f64, b: f64, c: f64, d: f64| {
        u.powi(3) * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t.powi(3) * d
    };
    (f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1))
}

pub fn bezier_tangent(p0: Pt, p1: Pt, p2: Pt, p3: Pt, t: f64) -> Pt {
    let u = 1.0 - t;
    let f = |a: f64, b: f64, c: f64, d: f64| {
        3.0 * u * u * (b - a) + 6.0 * u * t * (c - b) + 3.0 * t * t * (d - c)
    };
    (f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1))
}

/// One piece of an edge: a line, or a cubic curve with control points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub p0: Pt,
    pub p3: Pt,
    pub controls: Option<(Pt, Pt)>,
}

impl Segment {
    pub fn line(p0: Pt, p3: Pt) -> Self {
        Segment { p0, p3, controls: None }
    }

    pub fn curve(p0: Pt, p3: Pt, c1: Pt, c2: Pt) -> Self {
        Segment { p0, p3, controls: Some((c1, c2)) }
    }

    pub fn point(&self, t: f64) -> Pt {
        match self.controls {
            None => (self.p0.0 + t * (self.p3.0 - self.p0.0), self.p0.1 + t * (self.p3.1 - self.p0.1)),
            Some((c1, c2)) => bezier(self.p0, c1, c2, self.p3, t),
        }
    }

    pub fn tangent(&self, t: f64) -> Pt {
        let chord = (self.p3.0 - self.p0.0, self.p3.1 - self.p0.1);
        match self.controls {
            None => chord,
            Some((c1, c2)) => {
                let d = bezier_tangent(self.p0, c1, c2, self.p3, t);
                if d.0.abs() + d.1.abs() < 1e-9 { chord } else { d }
            }
        }
    }

    pub fn length(&self) -> f64 {
        match self.controls {
            None => (self.p3.0 - self.p0.0).hypot(self.p3.1 - self.p0.1),
            Some(_) => {
                let pts: Vec<Pt> = (0..=16).map(|k| self.point(f64::from(k) / 16.0)).collect();
                crate::text::py_sum(pts.windows(2).map(|w| match w {
                    [a, b] => (b.0 - a.0).hypot(b.1 - a.1),
                    _ => 0.0,
                }))
            }
        }
    }

    pub fn ops(&self) -> PathOp {
        match self.controls {
            None => PathOp::Line(self.p3),
            Some((c1, c2)) => PathOp::Curve(c1, c2, self.p3),
        }
    }
}

/// Shortens a segment at its end (for arrow heads).
pub fn shorten_end(seg: &Segment, amount: f64) -> Segment {
    let length = seg.length();
    if length <= amount + 0.1 {
        return *seg;
    }
    match seg.controls {
        None => Segment::line(seg.p0, seg.point(1.0 - amount / length)),
        Some((c1, c2)) => {
            // the end point moves back along the final tangent; the shape stays
            let (tx, ty) = seg.tangent(1.0);
            let n = tx.hypot(ty);
            let n = if n == 0.0 { 1.0 } else { n };
            let p3 = (seg.p3.0 - amount * tx / n, seg.p3.1 - amount * ty / n);
            let c2 = (c2.0 - amount * tx / n, c2.1 - amount * ty / n);
            Segment::curve(seg.p0, p3, c1, c2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_borders_and_anchors() {
        let r = rect_shape(0.0, 0.0, 20.0, 10.0, 5.0, 80.0, 28.0, 0.6, 0.0);
        assert_eq!(r.anchor("east"), (40.3, 0.0));
        assert_eq!(r.border_toward(100.0, 0.0), (40.3, 0.0));
        assert_eq!(r.anchor("nonsense"), (0.0, 0.0));
        assert!(r.contains(39.0, 13.0));
    }

    #[test]
    fn segments() {
        let s = Segment::line((0.0, 0.0), (10.0, 0.0));
        assert_eq!(s.point(0.5), (5.0, 0.0));
        assert_eq!(shorten_end(&s, 4.0).p3, (6.0, 0.0));
    }
}
