//! Text boxes, and how text is measured.

use super::prims::{LineLayout, Rgb, TextLayout, TextPrim};
use super::{Layout, anchor_offset};
use crate::fonts::{Family, Metrics, Style as FontStyle, run_metrics};
use crate::latex_text::{BASELINE, Fmt, Run, TextBlock, typeset};
use crate::text::is_space;

/// Measures runs of text: (width, height above the baseline, depth below it) in pt.
pub trait TextEngine {
    fn run_metrics(&self, run: &Run) -> (f64, f64, f64);
}

/// The Python app's rough metrics for tests without fonts: half the size per character.
/// Both apps lay out alike with it, which is how their layouts are compared exactly.
#[derive(Clone, Copy, Debug, Default)]
pub struct DummyEngine;

impl TextEngine for DummyEngine {
    fn run_metrics(&self, run: &Run) -> (f64, f64, f64) {
        let s = run.fmt.size;
        (0.5 * s * run.text.chars().count() as f64, 0.7 * s, 0.2 * s)
    }
}

/// Metrics from the bundled Latin Modern fonts.
#[derive(Clone, Copy, Debug, Default)]
pub struct FontEngine {
    pub metrics: Metrics,
}

impl TextEngine for FontEngine {
    fn run_metrics(&self, run: &Run) -> (f64, f64, f64) {
        let f = &run.fmt;
        let family = if f.mono {
            Family::Mono
        } else if f.sans {
            Family::Sans
        } else {
            Family::Roman
        };
        run_metrics(FontStyle::new(family, f.bold, f.italic), f.size, &run.text, self.metrics)
    }
}

fn baseline_skip(size: f64) -> f64 {
    BASELINE.iter().find(|(s, _)| *s == size).map_or(size * 1.2, |(_, b)| *b)
}

impl Layout<'_> {
    /// Lines of runs placed in a box.
    pub(super) fn text_layout(
        &self,
        block: &TextBlock,
        centred: bool,
        fixed_height: Option<f64>,
        fixed_depth: Option<f64>,
    ) -> TextLayout {
        let mut lines: Vec<LineLayout> = Vec::new();
        let (mut y, mut top, mut bottom) = (0.0f64, 0.0f64, 0.0f64);
        for (k, line) in block.lines.iter().enumerate() {
            let (mut x, mut asc, mut desc) = (0.0f64, 0.0f64, 0.0f64);
            let mut size: Option<f64> = None;
            let mut runs = Vec::new();
            for run in line {
                let (w, h, d) = self.engine.run_metrics(run);
                runs.push((x, run.clone(), w));
                x += w;
                asc = asc.max(h + run.fmt.rise);
                desc = desc.max(d - run.fmt.rise);
                if !run.text.trim_matches(is_space).is_empty() {
                    size = Some(size.unwrap_or(0.0).max(run.fmt.size));
                }
            }
            if k > 0 {
                let size = size.filter(|s| *s != 0.0).unwrap_or(10.0);
                let extra = block.extra_space.get(k - 1).copied().unwrap_or(0.0);
                y -= baseline_skip(size) + extra;
            } else {
                top = asc;
            }
            bottom = -y + desc;
            lines.push(LineLayout { y, x: 0.0, width: x, runs });
        }
        let width = lines.iter().map(|l| l.width).fold(f64::NEG_INFINITY, f64::max);
        let width = if lines.is_empty() { 0.0 } else { width };
        for ln in &mut lines {
            ln.x = if centred { (width - ln.width) / 2.0 } else { 0.0 };
        }
        if let Some(h) = fixed_height {
            top = h;
        }
        if let Some(d) = fixed_depth
            && lines.len() == 1
        {
            bottom = d;
        }
        TextLayout { width, height: top, depth: bottom, lines }
    }

    pub(super) fn text_box(&self, src: &str, fmt: &Fmt) -> TextLayout {
        self.text_layout(&typeset(src, fmt), true, None, None)
    }

    /// A text node whose `anchor` is at (x, y).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn place_text(
        &self,
        layout: TextLayout,
        x: f64,
        y: f64,
        anchor: &str,
        inner: f64,
        color: Option<Rgb>,
        rotation: f64,
        bg: Option<Rgb>,
    ) -> TextPrim {
        let hw = layout.width / 2.0 + inner;
        let hh = (layout.height + layout.depth) / 2.0 + inner;
        let (mut dx, mut dy) = anchor_offset(anchor, hw, hh);
        if rotation != 0.0 {
            let (c, s) = (rotation.to_radians().cos(), rotation.to_radians().sin());
            (dx, dy) = (c * dx - s * dy, s * dx + c * dy);
        }
        TextPrim { layout, cx: x - dx, cy: y - dy, rotation, color, bg, inner }
    }
}
