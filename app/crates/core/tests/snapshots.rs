//! The program's results on every test diagram and sample, kept as snapshots (`tests/snapshots/`): the
//! problems, the relational schema and its SQL (ER) or the ER diagram made from it (GSM), and where
//! each element is drawn. A change in any of them shows up as a failing test with the difference;
//! `cargo insta review` (or `INSTA_UPDATE=always`) accepts it once it is meant.

use std::path::Path;

use diagram_core::document::{Document, Kind};
use diagram_core::fonts::Metrics;
use diagram_core::gsm2er::{ConversionSettings, convert};
use diagram_core::layout::{FontEngine, layout_document};
use diagram_core::relational::{Marks, Overrides, Quote, Settings, render, transform};
use diagram_core::rules::check;
use diagram_core::style::LabelLanguage;
use diagram_core::svg::bbox;

/// (name, text, kind): the files in `test_diagrams/` and the examples in `tests/data/rm_examples.json`.
fn diagrams() -> Vec<(String, String, Kind)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    let mut files: Vec<_> = ["../../test_diagrams", "../../samples"]
        .iter()
        .flat_map(|d| std::fs::read_dir(root.join(d)).unwrap().map(|e| e.unwrap().path()))
        .collect();
    files.sort();
    for f in files {
        let kind = match f.extension().and_then(|e| e.to_str()) {
            Some("erd") => Kind::Er,
            Some("gsmd") => Kind::Gsm,
            _ => continue,
        };
        let name = f.file_name().unwrap().to_string_lossy().replace('.', "_");
        out.push((name, std::fs::read_to_string(&f).unwrap(), kind));
    }
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("tests/data/rm_examples.json")).unwrap())
            .unwrap();
    for (group, items) in v.as_object().unwrap() {
        for (key, text) in items.as_object().unwrap() {
            out.push((format!("example_{group}_{key}"), text.as_str().unwrap().to_string(), Kind::Er));
        }
    }
    out
}

fn report(text: &str, kind: Kind) -> String {
    let doc = Document::parse(text, kind).unwrap();
    let mut out = vec!["== problems".to_string()];
    for p in check(&doc) {
        let id = doc.get(p.element).and_then(|e| e.as_node()).map_or(String::new(), |n| n.id.clone());
        out.push(format!("{:?} {} [{id}] {}", p.severity, p.rule, p.message));
    }
    match kind {
        Kind::Er => {
            let schema = transform(&doc, &Settings::default(), &Overrides::new());
            out.push("== relational schema".into());
            out.push(render(&schema, "text", "", Marks::Underscore, true, Quote::Needed, false));
            out.push("== SQL (PostgreSQL)".into());
            out.push(render(&schema, "postgresql", "", Marks::Underscore, true, Quote::Needed, true));
            out.push("== SQL (SQLite)".into());
            out.push(render(&schema, "sqlite", "", Marks::Underscore, true, Quote::Needed, true));
        }
        Kind::Gsm => {
            let c = convert(&doc, &ConversionSettings::default(), None).unwrap();
            out.push("== questions".into());
            out.extend(
                c.choices.iter().map(|q| format!("{} ({}): {} -> {}", q.key, q.rule, q.title, q.current)),
            );
            out.push("== report".into());
            out.extend(c.report.iter().cloned());
            out.push("== ER diagram".into());
            out.push(c.doc.to_text());
        }
    }
    out.push("== layout (x0 y0 x1 y1 in pt, rounded)".into());
    let engine = FontEngine { metrics: Metrics::DesignSizes };
    match layout_document(&doc, &engine, LabelLanguage::English) {
        Ok(l) => {
            for g in &l.gfx {
                let id = g
                    .key
                    .and_then(|k| doc.get(k))
                    .and_then(|e| e.as_node())
                    .map_or("-".to_string(), |n| n.id.clone());
                if let Some((x0, y0, x1, y1)) = bbox(g) {
                    out.push(format!("{id} {x0:.1} {y0:.1} {x1:.1} {y1:.1}"));
                }
            }
        }
        Err(e) => out.push(format!("error: {e}")),
    }
    out.join("\n")
}

#[test]
fn every_test_diagram_gives_what_it_gave() {
    let all = diagrams();
    assert!(all.len() > 30, "{} test diagrams", all.len());
    for (name, text, kind) in all {
        insta::assert_snapshot!(name, report(&text, kind));
    }
}
