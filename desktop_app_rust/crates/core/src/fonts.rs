//! Text metrics read directly from the bundled Latin Modern fonts, at the design size LaTeX
//! would use: 9 pt text is set in lmroman9, not in lmroman10 scaled down.

use std::collections::HashMap;
use std::sync::OnceLock;

use rustybuzz::{Face, UnicodeBuffer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    Roman,
    Sans,
    Mono,
}

/// A font as a diagram asks for it: \rmfamily / \sffamily / \ttfamily, \bfseries, \itshape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Style {
    pub family: Family,
    pub bold: bool,
    pub italic: bool,
}

impl Style {
    pub const fn new(family: Family, bold: bool, italic: bool) -> Self {
        Style { family, bold, italic }
    }
}

static FILES: &[(&str, &[u8])] = &[
    ("lmmono10-regular", include_bytes!("../../../fonts/lmmono10-regular.otf")),
    ("lmmono12-regular", include_bytes!("../../../fonts/lmmono12-regular.otf")),
    ("lmmono8-regular", include_bytes!("../../../fonts/lmmono8-regular.otf")),
    ("lmmono9-regular", include_bytes!("../../../fonts/lmmono9-regular.otf")),
    ("lmroman10-bold", include_bytes!("../../../fonts/lmroman10-bold.otf")),
    ("lmroman10-bolditalic", include_bytes!("../../../fonts/lmroman10-bolditalic.otf")),
    ("lmroman10-italic", include_bytes!("../../../fonts/lmroman10-italic.otf")),
    ("lmroman10-regular", include_bytes!("../../../fonts/lmroman10-regular.otf")),
    ("lmroman12-bold", include_bytes!("../../../fonts/lmroman12-bold.otf")),
    ("lmroman12-italic", include_bytes!("../../../fonts/lmroman12-italic.otf")),
    ("lmroman12-regular", include_bytes!("../../../fonts/lmroman12-regular.otf")),
    ("lmroman17-regular", include_bytes!("../../../fonts/lmroman17-regular.otf")),
    ("lmroman5-bold", include_bytes!("../../../fonts/lmroman5-bold.otf")),
    ("lmroman5-regular", include_bytes!("../../../fonts/lmroman5-regular.otf")),
    ("lmroman6-bold", include_bytes!("../../../fonts/lmroman6-bold.otf")),
    ("lmroman6-regular", include_bytes!("../../../fonts/lmroman6-regular.otf")),
    ("lmroman7-bold", include_bytes!("../../../fonts/lmroman7-bold.otf")),
    ("lmroman7-italic", include_bytes!("../../../fonts/lmroman7-italic.otf")),
    ("lmroman7-regular", include_bytes!("../../../fonts/lmroman7-regular.otf")),
    ("lmroman8-bold", include_bytes!("../../../fonts/lmroman8-bold.otf")),
    ("lmroman8-italic", include_bytes!("../../../fonts/lmroman8-italic.otf")),
    ("lmroman8-regular", include_bytes!("../../../fonts/lmroman8-regular.otf")),
    ("lmroman9-bold", include_bytes!("../../../fonts/lmroman9-bold.otf")),
    ("lmroman9-italic", include_bytes!("../../../fonts/lmroman9-italic.otf")),
    ("lmroman9-regular", include_bytes!("../../../fonts/lmroman9-regular.otf")),
    ("lmsans10-bold", include_bytes!("../../../fonts/lmsans10-bold.otf")),
    ("lmsans10-boldoblique", include_bytes!("../../../fonts/lmsans10-boldoblique.otf")),
    ("lmsans10-oblique", include_bytes!("../../../fonts/lmsans10-oblique.otf")),
    ("lmsans10-regular", include_bytes!("../../../fonts/lmsans10-regular.otf")),
    ("lmsans12-oblique", include_bytes!("../../../fonts/lmsans12-oblique.otf")),
    ("lmsans12-regular", include_bytes!("../../../fonts/lmsans12-regular.otf")),
    ("lmsans17-oblique", include_bytes!("../../../fonts/lmsans17-oblique.otf")),
    ("lmsans17-regular", include_bytes!("../../../fonts/lmsans17-regular.otf")),
    ("lmsans8-oblique", include_bytes!("../../../fonts/lmsans8-oblique.otf")),
    ("lmsans8-regular", include_bytes!("../../../fonts/lmsans8-regular.otf")),
    ("lmsans9-oblique", include_bytes!("../../../fonts/lmsans9-oblique.otf")),
    ("lmsans9-regular", include_bytes!("../../../fonts/lmsans9-regular.otf")),
];

