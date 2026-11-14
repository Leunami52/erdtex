//! The sample diagrams (`samples/`, File → Open Example) show the program at its best: no problem of
//! any kind, nothing overlapping, no line through an element or across another, and they convert.

use std::path::Path;

use diagram_core::arrange::figures;
use diagram_core::document::{Document, Kind};
use diagram_core::gsm2er::{ConversionSettings, convert};
use diagram_core::relational::{Overrides, Settings, transform};
use diagram_core::rules::check;
use diagram_core::style::LabelLanguage;

fn sample(name: &str, kind: Kind) -> Document {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples").join(name);
    Document::parse(&std::fs::read_to_string(path).unwrap(), kind).unwrap()
}

fn clean(doc: &Document, name: &str) {
    let problems: Vec<String> = check(doc).into_iter().map(|p| format!("{} {}", p.rule, p.message)).collect();
    assert!(problems.is_empty(), "{name}: {problems:?}");
    let f = figures(doc, LabelLanguage::English, None);
    assert_eq!(
        (f.overlaps, f.through, f.close, f.crossings),
        (0, 0, 0, 0),
        "{name}: overlaps, through, close, crossings"
    );
}

#[test]
fn the_er_sample_is_clean_and_converts() {
    let doc = sample("festival.erd", Kind::Er);
    clean(&doc, "festival.erd");
    let s = transform(&doc, &Settings::default(), &Overrides::new());
    assert!(s.issues.is_empty(), "{:?}", s.issues);
    assert!(s.relations().count() >= 12, "{} relations", s.relations().count());
}

#[test]
fn the_gsm_sample_is_clean_and_converts() {
    let doc = sample("festival.gsmd", Kind::Gsm);
    clean(&doc, "festival.gsmd");
    let c = convert(&doc, &ConversionSettings::default(), None).unwrap();
    assert!(c.choices.len() >= 3, "it asks a few questions ({})", c.choices.len());
    assert!(
        check(&c.doc).iter().all(|p| p.severity != diagram_core::rules::Severity::Error),
        "the ER diagram it gives has no errors"
    );
}
