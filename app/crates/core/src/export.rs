//! What the LaTeX preview and the exports need from the core (from `preview.py` and
//! `export.py`): the LaTeX job's wrapper, reading pdflatex's errors, the `\input` line, and the
//! editor rendering as a self-contained SVG (glyph outlines, sizes in PostScript points).

use crate::document::{Document, Kind};
use crate::fonts::Metrics;
use crate::geometry::CM;
use crate::layout::{FontEngine, Gfx, Layout};
use crate::model::{Body, Coord, Grid};
use crate::style::LabelLanguage;
use crate::svg::{self, TextMode};

/// The preamble the preview starts with (Preferences → Output can change it).
pub const DEFAULT_PREAMBLE: &str = "\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}";

/// PostScript points (PDF, SVG) per TeX point.
pub const BP_PER_PT: f64 = 72.0 / 72.27;

/// The name of the diagram file in a LaTeX job.
pub fn job_file(kind: Kind) -> &'static str {
    if kind == Kind::Er { "diagram.erd" } else { "diagram.gsmd" }
}

/// The document that compiles a diagram file on its own.
pub fn wrapper(kind: Kind, diagram_file: &str, preamble: &str, language: &str, grid: bool) -> String {
    let cmd = if kind == Kind::Er { "\\inputerdiagram" } else { "\\inputgsmdiagram" };
    let opts = if grid { "[grid]" } else { "" };
    format!(
        "\\documentclass[border=3pt]{{standalone}}\n{preamble}\n\\usepackage{{erdiagram}}\n\\usepackage{{gsmdiagram}}\n\
         \\ersetup{{label language={language}}}\n\\begin{{document}}\n{cmd}{opts}{{{diagram_file}}}\n\\end{{document}}\n"
    )
}

/// `^(.*?):(\d+): (.*)$`
fn file_line_error(line: &str) -> Option<(&str, &str, &str)> {
    let mut search = 0;
    while let Some(i) = line.get(search..)?.find(':') {
        let at = search + i;
        let (file, rest) = (line.get(..at)?, line.get(at + 1..)?);
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && rest.get(digits..)?.starts_with(": ") {
            return Some((file, rest.get(..digits)?, rest.get(digits + 2..)?));
        }
        search = at + 1;
    }
    None
}

/// The errors of a `-file-line-error` log: `line N: message` for the diagram file, `file:N:
/// message` for others, else the first `! message`.
pub fn parse_errors(log: &str, diagram_name: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in log.lines() {
        if let Some((file, n, msg)) = file_line_error(line)
            && ([".tex", ".erd", ".gsmd", ".sty"].iter().any(|e| file.ends_with(e)) || file.contains('/'))
        {
            let base = file.rsplit('/').next().unwrap_or(file);
            let where_ =
                if file.ends_with(diagram_name) { format!("line {n}") } else { format!("{base}:{n}") };
            let m = format!("{where_}: {msg}");
            if m.contains("==> Fatal error") {
                continue;
            }
            if !out.contains(&m) {
                out.push(m);
            }
        } else if let Some(m) = line.strip_prefix("! ")
            && out.is_empty()
        {
            out.push(m.to_string());
        }
    }
    out
}

/// The line that includes a diagram in a LaTeX document (File → Copy \input Line).
pub fn input_line(kind: Kind, file_name: Option<&str>) -> String {
    let cmd = if kind == Kind::Er { "\\inputerdiagram" } else { "\\inputgsmdiagram" };
    let name = file_name.map_or_else(|| format!("diagram{}", kind.extension()), str::to_string);
    format!("{cmd}[max width=\\linewidth]{{{name}}}")
}

fn bbox_of(gfx: &[Gfx]) -> Option<(f64, f64, f64, f64)> {
    gfx.iter().filter_map(svg::bbox).reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
}