/// The first entry whose upper bound is above `size`, else `last` (the ranges of an .fd file).
fn pick(size: f64, ranges: &[(f64, &'static str)], last: &'static str) -> &'static str {
    ranges.iter().find(|(upper, _)| size < *upper).map_or(last, |(_, stem)| stem)
}

/// The font file LaTeX uses for `style` at `size` pt with lmodern in T1 encoding
/// (t1lmr.fd, t1lmss.fd, t1lmtt.fd). \bfseries is bold extended; \itshape is oblique in sans.
/// Bold or italic monospace isn't bundled and falls back on the upright medium face.
pub fn design_font(style: Style, size: f64) -> &'static str {
    match (style.family, style.bold, style.italic) {
        (Family::Roman, false, false) => pick(
            size,
            &[
                (5.5, "lmroman5-regular"),
                (6.5, "lmroman6-regular"),
                (7.5, "lmroman7-regular"),
                (8.5, "lmroman8-regular"),
                (9.5, "lmroman9-regular"),
                (11.0, "lmroman10-regular"),
                (15.0, "lmroman12-regular"),
            ],
            "lmroman17-regular",
        ),
        (Family::Roman, false, true) => pick(
            size,
            &[
                (7.5, "lmroman7-italic"),
                (8.5, "lmroman8-italic"),
                (9.5, "lmroman9-italic"),
                (11.0, "lmroman10-italic"),
            ],
            "lmroman12-italic",
        ),
        (Family::Roman, true, false) => pick(
            size,
            &[
                (5.5, "lmroman5-bold"),
                (6.5, "lmroman6-bold"),
                (7.5, "lmroman7-bold"),
                (8.5, "lmroman8-bold"),
                (9.5, "lmroman9-bold"),
                (11.0, "lmroman10-bold"),
            ],
            "lmroman12-bold",
        ),
        (Family::Roman, true, true) => "lmroman10-bolditalic",
        (Family::Sans, false, false) => pick(
            size,
            &[
                (8.5, "lmsans8-regular"),
                (9.5, "lmsans9-regular"),
                (11.0, "lmsans10-regular"),
                (15.5, "lmsans12-regular"),
            ],
            "lmsans17-regular",
        ),
        (Family::Sans, false, true) => pick(
            size,
            &[
                (8.5, "lmsans8-oblique"),
                (9.5, "lmsans9-oblique"),
                (11.0, "lmsans10-oblique"),
                (15.5, "lmsans12-oblique"),
            ],
            "lmsans17-oblique",
        ),
        (Family::Sans, true, false) => "lmsans10-bold",
        (Family::Sans, true, true) => "lmsans10-boldoblique",
        (Family::Mono, _, _) => pick(
            size,
            &[(8.5, "lmmono8-regular"), (9.5, "lmmono9-regular"), (11.0, "lmmono10-regular")],
            "lmmono12-regular",
        ),
    }
}

/// The bundled font files: name (as `design_font` returns it) and contents.
pub fn files() -> impl Iterator<Item = (&'static str, &'static [u8])> {
    FILES.iter().copied()
}

fn face(stem: &str) -> Option<&'static Face<'static>> {
    static FACES: OnceLock<HashMap<&'static str, Face<'static>>> = OnceLock::new();
    FACES
        .get_or_init(|| {
            FILES.iter().filter_map(|(s, data)| Face::from_slice(data, 0).map(|f| (*s, f))).collect()
        })
        .get(stem)
}

/// How text is measured.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Metrics {
    /// as LaTeX sets it: the design size LaTeX chooses for each size
    #[default]
    DesignSizes,
    /// as the Python app measures (Qt with the 10 pt fonts scaled; monospace is never bold);
    /// used to compare the two layouts
    TenPoint,
}

