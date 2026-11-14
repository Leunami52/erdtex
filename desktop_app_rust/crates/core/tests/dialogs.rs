//! The Relational Schema / SQL window (the Python app's tests/test_relational_window.py) and
//! what the GSM -> ER dialog makes (tests/test_gsm2er.py), on the core's data.

use std::path::Path;

use diagram_core::document::{Document, Kind};
use diagram_core::editor::Editor;
use diagram_core::gsm2er::{ConversionSettings, convert};
use diagram_core::newdefaults::{apply_defaults_to_generated, setting_name};
use diagram_core::rmwindow::{self, Options};
use diagram_core::session::Session;

fn example(group: &str, key: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/rm_examples.json");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    v[group][key].as_str().unwrap().to_string()
}

fn editor(src: &str) -> Editor {
    Editor::new(Session::new(Document::parse(src, Kind::Er).unwrap()))
}

#[test]
fn an_old_files_rule_set_keeps_its_results() {
    // versions up to 3.5 offered a rule set; `rm rules=db1` is still read, though nothing offers it
    let opts = Options::default();
    let gen_cd = example("EX", "gen_cd");
    let schema = |src: &str| {
        let ed = editor(src);
        rmwindow::view(ed.doc(), &opts).output.split("Foreign keys").next().unwrap().to_lowercase()
    };
    assert!(schema(&gen_cd).contains("category"), "a disjoint generalisation: categorised");
    let old = format!("\\setup{{rm rules=db1}}\n{gen_cd}");
    assert!(!schema(&old).contains("category"), "rm rules=db1: not categorised, as before");
    let boss = "\\entity{e}(0,0){Employee}\\attributes{e}{*SSN}\\relationship{s}(5,0){supervises}\
                \\link[role=boss]{e}{s}{c}\\link[role=staff]{e}{s}{cn}";
    assert!(
        schema(boss).contains("employee_supervises_employee : {("),
        "a recursive relationship: an own relation"
    );
    let old = format!("\\setup{{rm rules=db1}}{boss}");
    assert!(
        !schema(&old).contains("employee_supervises_employee : {("),
        "rm rules=db1: a foreign key, as before"
    );
    // nothing offers it any more: not the diagram's Element pane, not Preferences
    let mut ed = editor(&gen_cd);
    ed.set_selection(vec![]);
    assert!(
        diagram_core::panel::form(&mut ed).field("setup:rm rules").is_none(),
        "the Element pane offers no rule set"
    );
    assert!(!diagram_core::newdefaults::TOGGLES.iter().any(|t| t.key == "rm rules"), "nor Preferences");
}

#[test]
fn choosing_an_option_is_stored_and_shows_its_result() {
    let opts = Options::default();
    let mut ed = editor(&example("DE", "begleiten_ternary"));
    let v = rmwindow::view(ed.doc(), &opts);
    let options: Vec<_> = v.decisions.iter().flat_map(|d| d.options.iter()).collect();
    assert_eq!(options.len(), 4);
    // a candidate key, in both rule sets (REL-13)
    assert_eq!(v.decisions[0].current, 2);
    assert!(
        options.iter().filter(|o| o.valid).all(|o| o.result.contains(" : {(")),
        "every option shows its relations"
    );
    assert!(rmwindow::choose(&mut ed, "b", 3, &opts));
    assert_eq!(ed.doc().by_id("b").unwrap().as_node().unwrap().opts.get("rm"), Some("table"));
    assert_eq!(rmwindow::view(ed.doc(), &opts).decisions[0].current, 3);
    assert_eq!(ed.session().undo_label(), Some("Relational mapping"));
    ed.undo();
    assert_eq!(ed.doc().by_id("b").unwrap().as_node().unwrap().opts.get("rm"), None);
}

#[test]
fn minor_alternatives_are_marked_as_such() {
    let opts = Options::default();
    let mut ed = editor(&example("EX", "hosts_1m"));
    let v = rmwindow::view(ed.doc(), &opts);
    assert_eq!(v.decisions.iter().filter(|d| d.minor).count(), 1);
    assert!(v.summary().ends_with("0 decision(s)"));
    let id = v.decisions[0].element_id.clone();
    rmwindow::choose(&mut ed, &id, 1, &opts);
    let v = rmwindow::view(ed.doc(), &opts);
    assert!(v.decisions[0].current != v.decisions[0].default, "in use: the window keeps it open");
    assert!(v.output.contains("hosts"));
}

#[test]
fn unenforced_bounds_reach_the_problems_tab() {
    let v = rmwindow::view(
        &Document::parse(&example("DE", "mosaik_minmax"), Kind::Er).unwrap(),
        &Options::default(),
    );
    assert!(
        v.problems.iter().any(|p| p.text.contains("Not enforced") && p.text.contains("at least 4")),
        "{:?}",
        v.problems
    );
}

#[test]
fn data_types_guessed_stored_and_set() {
    let mut ed =
        editor(r"\entity{p}(0,0){Person}\attributes{p}{*PID, name : varchar(40), address{street, town}}");
    let rows = rmwindow::view(ed.doc(), &Options::default()).types;
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["PID", "name", "street", "town"], "a composite attribute is stored as its parts");
    assert!(rows[0].guessed && !rows[1].guessed && rows[1].r#type == "varchar(40)");
    assert!(rmwindow::set_type(&mut ed, 2, "char(30)"));
    assert!(ed.session().text().contains("street : char(30)"), "{}", ed.session().text());
    assert!(rmwindow::store_guesses(&mut ed));
    assert!(rmwindow::view(ed.doc(), &Options::default()).types.iter().all(|r| !r.guessed));
    assert_eq!(ed.session().undo_label(), Some("Store data types"));
}

#[test]
fn formats_and_options() {
    let doc = Document::parse(&example("EX", "hosts_1m"), Kind::Er).unwrap();
    for (format, want) in
        [("text", "Foreign keys"), ("latex", "\\"), ("sqlite", "CREATE TABLE"), ("oracle", "CREATE TABLE")]
    {
        let v = rmwindow::view(&doc, &Options { format: format.into(), ..Options::default() });
        assert!(v.output.contains(want), "{format}");
    }
    let v = rmwindow::view(&doc, &Options { format: "sqlite".into(), drop: true, ..Options::default() });
    assert!(v.output.contains("DROP TABLE"));
    let gsm = Document::parse(r"\abstract{a}(0,0){A}", Kind::Gsm).unwrap();
    assert!(!rmwindow::view(&gsm, &Options::default()).er);
}

#[test]
fn the_converted_diagram_gets_the_defaults_and_the_gsm_one_stays() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../desktop_app_rust/test_diagrams/ensemble.gsmd");
    let text = std::fs::read_to_string(path).unwrap();
    let gsm = Document::parse(&text, Kind::Gsm).unwrap();
    let before = gsm.to_text();
    let mut c = convert(&gsm, &ConversionSettings::default(), Some("globalaspects.gsmd")).unwrap();
    let kept = [
        (setting_name(Kind::Er, "isa shape"), "diamond"),
        (setting_name(Kind::Er, "card notation"), "minmax"),
    ];
    let get = |name: &str| kept.iter().find(|(k, _)| k == name).map(|(_, v)| v.to_string());
    c.report.extend(apply_defaults_to_generated(&mut c.doc, &get));
    let er = c.doc.to_text();
    assert!(er.contains("isa shape=diamond") && er.contains("card notation=minmax"), "{er}");
    assert_eq!(gsm.to_text(), before);
    // what the preview shows is what the new tab gets
    assert_eq!(Document::parse(&er, Kind::Er).unwrap().to_text(), er);
}
