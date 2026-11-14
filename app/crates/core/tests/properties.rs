//! Properties that must hold for any input: the core never panics, and what it reads it
//! writes back unchanged.

use diagram_core::colors::parse_color;
use diagram_core::document::{Document, Kind};
use diagram_core::fonts::Metrics;
use diagram_core::geometry as g;
use diagram_core::latex_text::{Fmt, font_switches, parse_length_pt, plain_text, typeset};
use diagram_core::layout::{DummyEngine, FontEngine, Layout};
use diagram_core::model::{AttrItem, Coord, Options};
use diagram_core::style::LabelLanguage;
use diagram_core::svg::{TextMode, render};
use proptest::prelude::*;

/// Text made of the characters that matter to the syntax, plus some others.
fn diagram_text() -> impl Strategy<Value = String> {
    let pieces = prop::sample::select(vec![
        "\\entity",
        "\\relationship",
        "\\link",
        "\\attributes",
        "\\isa",
        "\\note",
        "\\subdomain",
        "\\setup",
        "\\abstract",
        "\\attribute",
        "\\draw",
        "\\begin",
        "\\point",
        "\\grid",
        "{",
        "}",
        "[",
        "]",
        "(",
        ")",
        "%",
        ",",
        "=",
        "*",
        "+",
        "/",
        "@",
        ":",
        ";",
        "$",
        "^",
        "_",
        "~",
        " ",
        "\n",
        "\n\n",
        "\t",
        "\\\\",
        "\\{",
        "\\}",
        "a",
        "x1",
        "0",
        "1.5",
        "-2",
        "ä",
        "€",
        "\u{2003}",
        "\r\n",
        "\\textbf",
        "\\color",
        "--",
        "``",
    ]);
    prop::collection::vec(pieces, 0..60).prop_map(|v| v.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(3000))]

    #[test]
    fn parsing_never_panics_and_writes_back_what_it_read(text in diagram_text()) {
        for kind in [Kind::Er, Kind::Gsm] {
            if let Ok(doc) = Document::parse(&text, kind) {
                prop_assert_eq!(doc.to_text(), text.clone());
                prop_assert!(!doc.modified());
            }
        }
    }

    #[test]
    fn layout_and_svg_never_panic(text in diagram_text()) {
        for kind in [Kind::Er, Kind::Gsm] {
            if let Ok(doc) = Document::parse(&text, kind) {
                for engine in [&DummyEngine as &dyn diagram_core::layout::TextEngine, &FontEngine { metrics: Metrics::DesignSizes }] {
                    if let Ok(lay) = Layout::new(&doc, engine, LabelLanguage::German).run() {
                        let _ = render(&lay, TextMode::Outlines, Metrics::DesignSizes, 4.0);
                        let _ = render(&lay, TextMode::Text, Metrics::DesignSizes, 4.0);
                    }
                }
            }
        }
    }

    #[test]
    fn rules_schemas_and_conversion_never_panic(text in diagram_text()) {
        use diagram_core::relational::{self as rel, Marks, Quote, Settings};
        for kind in [Kind::Er, Kind::Gsm] {
            if let Ok(doc) = Document::parse(&text, kind) {
                let problems = diagram_core::rules::check(&doc);
                for p in problems.iter().filter(|p| p.fix.is_some()).take(3) {
                    let mut copy = doc.clone();
                    if let Some(f) = &p.fix {
                        f.apply(&mut copy, p.element);
                    }
                }
                for rules in [None, Some("db1".to_string())] {
                    let s = rel::transform(&doc, &Settings { rules, ..Settings::default() }, &Default::default());
                    let _ = rel::to_text(&s, Marks::Tags, true, "");
                    let _ = rel::to_latex(&s, "");
                    for d in rel::DIALECTS {
                        let _ = rel::to_sql(&s, d, Quote::All, true, "", 255);
                    }
                }
                let _ = diagram_core::gsm2er::convert(&doc, &Default::default(), None);
                let _ = diagram_core::cardinality::plan_diagram(&doc, "minmax");
            }
        }
    }

    #[test]
    fn editing_never_panics_and_undo_brings_the_file_back(
        text in diagram_text(),
        steps in proptest::collection::vec((0u8..14, -6.0f64..14.0, -8.0f64..6.0, 0u8..8), 1..40),
    ) {
        use diagram_core::editor::{Editor, Mods, edge_tools, node_tools, special_tools};
        use diagram_core::geometry::CM;
        for kind in [Kind::Er, Kind::Gsm] {
            let Ok(doc) = Document::parse(&text, kind) else { continue };
            let original = doc.to_text();
            let mut ed = Editor::new(diagram_core::session::Session::new(doc));
            let mut tools: Vec<&str> = vec!["select"];
            tools.extend(node_tools(kind).iter().map(|t| t.0));
            tools.extend(edge_tools(kind).iter().map(|t| t.0));
            tools.extend(special_tools(kind).iter().copied());
            let mut clip = String::new();
            for &(what, x, y, k) in &steps {
                let (x, y) = (x * CM, y * CM);
                let mods = Mods { shift: k & 1 != 0, ctrl: false, alt: k & 2 != 0 };
                match what {
                    0 => ed.set_tool(tools[k as usize % tools.len()]),
                    1 | 2 => ed.press(x, y, mods),
                    3 | 4 => ed.move_to(x, y, mods),
                    5 => ed.release(x, y, mods),
                    6 => { ed.double_click(x, y); }
                    7 => { ed.key(["cancel", "delete", "nudge left", "nudge up"][k as usize % 4], mods); }
                    8 => clip = ed.copy(),
                    9 => ed.paste(&clip, Some((x / CM, y / CM)), None),
                    10 => ed.duplicate(),
                    11 => { ed.undo(); }
                    12 => { ed.context(x, y); ed.straighten(); }
                    _ => { let _ = ed.overlay(); ed.cursor(x, y); }
                }
            }
            ed.release(0.0, 0.0, Mods::default());
            while ed.undo() {}
            prop_assert_eq!(ed.session().text(), original);
        }
    }

    #[test]
    fn canonical_output_is_never_a_panic(text in diagram_text()) {
        if let Ok(mut doc) = Document::parse(&text, Kind::Er) {
            for e in &mut doc.elements {
                e.touch();
            }
            let _ = doc.to_text();
        }
    }

    #[test]
    fn labels_and_values_never_panic(s in "\\PC{0,40}", t in diagram_text()) {
        for x in [s.as_str(), t.as_str()] {
            let _ = typeset(x, &Fmt::default());
            let _ = plain_text(x);
            let _ = font_switches(x, &Fmt::default());
            let _ = parse_length_pt(x, 10.0);
            let _ = parse_color(Some(x), None);
            let _ = AttrItem::parse(x).map(|i| i.to_tex());
            let _ = Options::parse(Some(x)).to_tex();
            let _ = Coord::parse(x).to_tex();
        }
    }

    #[test]
    fn shapes_never_panic(v in prop::collection::vec(any::<f64>(), 9), name in "\\PC{0,12}") {
        let shapes = [
            g::rect_shape(v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8]),
            g::diamond_shape(v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8]),
            g::hexagon_shape(v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8]),
            g::circle_shape(v[0], v[1], v[2], v[3], v[4], v[5], v[6]),
            g::ellipse_shape(v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]),
            g::triangle_shape(v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8]),
        ];
        for s in &shapes {
            let _ = (s.outline(), s.bbox(), s.anchor(&name), s.border_toward(v[2], v[3]), s.contains(v[4], v[5]));
        }
    }
}