/// The pixel size at which the Python app has Qt measure text.
const QT_REFERENCE_PX: f64 = 100.0;

/// The font file a run is set in.
pub fn font_file(style: Style, size: f64, metrics: Metrics) -> &'static str {
    match metrics {
        Metrics::DesignSizes => design_font(style, size),
        Metrics::TenPoint => match (style.family, style.bold, style.italic) {
            (Family::Mono, _, _) => "lmmono10-regular",
            (Family::Roman, false, false) => "lmroman10-regular",
            (Family::Roman, false, true) => "lmroman10-italic",
            (Family::Roman, true, false) => "lmroman10-bold",
            (Family::Roman, true, true) => "lmroman10-bolditalic",
            (Family::Sans, false, false) => "lmsans10-regular",
            (Family::Sans, false, true) => "lmsans10-oblique",
            (Family::Sans, true, false) => "lmsans10-bold",
            (Family::Sans, true, true) => "lmsans10-boldoblique",
        },
    }
}

/// Shaped text, in font units.
pub struct Shaped {
    /// (glyph id, x offset, y offset)
    pub glyphs: Vec<(u16, i64, i64)>,
    pub advance: i64,
    pub units_per_em: f64,
}

/// The shaped glyphs of `text` in a font file.
pub fn shape_text(file: &str, text: &str) -> Option<Shaped> {
    let face = face(file)?;
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    let glyphs = rustybuzz::shape(face, &[], buffer);
    let mut x = 0i64;
    let mut out = Vec::new();
    for (info, pos) in glyphs.glyph_infos().iter().zip(glyphs.glyph_positions()) {
        let id = u16::try_from(info.glyph_id).unwrap_or(0);
        out.push((id, x + i64::from(pos.x_offset), i64::from(pos.y_offset)));
        x += i64::from(pos.x_advance);
    }
    Some(Shaped { glyphs: out, advance: x, units_per_em: f64::from(face.units_per_em()) })
}

/// The advance of each glyph of `text` (kerning included), in font units.
fn glyph_advances(file: &str, text: &str) -> Vec<i64> {
    let Some(face) = face(file) else { return vec![] };
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    let glyphs = rustybuzz::shape(face, &[], buffer);
    glyphs.glyph_positions().iter().map(|p| i64::from(p.x_advance)).collect()
}

/// (width, height above the baseline, depth below it) in pt: the advance, and the ink of the
/// glyphs, as TeX's box dimensions and Qt's `tightBoundingRect` give them.  White space has no
/// height or depth.
pub fn run_metrics(style: Style, size: f64, text: &str, metrics: Metrics) -> (f64, f64, f64) {
    let file = font_file(style, size, metrics);
    let (Some(face), Some(Shaped { glyphs, advance, units_per_em: upem })) =
        (face(file), shape_text(file, text))
    else {
        return (0.0, 0.0, 0.0);
    };
    let scale = size / upem;
    let width = match metrics {
        Metrics::DesignSizes => advance as f64 * scale,
        // Qt (FreeType) scales each advance to 1/64 pixel at 100 px and truncates
        Metrics::TenPoint => {
            let x_scale = (6400i64 << 16) / (upem as i64).max(1);
            let px64: i64 = glyph_advances(file, text).iter().map(|a| (a * x_scale) >> 16).sum();
            px64 as f64 / 64.0 * size / QT_REFERENCE_PX
        }
    };
    if text.chars().all(crate::text::is_space) {
        return (width, 0.0, 0.0);
    }
    let (mut top, mut bottom) = (0.0f64, 0.0f64);
    for (id, _, dy) in glyphs {
        if let Some(r) = face.glyph_bounding_box(rustybuzz::ttf_parser::GlyphId(id)) {
            top = top.max(f64::from(r.y_max) + dy as f64);
            bottom = bottom.max(-(f64::from(r.y_min) + dy as f64));
        }
    }
    match metrics {
        Metrics::DesignSizes => (width, top * scale, bottom * scale),
        // Qt measures the ink in whole pixels of its 100 px reference size, rounded outwards
        Metrics::TenPoint => {
            let px =
                |units: f64| (units * QT_REFERENCE_PX / upem - 1e-9).ceil().max(0.0) * size / QT_REFERENCE_PX;
            (width, px(top), px(bottom))
        }
    }
}

