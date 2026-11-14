//! A layout as SVG.  Coordinates are TeX points; y points up, as in the layout, until
//! [`document`] flips the whole drawing once.  Every element is a `<g data-key="…">`.
//!
//! Text is written either as glyph outlines (looks the same in every program, used for
//! comparisons and exports) or as `<text>` in the bundled fonts, drawn a hundred times larger
//! and scaled down: WebKit rounds text widths to whole pixels at small sizes (Milestone 0).
//!
//! On the dark canvas, colours are changed while drawing ([`Colors::DarkCanvas`], as the
//! Python app's `DarkCanvas`): on screen only, never in a file, LaTeX or an export.

use std::collections::{BTreeSet, HashSet};
use std::fmt::Write as _;

use crate::colors::parse_color;
use crate::fonts::{self, Family, Metrics, Style as FontStyle};
use crate::geometry::PathOp;
use crate::latex_text::Underline;
use crate::layout::{Arrow, Gfx, Layout, PathPrim, Prim, Rgb, TextPrim};
use crate::model::ElementKey;

/// How text is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextMode {
    Outlines,
    /// `<text>` with the font file's name as its font family (the page registers the fonts)
    Text,
}

/// The lightness of the dark canvas.
pub const CANVAS_DARK: f64 = 0.11;

/// Which colours are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Colors {
    /// as the diagram says: everything outside of a dark canvas
    Original,
    /// black becomes light, white the colour of the canvas, greys in between; fills you chose
    /// stay; coloured lines and text get as light as they need to be readable, in the same hue;
    /// text on a fill you chose goes by that fill
    DarkCanvas,
}

fn is_neutral(c: Rgb) -> bool {
    c.0.max(c.1).max(c.2) - c.0.min(c.1).min(c.2) < 0.04
}

fn neutral_on_dark(c: Rgb) -> Rgb {
    let v = 0.90 - (0.90 - CANVAS_DARK) * ((c.0 + c.1 + c.2) / 3.0);
    (v, v, v)
}

/// Python's `colorsys.rgb_to_hls`.
fn rgb_to_hls(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let (maxc, minc) = (r.max(g).max(b), r.min(g).min(b));
    let (sumc, rangec) = (maxc + minc, maxc - minc);
    let l = sumc / 2.0;
    if minc == maxc {
        return (0.0, l, 0.0);
    }
    let s = if l <= 0.5 { rangec / sumc } else { rangec / (2.0 - maxc - minc) };
    let (rc, gc, bc) = ((maxc - r) / rangec, (maxc - g) / rangec, (maxc - b) / rangec);
    let h = if r == maxc {
        bc - gc
    } else if g == maxc {
        2.0 + rc - bc
    } else {
        4.0 + gc - rc
    };
    (crate::text::py_mod(h / 6.0, 1.0), l, s)
}

/// Python's `colorsys.hls_to_rgb`.
fn hls_to_rgb(h: f64, l: f64, s: f64) -> Rgb {
    if s == 0.0 {
        return (l, l, l);
    }
    let m2 = if l <= 0.5 { l * (1.0 + s) } else { l + s - (l * s) };
    let m1 = 2.0 * l - m2;
    let v = |hue: f64| {
        let hue = crate::text::py_mod(hue, 1.0);
        if hue < 1.0 / 6.0 {
            m1 + (m2 - m1) * hue * 6.0
        } else if hue < 0.5 {
            m2
        } else if hue < 2.0 / 3.0 {
            m1 + (m2 - m1) * (2.0 / 3.0 - hue) * 6.0
        } else {
            m1
        }
    };
    (v(h + 1.0 / 3.0), v(h), v(h - 1.0 / 3.0))
}

impl Colors {
    /// The colour of a fill.
    pub fn fill(self, c: Rgb) -> Rgb {
        match self {
            Colors::DarkCanvas if is_neutral(c) => neutral_on_dark(c),
            _ => c,
        }
    }

    /// The colour of a line, an arrow head or a dotted pattern.
    pub fn ink(self, c: Rgb) -> Rgb {
        match self {
            Colors::Original => c,
            Colors::DarkCanvas if is_neutral(c) => neutral_on_dark(c),
            Colors::DarkCanvas => {
                let (h, l, s) = rgb_to_hls(c.0, c.1, c.2);
                hls_to_rgb(h, l.max(0.68), s)
            }
        }
    }