/// What the editor draws, with the package's labelled 1 cm grid if `grid`; the extent with
/// 4 pt around it.
fn editor_gfx(doc: &Document, language: LabelLanguage, grid: bool) -> (Vec<Gfx>, (f64, f64, f64, f64)) {
    let engine = FontEngine { metrics: Metrics::DesignSizes };
    let mut gfx = Layout::new(doc, &engine, language).run().map(|l| l.gfx).unwrap_or_default();
    if grid && let Some((x0, y0, x1, y1)) = bbox_of(&gfx) {
        let (x0, y0, x1, y1) = (x0 - 4.0, y0 - 4.0, x1 + 4.0, y1 + 4.0);
        let mut with_grid = doc.clone();
        let key = with_grid.add(
            Body::Grid(Grid {
                a: Coord::numeric((x0 / CM).floor(), (y0 / CM).floor()),
                b: Coord::numeric((x1 / CM).ceil(), (y1 / CM).ceil()),
            }),
            None,
        );
        if let Ok(l) = Layout::new(&with_grid, &engine, language).run()
            && let Some(g) = l.gfx.into_iter().find(|g| g.key == Some(key))
        {
            gfx.push(g);
        }
    }
    let (x0, y0, x1, y1) = bbox_of(&gfx).unwrap_or((0.0, 0.0, 1.0, 1.0));
    (gfx, (x0 - 4.0, y0 - 4.0, x1 + 4.0, y1 + 4.0))
}

/// The diagram as the editor draws it, as a self-contained SVG: text as glyph outlines, the
/// size in PostScript points, 4 pt around it.
pub fn editor_svg(doc: &Document, language: LabelLanguage, grid: bool) -> String {
    let (gfx, (x0, y0, x1, y1)) = editor_gfx(doc, language, grid);
    let d = svg::draw(&gfx, &svg::Options::export(TextMode::Outlines));
    let n = |v: f64| crate::text::fmt_g(crate::text::py_round(v, 3));
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{}pt\" height=\"{}pt\">\
         <title>ERDTeX</title><defs>{}</defs><g transform=\"scale(1 -1)\">{}</g></svg>\n",
        n(x0),
        n(-y1),
        n(x1 - x0),
        n(y1 - y0),
        n((x1 - x0) * BP_PER_PT),
        n((y1 - y0) * BP_PER_PT),
        d.defs,
        d.body
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_of_a_log() {
        let log = "This is pdfTeX\n./diagram.erd:1: Undefined control sequence.\n\
                   ./diagram.erd:1: Undefined control sequence.\n/usr/share/x/erdiagram.sty:12: oops\n\
                   ! ==> Fatal error occurred, no output PDF file produced!\n";
        assert_eq!(
            parse_errors(log, "diagram.erd"),
            ["line 1: Undefined control sequence.", "erdiagram.sty:12: oops"]
        );
        assert_eq!(parse_errors("! Emergency stop.\n! Other\n", "diagram.erd"), ["Emergency stop."]);
        assert_eq!(parse_errors("a: b: 3: no\nnote: 12 apples\n", "diagram.erd"), Vec::<String>::new());
    }

    #[test]
    fn wrapper_and_input_line() {
        let w = wrapper(Kind::Gsm, "diagram.gsmd", DEFAULT_PREAMBLE, "german", true);
        assert!(w.contains("\\inputgsmdiagram[grid]{diagram.gsmd}") && w.contains("label language=german"));
        assert_eq!(input_line(Kind::Er, Some("a.erd")), "\\inputerdiagram[max width=\\linewidth]{a.erd}");
        assert_eq!(input_line(Kind::Gsm, None), "\\inputgsmdiagram[max width=\\linewidth]{diagram.gsmd}");
    }

    #[test]
    fn the_editor_rendering() {
        let doc = Document::parse("\\entity{a}(0,0){Alpha}\\entity{b}(4,0){B}", Kind::Er).unwrap();
        let plain = editor_svg(&doc, LabelLanguage::English, false);
        let grid = editor_svg(&doc, LabelLanguage::English, true);
        assert!(plain.starts_with("<svg") && !plain.contains("<text"), "text as outlines");
        assert!(grid.len() > plain.len() + 1000, "the grid with its labels");
    }
}