/// The outline of a glyph, in font units (y up), as path operations.
pub fn glyph_outline(file: &str, id: u16) -> Vec<crate::geometry::PathOp> {
    use crate::geometry::PathOp;
    struct Builder(Vec<PathOp>);
    impl rustybuzz::ttf_parser::OutlineBuilder for Builder {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0.push(PathOp::Move((f64::from(x), f64::from(y))));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0.push(PathOp::Line((f64::from(x), f64::from(y))));
        }
        fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
            // a quadratic curve as a cubic one
            let last = match self.0.last() {
                Some(PathOp::Move(p) | PathOp::Line(p) | PathOp::Curve(_, _, p)) => *p,
                _ => (0.0, 0.0),
            };
            let (qx, qy) = (f64::from(x1), f64::from(y1));
            let (ex, ey) = (f64::from(x), f64::from(y));
            let c1 = (last.0 + 2.0 / 3.0 * (qx - last.0), last.1 + 2.0 / 3.0 * (qy - last.1));
            let c2 = (ex + 2.0 / 3.0 * (qx - ex), ey + 2.0 / 3.0 * (qy - ey));
            self.0.push(PathOp::Curve(c1, c2, (ex, ey)));
        }
        fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
            self.0.push(PathOp::Curve(
                (f64::from(x1), f64::from(y1)),
                (f64::from(x2), f64::from(y2)),
                (f64::from(x), f64::from(y)),
            ));
        }
        fn close(&mut self) {
            self.0.push(PathOp::Close);
        }
    }
    let mut b = Builder(Vec::new());
    if let Some(face) = face(file) {
        let _ = face.outline_glyph(rustybuzz::ttf_parser::GlyphId(id), &mut b);
    }
    b.0
}

/// Width in pt of `text` in `style` at `size` pt, shaped with kerning and ligatures.
pub fn text_width(style: Style, size: f64, text: &str) -> f64 {
    run_metrics(style, size, text, Metrics::DesignSizes).0
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROMAN: Style = Style::new(Family::Roman, false, false);
    const BOLD: Style = Style::new(Family::Roman, true, false);

    #[test]
    fn every_font_the_table_names_is_bundled() {
        for family in [Family::Roman, Family::Sans, Family::Mono] {
            for (bold, italic) in [(false, false), (false, true), (true, false), (true, true)] {
                for size in [4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 10.95, 12.0, 14.4, 17.28, 20.74] {
                    let stem = design_font(Style::new(family, bold, italic), size);
                    assert!(face(stem).is_some(), "{stem}");
                }
            }
        }
    }

    #[test]
    fn sizes_pick_their_design_size_as_latex_does() {
        assert_eq!(design_font(ROMAN, 9.0), "lmroman9-regular");
        assert_eq!(design_font(ROMAN, 8.0), "lmroman8-regular");
        assert_eq!(design_font(ROMAN, 10.95), "lmroman10-regular");
        assert_eq!(design_font(ROMAN, 11.0), "lmroman12-regular");
        assert_eq!(design_font(BOLD, 14.4), "lmroman12-bold");
    }

    #[test]
    fn widths_and_ligatures() {
        let w = text_width(BOLD, 10.0, "Halter");
        assert!((w - 32.26).abs() < 0.01, "{w}"); // LaTeX: 32.26pt
        let joined = text_width(ROMAN, 10.0, "fi");
        let apart = text_width(ROMAN, 10.0, "f") + text_width(ROMAN, 10.0, "i");
        assert!(joined < apart, "{joined} {apart}");
    }

    #[test]
    fn the_9pt_font_is_wider_than_10pt_scaled() {
        // design sizes: smaller fonts are drawn relatively wider, which is the point
        assert!(text_width(ROMAN, 9.0, "HalterID") > 0.9 * text_width(ROMAN, 10.0, "HalterID"));
    }
}