    /// The colour of text, which may sit on a fill someone chose.
    pub fn text(self, c: Rgb, on_fill: Option<Rgb>) -> Rgb {
        match (self, on_fill) {
            (Colors::Original, _) => c,
            (_, None) => self.ink(c),
            (_, Some(_)) if !is_neutral(c) => c,
            (_, Some((r, g, b))) => {
                if 0.299 * r + 0.587 * g + 0.114 * b > 0.5 {
                    (0.0, 0.0, 0.0)
                } else {
                    (1.0, 1.0, 1.0)
                }
            }
        }
    }
}

/// What a drawing is for.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub text: TextMode,
    pub metrics: Metrics,
    pub colors: Colors,
    /// lines thinner than this (in pt) are drawn this wide, as PDF viewers draw thin lines one
    /// device pixel wide; 0 for none
    pub min_stroke: f64,
    /// the invisible junction points, which the canvas marks with a small circle
    pub points: HashSet<ElementKey>,
}

impl Options {
    /// For a file: outlines, the diagram's own colours, no minimum width, no markers.
    pub fn export(text: TextMode) -> Options {
        Options {
            text,
            metrics: Metrics::DesignSizes,
            colors: Colors::Original,
            min_stroke: 0.0,
            points: HashSet::new(),
        }
    }
}

/// A drawing: what goes into `<defs>`, the elements, and their extent (x0, y0, x1, y1).
#[derive(Clone, Debug, PartialEq)]
pub struct Drawing {
    pub defs: String,
    pub body: String,
    pub bbox: Option<(f64, f64, f64, f64)>,
}

