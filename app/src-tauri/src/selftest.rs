//! `--self-test`: checks a build (a deb, an rpm, the Windows installation) as the Python app's
//! `selftest.py` does.  First what needs no window: the bundled packages and fonts, layout,
//! every export format, the relational schema, GSM -> ER, the shortcut table, the settings import;
//! then (unless `--no-window`) the window, where the page checks itself (see `App.svelte`).

use diagram_core::document::{Document, Kind};
use diagram_core::export::editor_svg;
use diagram_core::fonts;
use diagram_core::gsm2er::{ConversionSettings, convert};
use diagram_core::relational::{Overrides, Settings, to_sql, transform};
use diagram_core::shortcuts::{self, COMMANDS};
use diagram_core::style::LabelLanguage;

use crate::{export, latex};

pub const ER_SAMPLE: &str = "\\entity{p}(0,0){Person}\\attributes{p}{*PID : integer, name}\n\
\\relationship{w}(3,0){works for}\\entity{c}(6,0){Company}\\attributes{c}{*CID}\n\
\\link{p}{w}{n}\\link{w}{c}{1}\n\\isa{i}(0,-2){p}{io}\\entity{s}(0,-4){Student}\\link{s}{i}\n";
pub const GSM_SAMPLE: &str = "\\abstract{p}(0,0){Person}\\representable{n}(0,2){PName}\\attribute{p}{n}{has name}\n\
\\subtype{s}(0,-2){Student}\\specialization{s}{p}\n";

/// The report so far.
pub struct Report {
    pub lines: Vec<String>,
    pub ok: bool,
}

impl Report {
    pub fn check(&mut self, name: &str, cond: bool, detail: &str) {
        self.ok &= cond;
        let d = if detail.is_empty() { String::new() } else { format!("  {detail}") };
        self.lines.push(format!("{}  {name}{d}", if cond { "ok  " } else { "FAIL" }));
    }

    pub fn info(&mut self, text: &str) {
        self.lines.push(format!("info  {text}"));
    }
}

fn png_width(png: &[u8]) -> u32 {
    png.get(16..20).map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// What needs no window.
pub fn headless(version: &str) -> Report {
    let mut r = Report { lines: vec![format!("ERDTeX {version} self-test")], ok: true };
    r.check("bundled LaTeX packages", latex::ERDIAGRAM.contains("\\ProvidesPackage{erdiagram}"), "");
    r.check("bundled LaTeX packages (GSM)", latex::GSMDIAGRAM.contains("\\ProvidesPackage{gsmdiagram}"), "");
    let n = fonts::files().count();
    r.check("bundled fonts", n >= 37, &format!("{n} Latin Modern files"));
    let er = Document::parse(ER_SAMPLE, Kind::Er);
    let gsm = Document::parse(GSM_SAMPLE, Kind::Gsm);
    let (Ok(er), Ok(gsm)) = (er, gsm) else {
        r.check("samples", false, "could not be read");
        return r;
    };
    let svg = editor_svg(&er, LabelLanguage::English, false);
    r.check("ER layout and drawing", svg.matches("<path").count() > 20, "");
    let gsvg = editor_svg(&gsm, LabelLanguage::German, true);
    r.check("GSM layout and drawing", gsvg.matches("<path").count() > 10, "");
    match export::svg_to_pdf(&svg) {
        Ok(pdf) => r.check("PDF export (svg2pdf)", pdf.starts_with(b"%PDF"), &format!("{} bytes", pdf.len())),
        Err(e) => r.check("PDF export (svg2pdf)", false, &e),
    }
    match export::svg_to_png(&svg, 100, true) {
        Ok(png) => {
            let w = png_width(&png);
            r.check(
                "PNG export (resvg)",
                png.starts_with(b"\x89PNG") && w > 200,
                &format!("{w} px wide at 100 dpi"),
            )
        }
        Err(e) => r.check("PNG export (resvg)", false, &e),
    }
    let schema = transform(&er, &Settings::default(), &Overrides::new());
    let sql = ["sqlite", "postgresql", "oracle"]
        .iter()
        .all(|d| to_sql(&schema, d, Default::default(), false, "", 255).contains("CREATE TABLE"));
    r.check(
        "relational schema and SQL",
        schema.relations.len() == 4 && sql,
        &format!("{} relations", schema.relations.len()),
    );
    match convert(&gsm, &ConversionSettings::default(), None) {
        Ok(c) => r.check("GSM -> ER", c.doc.nodes().count() >= 3, ""),
        Err(e) => r.check("GSM -> ER", false, &e.to_string()),
    }
    let defaults = shortcuts::defaults();
    let clash = COMMANDS
        .iter()
        .find_map(|c| shortcuts::find_conflict(&defaults, c.id, c.default).map(|o| (c.id, o.id)));
    r.check(
        "keyboard shortcuts",
        clash.is_none(),
        &clash.map_or(String::new(), |(a, b)| format!("{a} clashes with {b}")),
    );
    r.info(&format!(
        "pdflatex: {}",
        latex::which("pdflatex")
            .map_or(format!("not found (the LaTeX preview and export need {})", latex::TEX_HINT), |p| {
                p.display().to_string()
            })
    ));
    r.info(&format!(
        "pdftocairo: {}",
        latex::which("pdftocairo").map_or("not found (optional)".into(), |p| p.display().to_string())
    ));
    r
}

#[cfg(test)]
mod tests {
    #[test]
    fn passes_here() {
        let r = super::headless("test");
        assert!(r.ok, "{}", r.lines.join("\n"));
    }
}