// Editing real diagrams lays them out after every step: fewer cases by default (debug builds are
// slow); `PROPTEST_CASES=4000 cargo test --release --test properties editing` for a thorough run.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(std::env::var("PROPTEST_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(48)))]

    #[test]
    fn editing_real_diagrams_never_panics(
        which in 0usize..4,
        steps in proptest::collection::vec((0u8..14, -6.0f64..20.0, -12.0f64..10.0, 0u8..8), 1..60),
    ) {
        use diagram_core::editor::{Editor, Mods, edge_tools, node_tools, special_tools};
        use diagram_core::geometry::CM;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let files = ["app/test_diagrams/railway.erd", "app/test_diagrams/ensemble.gsmd",
                     "app/test_diagrams/features.erd", "app/test_diagrams/imkerei.erd"];
        let path = root.join(files[which]);
        let text = std::fs::read_to_string(&path).unwrap();
        let kind = if files[which].ends_with(".gsmd") { Kind::Gsm } else { Kind::Er };
        let doc = Document::parse(&text, kind).unwrap();
        let original = doc.to_text();
        let mut ed = Editor::new(diagram_core::session::Session::new(doc));
        let mut tools: Vec<&str> = vec!["select", "select", "select"];
        tools.extend(node_tools(kind).iter().map(|t| t.0));
        tools.extend(edge_tools(kind).iter().map(|t| t.0));
        tools.extend(special_tools(kind).iter().copied());
        let mut clip = String::new();
        for &(what, x, y, k) in &steps {
            let (x, y) = (x * CM, y * CM);
            let mods = Mods { shift: k & 1 != 0, ctrl: false, alt: k & 2 != 0 };
            match what {
                0 => ed.set_tool(tools[k as usize % tools.len()]),
                1 | 2 => ed.press(x, y, mods),
                3 | 4 => ed.move_to(x, y, mods),
                5 => ed.release(x, y, mods),
                6 => { ed.double_click(x, y); }
                7 => { ed.key(["cancel", "delete", "nudge left", "nudge up"][k as usize % 4], mods); }
                8 => clip = ed.copy(),
                9 => ed.paste(&clip, Some((x / CM, y / CM)), None),
                10 => ed.duplicate(),
                11 => { if k < 4 { ed.undo(); } else { ed.redo(); } }
                12 => { ed.context(x, y); if k < 4 { ed.straighten(); } else { ed.add_attribute_list(); } }
                _ => { let _ = ed.overlay(); ed.cursor(x, y); }
            }
        }
        ed.release(0.0, 0.0, Mods::default());
        while ed.undo() {}
        prop_assert_eq!(ed.session().text(), original);
    }

    #[test]
    fn every_field_of_every_element_takes_odd_values(
        which in 0usize..4,
        picks in proptest::collection::vec((0usize..400, 0usize..64, 0u8..10), 1..40),
    ) {
        use diagram_core::editor::Editor;
        use diagram_core::panel::{self, Field};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let files = ["app/test_diagrams/railway.erd", "app/test_diagrams/ensemble.gsmd",
                     "app/test_diagrams/features.erd", "app/test_diagrams/imkerei.erd"];
        let text = std::fs::read_to_string(root.join(files[which])).unwrap();
        let kind = if files[which].ends_with(".gsmd") { Kind::Gsm } else { Kind::Er };
        let doc = Document::parse(&text, kind).unwrap();
        let original = doc.to_text();
        let mut ed = Editor::new(diagram_core::session::Session::new(doc));
        for &(element, row, value) in &picks {
            let keys: Vec<_> = ed.doc().elements.iter().map(|e| e.key).collect();
            // nothing selected (the diagram's form) now and then
            ed.set_selection(if element % 9 == 0 { vec![] } else { vec![keys[element % keys.len()]] });
            let form = panel::form(&mut ed);
            if form.rows.is_empty() {
                continue;
            }
            let r = &form.rows[row % form.rows.len()];
            let first = match &r.field {
                Field::Choice { items, .. } => items.get(value as usize % items.len().max(1)).map(|i| i.value.clone()),
                Field::Color { swatches, .. } => swatches.get(value as usize % swatches.len().max(1)).map(|s| s.value.clone()),
                Field::Cardinality(c) => c.choices.first().map(|i| i.value.clone()),
                _ => None,
            };
            let odd = ["", "true", "false", "-3.5", "x{", "a, b=c", "(1,*)", "\\", "  ", "9999999999999999999999"];
            let v = first.unwrap_or_else(|| odd[value as usize % odd.len()].to_string());
            let id = match &r.field {
                Field::Position { .. } => format!("{}.{}", r.id, if value % 2 == 0 { "x" } else { "y" }),
                Field::Attributes(rows) => {
                    let what = ["name", "key", "unique", "type", "kind", "formula", "add", "remove", "up", "down", "indent", "outdent", "next"];
                    format!("attr.{}.{}", what[value as usize % what.len()], row % (rows.len() + 1))
                }
                Field::Cardinality(_) if value % 3 == 0 => "card.minmax".into(),
                Field::Cardinality(_) if value % 3 == 1 => "card.deviate".into(),
                _ => r.id.clone(),
            };
            let _ = panel::set(&mut ed, &id, &v, value % 2 == 0);
        }
        while ed.undo() {}
        prop_assert_eq!(ed.session().text(), original);
    }
}