fn color(c: Rgb) -> String {
    let to = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", to(c.0), to(c.1), to(c.2))
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

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The extent of a drawing: (x0, y0, x1, y1).
pub fn bbox(gfx: &Gfx) -> Option<(f64, f64, f64, f64)> {
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for p in &gfx.prims {
        match p {
            Prim::Path(p) => {
                for op in &p.segs {
                    let pts: Vec<(f64, f64)> = match op {
                        PathOp::Move(a) | PathOp::Line(a) => vec![*a],
                        PathOp::Curve(a, b, c) => vec![*a, *b, *c],
                        PathOp::Close => vec![],
                    };
                    for (x, y) in pts {
                        xs.push(x);
                        ys.push(y);
                    }
                }
                for a in &p.arrows {
                    xs.push(a.tip.0);
                    ys.push(a.tip.1);
                }
            }
            Prim::Text(t) => {
                let w = t.layout.width / 2.0 + t.inner;
                let h = (t.layout.height + t.layout.depth) / 2.0 + t.inner;
                let (rw, rh) = if t.rotation != 0.0 { (w.hypot(h), w.hypot(h)) } else { (w, h) };
                xs.extend([t.cx - rw, t.cx + rw]);
                ys.extend([t.cy - rh, t.cy + rh]);
            }
        }
    }
    if let Some(s) = &gfx.shape {
        let (x0, y0, x1, y1) = s.bbox();
        xs.extend([x0, x1]);
        ys.extend([y0, y1]);
    }
    let min = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
    let max = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (!xs.is_empty()).then(|| (min(&xs), min(&ys), max(&xs), max(&ys)))
}

/// Where a drawing goes: SVG elements with glyph outlines ([`SvgPainter`]) or with `<text>` (the
/// canvas).  The [`Writer`] decides what is drawn (colours, double lines, arrows, glyphs, underlines);
/// a painter only writes it.
pub(crate) trait Painter {
    fn fill(&mut self, ops: &[PathOp], c: Rgb);
    /// the dotted background of a subdomain
    fn dots(&mut self, ops: &[PathOp], ink: Rgb);
    fn stroke(&mut self, ops: &[PathOp], c: Rgb, width: f64, dash: Option<(f64, f64)>);
    /// translate, then rotate (degrees), until `pop`
    fn push(&mut self, x: f64, y: f64, rotate: f64);
    fn pop(&mut self);
    /// shaped glyphs of a font file at (x, y), scaled by k (font units to pt)
    fn glyphs(&mut self, file: &str, x: f64, y: f64, k: f64, glyphs: &[(u16, i64, i64)], c: Rgb);
    /// text in a bundled font (the canvas); painters without it draw the outlines instead
    fn text(&mut self, _file: &str, _x: f64, _y: f64, _size: f64, _text: &str, _c: Rgb) -> bool {
        false
    }
    fn begin_element(&mut self, _key: u64) {}
    fn end_element(&mut self) {}
    /// an invisible junction (the canvas only)
    fn junction(&mut self, _x: f64, _y: f64) {}
}

/// SVG elements.
pub(crate) struct SvgPainter {
    pub out: String,
    pub glyphs: BTreeSet<(String, u16)>,
    pub patterns: BTreeSet<String>,
}

impl Painter for SvgPainter {
    fn fill(&mut self, ops: &[PathOp], c: Rgb) {
        let _ = write!(self.out, r#"<path d="{}" fill="{}" stroke="none"/>"#, path_d(ops), color(c));
    }

    fn dots(&mut self, ops: &[PathOp], ink: Rgb) {
        let ink = color(ink);
        let id = ink.trim_start_matches('#').to_string();
        let _ = write!(self.out, r#"<path d="{}" fill="url(#dots-{id})" stroke="none"/>"#, path_d(ops));
        self.patterns.insert(ink);
    }

    fn stroke(&mut self, ops: &[PathOp], c: Rgb, width: f64, dash: Option<(f64, f64)>) {
        let dash = dash.map_or(String::new(), |(a, b)| format!(r#" stroke-dasharray="{} {}""#, n(a), n(b)));
        let _ = write!(
            self.out,
            r#"<path d="{}" fill="none" stroke-linecap="butt" stroke-linejoin="miter" stroke="{}" stroke-width="{}"{dash}/>"#,
            path_d(ops),
            color(c),
            n(width)
        );
    }

    fn push(&mut self, x: f64, y: f64, rotate: f64) {
        let rot = if rotate != 0.0 { format!(" rotate({})", n(rotate)) } else { String::new() };
        let _ = write!(self.out, r#"<g transform="translate({} {}){rot}">"#, n(x), n(y));
    }

    fn pop(&mut self) {
        self.out.push_str("</g>");
    }

    fn glyphs(&mut self, file: &str, x: f64, y: f64, k: f64, glyphs: &[(u16, i64, i64)], c: Rgb) {
        let _ = write!(
            self.out,
            r#"<g fill="{}" transform="translate({} {}) scale({k})">"#,
            color(c),
            n(x),
            n(y)
        );
        for (id, gx, gy) in glyphs {
            self.glyphs.insert((file.to_string(), *id));
            let _ = write!(self.out, r##"<use href="#g-{file}-{id}" x="{gx}" y="{gy}"/>"##);
        }
        self.out.push_str("</g>");
    }

    fn begin_element(&mut self, key: u64) {
        let _ = write!(self.out, r#"<g data-key="{key}">"#);
    }

    fn end_element(&mut self) {
        self.out.push_str("</g>");
    }

    fn junction(&mut self, x: f64, y: f64) {
        // an invisible junction: a small grey circle, one pixel wide
        let _ = write!(
            self.out,
            r##"<circle cx="{}" cy="{}" r="1.5" fill="none" stroke="#787878" stroke-width="1" vector-effect="non-scaling-stroke"/>"##,
            n(x),
            n(y)
        );
    }
}

/// `<text>` in the bundled fonts (the canvas), drawn a hundred times larger and scaled down.
struct TextSvgPainter(SvgPainter);

impl Painter for TextSvgPainter {
    fn fill(&mut self, ops: &[PathOp], c: Rgb) {
        self.0.fill(ops, c);
    }
    fn dots(&mut self, ops: &[PathOp], ink: Rgb) {
        self.0.dots(ops, ink);
    }
    fn stroke(&mut self, ops: &[PathOp], c: Rgb, width: f64, dash: Option<(f64, f64)>) {
        self.0.stroke(ops, c, width, dash);
    }
    fn push(&mut self, x: f64, y: f64, rotate: f64) {
        self.0.push(x, y, rotate);
    }
    fn pop(&mut self) {
        self.0.pop();
    }
    fn glyphs(&mut self, file: &str, x: f64, y: f64, k: f64, glyphs: &[(u16, i64, i64)], c: Rgb) {
        self.0.glyphs(file, x, y, k, glyphs, c);
    }
    fn text(&mut self, file: &str, x: f64, y: f64, size: f64, text: &str, c: Rgb) -> bool {
        let _ = write!(
            self.0.out,
            r#"<text transform="translate({} {}) scale(0.01 -0.01)" font-family="{file}" font-size="{}" fill="{}" xml:space="preserve">{}</text>"#,
            n(x),
            n(y),
            n(size * 100.0),
            color(c),
            escape(text)
        );
        true
    }
    fn begin_element(&mut self, key: u64) {
        self.0.begin_element(key);
    }
    fn end_element(&mut self) {
        self.0.end_element();
    }
    fn junction(&mut self, x: f64, y: f64) {
        self.0.junction(x, y);
    }
}

/// What is drawn, for any painter.
pub(crate) struct Writer<'o, P: Painter> {
    pub painter: P,
    opts: &'o Options,
}

impl<'o, P: Painter> Writer<'o, P> {
    pub fn new(painter: P, opts: &'o Options) -> Self {
        Writer { painter, opts }
    }

    fn width(&self, w: f64) -> f64 {
        w.max(self.opts.min_stroke)
    }

    fn path(&mut self, p: &PathPrim) {
        let colors = self.opts.colors;
        if let Some(fill) = p.fill {
            self.painter.fill(&p.segs, colors.fill(fill));
        }
        if let Some(c) = p.pattern {
            self.painter.dots(&p.segs, colors.ink(c));
        }
        if let Some(st) = &p.stroke
            && let Some(c) = st.color
        {
            let ink = colors.ink(c);
            match st.double {
                // a double line too thin to show its gap is drawn as one line
                Some(gap) if gap > 0.0 && 2.0 * st.width + gap >= self.opts.min_stroke => {
                    self.painter.stroke(&p.segs, ink, 2.0 * st.width + gap, st.dash);
                    self.painter.stroke(&p.segs, colors.fill((1.0, 1.0, 1.0)), gap, st.dash);
                }
                _ => self.painter.stroke(&p.segs, ink, self.width(st.width), st.dash),
            }
        }
        for a in &p.arrows {
            self.arrow(a);
        }
    }

    fn arrow(&mut self, a: &Arrow) {
        let Some(c) = a.color else { return };
        let (ux, uy) = (a.angle.to_radians().cos(), a.angle.to_radians().sin());
        let (tx, ty) = a.tip;
        let (bx, by) = (tx - a.length * ux, ty - a.length * uy);
        let (nx, ny) = (-uy * a.width / 2.0, ux * a.width / 2.0);
        let ops = [
            PathOp::Move((tx, ty)),
            PathOp::Line((bx + nx, by + ny)),
            PathOp::Line((bx - nx, by - ny)),
            PathOp::Close,
        ];
        self.painter.fill(&ops, self.opts.colors.ink(c));
    }

    fn text(&mut self, t: &TextPrim, on_fill: Option<Rgb>) {
        let colors = self.opts.colors;
        let lay = &t.layout;
        self.painter.push(t.cx, t.cy, t.rotation);
        if let Some(bg) = t.bg {
            let (w, h) = (lay.width / 2.0 + t.inner, (lay.height + lay.depth) / 2.0 + t.inner);
            let ops = [
                PathOp::Move((-w, -h)),
                PathOp::Line((w, -h)),
                PathOp::Line((w, h)),
                PathOp::Line((-w, h)),
                PathOp::Close,
            ];
            self.painter.fill(&ops, colors.fill(bg));
        }
        // the first baseline: the top of the box is at +(height + depth) / 2
        let base0 = (lay.height + lay.depth) / 2.0 - lay.height;
        let left = -lay.width / 2.0;
        for line in &lay.lines {
            for (x, run, w) in &line.runs {
                if run.text.is_empty() {
                    continue;
                }
                let f = &run.fmt;
                let c = match &f.color {
                    Some(expr) => parse_color(Some(expr), t.color),
                    None => t.color,
                };
                let Some(c) = c else { continue };
                let c = colors.text(c, on_fill);
                let bx = left + line.x + x;
                let by = base0 + line.y + f.rise;
                let family = if f.mono {
                    Family::Mono
                } else if f.sans {
                    Family::Sans
                } else {
                    Family::Roman
                };
                let file =
                    fonts::font_file(FontStyle::new(family, f.bold, f.italic), f.size, self.opts.metrics);
                let as_text =
                    self.opts.text == TextMode::Text && self.painter.text(file, bx, by, f.size, &run.text, c);
                if !as_text && let Some(shaped) = fonts::shape_text(file, &run.text) {
                    let k = f.size / shaped.units_per_em;
                    self.painter.glyphs(file, bx, by, k, &shaped.glyphs, c);
                }
                if f.underline != Underline::None {
                    let uy = by - 0.18 * f.size;
                    let dash = (f.underline == Underline::Dashed).then_some((2.0, 1.2));
                    let ops = [PathOp::Move((bx, uy)), PathOp::Line((bx + w, uy))];
                    self.painter.stroke(&ops, c, self.width(0.4), dash);
                }
            }
        }
        self.painter.pop();
    }

    /// The elements in drawing order; their extent.
    pub fn draw(&mut self, gfx: &[Gfx]) -> Option<(f64, f64, f64, f64)> {
        let opts = self.opts;
        let mut order: Vec<&Gfx> = gfx.iter().collect();
        order.sort_by(|a, b| a.z.total_cmp(&b.z));
        let mut extent: Option<(f64, f64, f64, f64)> = None;
        for g in &order {
            if let Some(b) = bbox(g) {
                extent = Some(extent.map_or(b, |e| (e.0.min(b.0), e.1.min(b.1), e.2.max(b.2), e.3.max(b.3))));
            }
            self.painter.begin_element(g.key.map_or(0, |k| k.0));
            // text sits on the first fill that someone chose
            let on_fill = match opts.colors {
                Colors::Original => None,
                Colors::DarkCanvas => g.prims.iter().find_map(|p| match p {
                    Prim::Path(p) => p.fill.filter(|f| !is_neutral(*f)),
                    Prim::Text(_) => None,
                }),
            };
            for p in &g.prims {
                match p {
                    Prim::Path(p) => self.path(p),
                    Prim::Text(t) => self.text(t, on_fill),
                }
            }
            if let Some(k) = g.key
                && opts.points.contains(&k)
                && let Some(crate::layout::Handle::Point((x, y))) = g.handles.get("center")
            {
                self.painter.junction(*x, *y);
            }
            self.painter.end_element();
        }
        extent
    }
}

/// The elements of a layout, drawn (y up).
pub fn draw(gfx: &[Gfx], opts: &Options) -> Drawing {
    let svg = SvgPainter { out: String::new(), glyphs: BTreeSet::new(), patterns: BTreeSet::new() };
    let (extent, svg) = if opts.text == TextMode::Text {
        let mut w = Writer::new(TextSvgPainter(svg), opts);
        let e = w.draw(gfx);
        (e, w.painter.0)
    } else {
        let mut w = Writer::new(svg, opts);
        let e = w.draw(gfx);
        (e, w.painter)
    };
    let mut defs = String::new();
    for (file, id) in &svg.glyphs {
        let _ =
            write!(defs, r#"<path id="g-{file}-{id}" d="{}"/>"#, path_d(&fonts::glyph_outline(file, *id)));
    }
    for c in &svg.patterns {
        let _ = write!(
            defs,
            r#"<pattern id="dots-{}" width="3" height="3" patternUnits="userSpaceOnUse"><circle cx="1.5" cy="1.5" r="0.25" fill="{c}"/></pattern>"#,
            c.trim_start_matches('#')
        );
    }
    Drawing { defs, body: svg.out, bbox: extent }
}

/// A drawing as an SVG document, with `margin` pt around it.
pub fn document(gfx: &[Gfx], opts: &Options, margin: f64) -> String {
    let d = draw(gfx, opts);
    let (x0, y0, x1, y1) = d.bbox.unwrap_or((0.0, 0.0, 1.0, 1.0));
    let (x0, y0, x1, y1) = (x0 - margin, y0 - margin, x1 + margin, y1 + margin);
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{} {} {} {}" width="{}pt" height="{}pt"><defs>{}</defs><g transform="scale(1 -1)">{}</g></svg>"#,
        n(x0),
        n(-y1),
        n(x1 - x0),
        n(y1 - y0),
        n(x1 - x0),
        n(y1 - y0),
        d.defs,
        d.body
    )
}

/// The whole layout as an SVG document, with `margin` pt around it.
pub fn render(layout: &Layout<'_>, mode: TextMode, metrics: Metrics, margin: f64) -> String {
    document(&layout.gfx, &Options { metrics, ..Options::export(mode) }, margin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_canvas_colours() {
        let dark = Colors::DarkCanvas;
        assert!((dark.fill((1.0, 1.0, 1.0)).0 - CANVAS_DARK).abs() < 1e-12);
        assert_eq!(dark.ink((0.0, 0.0, 0.0)), (0.9, 0.9, 0.9));
        assert_eq!(dark.fill((1.0, 0.8, 0.8)), (1.0, 0.8, 0.8)); // a chosen fill stays
        let red = dark.ink((0.8, 0.0, 0.0));
        assert!((rgb_to_hls(red.0, red.1, red.2).1 - 0.68).abs() < 1e-12);
        assert_eq!(dark.text((0.0, 0.0, 0.0), Some((1.0, 0.9, 0.6))), (0.0, 0.0, 0.0));
        assert_eq!(dark.text((0.0, 0.0, 0.0), Some((0.1, 0.1, 0.5))), (1.0, 1.0, 1.0));
        assert_eq!(Colors::Original.ink((0.2, 0.3, 0.4)), (0.2, 0.3, 0.4));
    }
}
