//! Exports (from `export.py`): the editor rendering as SVG (the core's, glyph outlines), PDF
//! (`svg2pdf`) or PNG (`resvg`: the same pixels on every system, and without a window); the LaTeX
//! result as PDF, or as SVG / PNG through `pdftocairo` like the Python app.

use std::path::Path;
use std::process::Command;

use diagram_core::document::{Document, Kind};
use diagram_core::export::editor_svg;
use diagram_core::style::LabelLanguage;

use crate::latex::{self, Job};

/// What an export makes (kept in the settings as `export/...`).
pub struct Settings {
    /// `png`, `pdf` or `svg`
    pub format: String,
    pub grid: bool,
    pub dpi: u32,
    pub transparent: bool,
}

/// The LaTeX result as PNG needs a program that draws PDFs; without one, the page draws it.
pub const NEEDS_PAGE: &str = "pdftocairo was not found";

fn usvg_tree(svg: &str) -> Result<resvg::usvg::Tree, String> {
    resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).map_err(|e| e.to_string())
}

/// An SVG as PDF; its size in pt stays the page's size.
pub fn svg_to_pdf(svg: &str) -> Result<Vec<u8>, String> {
    let tree =
        svg2pdf::usvg::Tree::from_str(svg, &svg2pdf::usvg::Options::default()).map_err(|e| e.to_string())?;
    // usvg reads pt as 4/3 px; 96 "dpi" turns them back into pt
    svg2pdf::to_pdf(&tree, svg2pdf::ConversionOptions::default(), svg2pdf::PageOptions { dpi: 96.0 })
        .map_err(|e| format!("{e:?}"))
}

/// An SVG as PNG at `dpi`, white or transparent behind it.
pub fn svg_to_png(svg: &str, dpi: u32, transparent: bool) -> Result<Vec<u8>, String> {
    let tree = usvg_tree(svg)?;
    let scale = dpi as f32 / 96.0;
    let size = tree.size();
    let (w, h) =
        ((size.width() * scale).ceil().max(1.0) as u32, (size.height() * scale).ceil().max(1.0) as u32);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)
        .ok_or_else(|| format!("{w} × {h} pixels is too large; please choose a lower resolution"))?;
    if !transparent {
        pixmap.fill(resvg::tiny_skia::Color::WHITE);
    }
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| e.to_string())
}

/// The diagram as the editor draws it.
pub fn editor(text: &str, kind: Kind, language: &str, st: &Settings, path: &Path) -> Result<(), String> {
    let doc = Document::parse(text, kind).map_err(|e| e.to_string())?;
    let lang = if language == "german" { LabelLanguage::German } else { LabelLanguage::English };
    let svg = editor_svg(&doc, lang, st.grid);
    let bytes = match st.format.as_str() {
        "svg" => svg.into_bytes(),
        "pdf" => svg_to_pdf(&svg)?,
        _ => svg_to_png(&svg, st.dpi, st.transparent)?,
    };
    std::fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// The diagram as LaTeX sets it.  `Err(NEEDS_PAGE)`: no pdftocairo for a PNG.
pub fn latex(job: &Job<'_>, st: &Settings, path: &Path) -> Result<(), Vec<String>> {
    let pdf = latex::compile(job)?;
    if st.format == "pdf" {
        return std::fs::write(path, pdf).map_err(|e| vec![format!("cannot write {}: {e}", path.display())]);
    }
    let Some(cairo) = latex::which("pdftocairo") else {
        return Err(vec![if st.format == "svg" {
            "SVG export of the LaTeX result needs 'pdftocairo' (poppler-utils)".into()
        } else {
            NEEDS_PAGE.into()
        }]);
    };
    let dir = tempfile::tempdir().map_err(|e| vec![e.to_string()])?;
    let input = dir.path().join("result.pdf");
    std::fs::write(&input, pdf).map_err(|e| vec![e.to_string()])?;
    let mut cmd = Command::new(cairo);
    if st.format == "svg" {
        cmd.args(["-svg"]).arg(&input).arg(path);
    } else {
        cmd.args(["-png", "-singlefile", "-r", &st.dpi.to_string()]);
        if st.transparent {
            cmd.arg("-transp");
        }
        cmd.arg(&input).arg(dir.path().join("png"));
    }
    let out = cmd.output().map_err(|e| vec![e.to_string()])?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(vec![if msg.is_empty() { "pdftocairo failed".into() } else { msg }]);
    }
    if st.format == "png" {
        std::fs::copy(dir.path().join("png.png"), path).map_err(|e| vec![e.to_string()])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn airline() -> String {
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/test_diagrams/railway.erd"))
            .unwrap()
    }

    fn check(path: &Path, format: &str) {
        let bytes = std::fs::read(path).unwrap();
        assert!(bytes.len() > 1000, "{}", path.display());
        match format {
            "pdf" => assert!(bytes.starts_with(b"%PDF")),
            "png" => assert!(bytes.starts_with(b"\x89PNG")),
            _ => assert!(String::from_utf8_lossy(&bytes[..2000.min(bytes.len())]).contains("<svg")),
        }
    }

    fn png_width(path: &Path) -> u32 {
        let b = std::fs::read(path).unwrap();
        u32::from_be_bytes([b[16], b[17], b[18], b[19]])
    }

    #[test]
    fn editor_rendering() {
        let dir = tempfile::tempdir().unwrap();
        let text = airline();
        for format in ["png", "pdf", "svg"] {
            for grid in [false, true] {
                let p = dir.path().join(format!("e{grid}.{format}"));
                let st = Settings { format: format.into(), grid, dpi: 150, transparent: false };
                editor(&text, Kind::Er, "english", &st, &p).unwrap();
                check(&p, format);
            }
        }
        let w = png_width(&dir.path().join("efalse.png"));
        assert!((1000..1250).contains(&w), "about 18.7 cm at 150 dpi: {w}");
        // the PDF page is as large as the drawing (in pt), as the SVG says
        let pdf =
            String::from_utf8_lossy(&std::fs::read(dir.path().join("efalse.pdf")).unwrap()).into_owned();
        let svg = std::fs::read_to_string(dir.path().join("efalse.svg")).unwrap();
        let width: f64 = svg.split("width=\"").nth(1).unwrap().split("pt").next().unwrap().parse().unwrap();
        let mediabox = pdf.split("/MediaBox [").nth(1).unwrap().split(']').next().unwrap().to_string();
        let mb_w: f64 = mediabox.split_whitespace().nth(2).unwrap().parse().unwrap();
        assert!((mb_w - width).abs() < 1.0, "{mediabox} vs {width}");
    }

    #[test]
    fn latex_result() {
        if latex::which("pdflatex").is_none() || latex::which("pdftocairo").is_none() {
            eprintln!("no pdflatex or pdftocairo: skipped");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let text = airline();
        let job = Job {
            text: &text,
            kind: Kind::Er,
            preamble: diagram_core::export::DEFAULT_PREAMBLE,
            language: "english",
            grid: false,
        };
        for format in ["png", "pdf", "svg"] {
            let p = dir.path().join(format!("l.{format}"));
            let st = Settings { format: format.into(), grid: format == "png", dpi: 100, transparent: true };
            latex(&Job { grid: st.grid, ..job }, &st, &p).unwrap();
            check(&p, format);
        }
        let png = std::fs::read(dir.path().join("l.png")).unwrap();
        assert_eq!(png[25], 6, "RGBA: a transparent background");
    }
}
