//! The properties and Problems panels: the Python app's panel tests (test_attributes,
//! test_cardinality, test_db1_concepts, test_generalisation, test_settings, test_editor,
//! test_tabs, test_focus), on the panel's data instead of its widgets.

use diagram_core::document::{Document, Kind};
use diagram_core::editor::Editor;
use diagram_core::model::{Body, ElementKey};
use diagram_core::panel::{self, Field, Form, Outcome};
use diagram_core::problems;
use diagram_core::rules::{Severity, check};
use diagram_core::session::Session;

fn editor(src: &str, kind: Kind) -> Editor {
    Editor::new(Session::new(Document::parse(src, kind).unwrap()))
}

fn select(ed: &mut Editor, id: &str) -> Form {
    let key = ed.doc().by_id(id).unwrap().key;
    ed.set_selection(vec![key]);
    panel::form(ed)
}

fn select_key(ed: &mut Editor, key: ElementKey) -> Form {
    ed.set_selection(vec![key]);
    panel::form(ed)
}

fn nothing(ed: &mut Editor) -> Form {
    ed.set_selection(vec![]);
    panel::form(ed)
}

fn set(ed: &mut Editor, id: &str, value: &str) -> Outcome {
    panel::set(ed, id, value, true)
}

fn text(ed: &Editor) -> String {
    ed.session().text()
}

/// The value of a drop-down, and its entries (value, text, tooltip).
fn choice(f: &Form, id: &str) -> (String, Vec<(String, String, String)>) {
    match f.field(id) {
        Some(Field::Choice { items, current, .. }) => (
            current.clone(),
            items.iter().map(|i| (i.value.clone(), i.text.clone(), i.tip.clone())).collect(),
        ),
        other => panic!("{id}: {other:?}"),
    }
}

fn checked(f: &Form, id: &str) -> (bool, bool) {
    match f.field(id) {
        Some(Field::Check { value, enabled, .. }) => (*value, *enabled),
        other => panic!("{id}: {other:?}"),
    }
}

fn has_row(f: &Form, label: &str) -> bool {
    f.rows.iter().any(|r| r.label == label)
}

// -- the diagram ------------------------------------------------------------------------------

const SAMPLE: &str = r"\entity{p}(0,0){Person}\attributes{p}{*PID, name}
\relationship[dependency=Id]{r}(4,0){has}\link{p}{r}{1}
\isa{i}(0,-2.2){p}{cd}\entity{a}(0,-4){Sub}\link{a}{i}
\canbe{k}(4,-3){p}{io}\partof{o}(8,0)
";
const GSM_SAMPLE: &str = r"\abstract{a}(0,0){Abstract}\representable{b}(4,0){Repr}\attribute{a}{b}{x}";

#[test]
fn panel_sans_serif() {
    for (kind, src) in [(Kind::Er, SAMPLE), (Kind::Gsm, GSM_SAMPLE)] {
        let mut ed = editor(src, kind);
        assert!(!checked(&nothing(&mut ed), "sans serif").0);
        set(&mut ed, "sans serif", "true");
        assert!(text(&ed).contains("sans serif"));
        assert!(checked(&nothing(&mut ed), "sans serif").0);
        set(&mut ed, "sans serif", "false");
        assert!(!text(&ed).contains("sans serif"));
    }
}

#[test]
fn panel_shows_an_existing_value() {
    for (setup, on) in [
        (r"\setup{sans serif}", true),
        (r"\setup{sans serif=true}", true),
        (r"\setup{sans serif=false}", false),
    ] {
        let mut ed = editor(&format!("{setup}{SAMPLE}"), Kind::Er);
        assert_eq!(checked(&nothing(&mut ed), "sans serif").0, on, "{setup}");
    }
}

#[test]
fn panel_offers_no_rule_set() {
    // versions up to 3.5 offered one; `rm rules=db1` in old files is still read by the transformation
    let mut ed = editor(&format!("\\setup{{rm rules=db1}}{SAMPLE}"), Kind::Er);
    assert!(nothing(&mut ed).field("setup:rm rules").is_none());
}

#[test]
fn panel_and_menu_agree() {
    let mut ed = editor(r"\entity{p}(0,0){P}\attributes{p}{a}", Kind::Er);
    assert!(!checked(&nothing(&mut ed), "types").0);
    set(&mut ed, "types", "true");
    assert!(ed.doc().types_shown());
    assert!(checked(&nothing(&mut ed), "types").0);
    assert_eq!(ed.session().undo_label(), Some("Show data types"));
    ed.undo();
    assert!(!checked(&nothing(&mut ed), "types").0);
}

#[test]
fn panel_toggles_the_attribute_style() {
    let mut ed = editor(r"\entity{p}(0,0){P}\attributes{p}{a}", Kind::Er);
    assert_eq!(choice(&nothing(&mut ed), "setup:attribute style").0, "compact");
    set(&mut ed, "setup:attribute style", "chen");
    assert!(text(&ed).contains("attribute style=chen"));
    set(&mut ed, "setup:attribute style", "compact");
    assert!(!text(&ed).contains("attribute style"));
}

#[test]
fn generalisation_diagram_toggles() {
    let mut ed = editor(&isa_doc("{cd}"), Kind::Er);
    assert_eq!(choice(&nothing(&mut ed), "setup:isa shape").0, "hexagon");
    set(&mut ed, "setup:isa shape", "diamond");
    assert!(text(&ed).contains("isa shape=diamond"));
    set(&mut ed, "setup:isa shape", "hexagon");
    assert!(!text(&ed).contains("isa shape"));
    set(&mut ed, "setup:constraint style", "words");
    assert!(text(&ed).contains("constraint style=words"));
}

#[test]
fn gsm_diagram_has_its_own_settings() {
    let mut ed = editor(GSM_SAMPLE, Kind::Gsm);
    let f = nothing(&mut ed);
    assert!(f.field("setup:label sloped").is_some() && f.field("notation").is_none());
    assert!(f.all_text().contains("GSM diagram"));
}

// -- labels and ids ---------------------------------------------------------------------------

#[test]
fn label_edit_renames_default_id() {
    let mut ed = editor(
        r"\entity{entity}(0,0){Entity}\relationship{relates}(4,0){relates}\link{entity}{relates}",
        Kind::Er,
    );
    let f = select(&mut ed, "entity");
    assert_eq!(f.focus.as_deref(), Some("label"));
    set(&mut ed, "label", "Student\nfirst year");
    assert!(text(&ed).contains(r"\entity{student}(0,0){Student\\first year}"), "{}", text(&ed));
    assert!(text(&ed).contains(r"\link{student}{relates}"));
    // an id chosen by hand stays
    set(&mut ed, "label", "Pupil");
    assert!(text(&ed).contains(r"\entity{student}(0,0){Pupil}"));
}

#[test]
fn ids_must_be_unique() {
    let mut ed = editor(r"\entity{a}(0,0){A}\entity{b}(4,0){B}", Kind::Er);
    select(&mut ed, "a");
    assert_eq!(set(&mut ed, "id", "b"), Outcome::Refused("The id 'b' is already used.".into()));
    set(&mut ed, "id", "c");
    assert!(text(&ed).contains(r"\entity{c}"));
    assert_eq!(ed.session().undo_label(), Some("Rename"));
}

#[test]
fn position_fields_move_and_relative_positions_offer_a_button() {
    let mut ed = editor(r"\entity{a}(0,0){A}\entity[right=of a]{b}{B}", Kind::Er);
    let f = select(&mut ed, "a");
    assert!(
        matches!(f.field("pos"), Some(Field::Position { x, y, step }) if *x == 0.0 && *y == 0.0 && *step == 0.25)
    );
    set(&mut ed, "pos.x", "1.23456");
    assert!(text(&ed).contains(r"\entity{a}(1.235,0){A}"), "{}", text(&ed));
    let f = select(&mut ed, "b");
    assert!(f.field("pos").is_none() && f.field("absolute").is_some());
    set(&mut ed, "absolute", "");
    assert!(matches!(select(&mut ed, "b").field("pos"), Some(Field::Position { .. })));
}

#[test]
fn a_junction_point_shows_its_position() {
    let mut ed = editor(r"\point{p}(1,2)", Kind::Er);
    let f = select(&mut ed, "p");
    assert_eq!(f.title, "Junction point");
    assert!(matches!(f.field("pos"), Some(Field::Position { x, y, .. }) if *x == 1.0 && *y == 2.0));
}

#[test]
fn more_options_keep_the_ones_with_fields() {
    let mut ed = editor(r"\entity[weak, minimum width=3cm]{a}(0,0){A}", Kind::Er);
    let f = select(&mut ed, "a");
    assert!(matches!(f.field("more"), Some(Field::Line { value, .. }) if value == "minimum width=3cm"));
    set(&mut ed, "more", "fill=red, draw=blue");
    assert!(text(&ed).contains(r"\entity[weak, fill=red, draw=blue]{a}"), "{}", text(&ed));
}

#[test]
fn several_selected() {
    let mut ed = editor(r"\entity{a}(0,0){A}\entity{b}(4,0){B}", Kind::Er);
    ed.select_all();
    let f = panel::form(&mut ed);
    assert_eq!(f.title, "2 elements selected");
    set(&mut ed, "delete", "");
    assert!(ed.doc().nodes().next().is_none());
}

#[test]
fn plain_tex_is_shown_not_edited() {
    let mut ed = editor("\\draw (0,0) -- (1,1);\n", Kind::Er);
    let key = ed.doc().elements[0].key;
    let f = select_key(&mut ed, key);
    assert!(f.title.starts_with("Plain TeX"));
    assert!(matches!(f.rows.first().map(|r| &r.field), Some(Field::Code(t)) if t.contains("draw")));
}

// -- weak entities and dependencies -----------------------------------------------------------

#[test]
fn weak_checkbox_drops_the_double_line_and_a_fix_brings_it_back() {
    let mut ed = editor(
        r"\entity{a}(0,0){A}\relationship[dependency=Id]{r}(4,0){R}\entity[weak]{weak}(8,0){W}\link{a}{r}{1}\link[double]{r}{weak}{n}",
        Kind::Er,
    );
    select(&mut ed, "weak");
    set(&mut ed, "weak", "false");
    assert!(!ed.doc().edges().nth(1).unwrap().opts.contains("double"));
    set(&mut ed, "weak", "true");
    let (list, _, _) = problems::listed(&mut ed, true);
    assert_eq!(list.iter().filter(|p| p.fix.is_some()).count(), 1, "{list:?}");
    assert_eq!(problems::fix_all(&mut ed, true), 1);
    assert!(ed.doc().edges().nth(1).unwrap().opts.contains("double"));
    assert_eq!(ed.session().undo_label(), Some("Fix problems"));
}

const KIND_SRC: &str = r"\entity{g}(0,0){Building}\attributes{g}{*nr}\relationship[%s]{r}(4,0){has}\entity[weak]{w}(8,0){Room}\attributes{w}{*no}\link{g}{r}{1}\link[double]{r}{w}{n}";

fn kind_panel(opts: &str) -> (Editor, Form) {
    let mut ed = editor(&KIND_SRC.replace("%s", opts), Kind::Er);
    let f = select(&mut ed, "r");
    (ed, f)
}

#[test]
fn id_explains_that_it_includes_existence() {
    let text = kind_panel("dependency=Id").1.all_text();
    for want in [
        "which includes existence dependency",
        "Each Room can only exist with its Building",
        "the key of Building becomes part of the primary key",
        "dashes",
    ] {
        assert!(text.contains(want), "{want}: {text}");
    }
}

#[test]
fn e_explains_that_it_is_existence_only() {
    let text = kind_panel("dependency=E").1.all_text();
    for want in [
        "Existence dependency only",
        "unique everywhere",
        "the key of Building becomes a foreign key",
        "solidly",
    ] {
        assert!(text.contains(want), "{want}: {text}");
    }
}

#[test]
fn plain_weak_says_what_it_is_treated_as() {
    let text = kind_panel("weak").1.all_text();
    assert!(text.contains("Not specified") && text.contains("identification dependency (Id)"), "{text}");
}

#[test]
fn normal_relationship_has_no_explanation() {
    let mut ed = editor(&KIND_SRC.replace("[%s]", ""), Kind::Er);
    assert!(!select(&mut ed, "r").all_text().contains("can only exist"));
}

#[test]
fn not_linked_yet() {
    let mut ed = editor(r"\relationship[dependency=E]{r}(0,0){has}", Kind::Er);
    assert!(select(&mut ed, "r").all_text().contains("Each weak entity can only exist with its owner"));
}

#[test]
fn every_choice_explains_itself_in_its_tooltip() {
    let (_, items) = choice(&kind_panel("dependency=E").1, "kind");
    let (_, text, tip) = items.iter().find(|i| i.0 == "Id").unwrap();
    assert_eq!(text, "identification dependency (Id)");
    assert!(tip.contains("Each Room can only exist with its Building"));
}

#[test]
fn choosing_id_changes_the_file_and_undo_takes_it_back() {
    let (mut ed, _) = kind_panel("dependency=E");
    set(&mut ed, "kind", "Id");
    assert!(text(&ed).contains(r"\relationship[dependency=Id]{r}"));
    assert!(select(&mut ed, "r").all_text().contains("includes existence"));
    ed.undo();
    assert!(text(&ed).contains(r"\relationship[dependency=E]{r}"));
}

const DEPENDENT: &str = r"\entity[weak]{e2}(0,0){Room}\entity[weak]{e3}(8,0){Seat}\relationship[dependency=Id]{b}(4,0){B}\link{b}{e2}{1}%s";

fn dependent_panel(last: &str) -> (Editor, Form) {
    let mut ed = editor(&DEPENDENT.replace("%s", last), Kind::Er);
    let f = select(&mut ed, "b");
    (ed, f)
}

#[test]
fn the_text_has_the_right_direction() {
    assert!(
        dependent_panel(r"\link[double]{b}{e3}{n}")
            .1
            .all_text()
            .contains("Each Seat can only exist with its Room")
    );
}

#[test]
fn the_text_asks_when_it_is_not_decided() {
    let (_, f) = dependent_panel(r"\link{b}{e3}{n}");
    let text = f.all_text();
    assert!(text.contains("choose below which one depends on the other"));
    assert!(!text.contains("Each Room can") && !text.contains("Each Seat can"));
    assert_eq!(choice(&f, "dependent").0, "");
}

#[test]
fn choosing_moves_the_double_line() {
    let (mut ed, f) = dependent_panel(r"\link[double]{b}{e3}{n}");
    let (current, items) = choice(&f, "dependent");
    assert_eq!(current, "e3");
    assert_eq!(items.iter().find(|i| i.0 == "e2").unwrap().1, "Room");
    set(&mut ed, "dependent", "e2");
    assert!(text(&ed).contains(r"\link[double]{b}{e2}{1}"));
    assert!(text(&ed).contains(r"\link{b}{e3}{n}"));
    assert!(select(&mut ed, "b").all_text().contains("Each Room can only exist with its Seat"));
    ed.undo();
    assert!(text(&ed).contains(r"\link[double]{b}{e3}{n}"));
}

#[test]
fn one_weak_entity_has_no_such_choice() {
    let mut ed = editor(
        r"\entity{o}(0,0){O}\entity[weak]{w}(8,0){W}\relationship[weak]{b}(4,0){B}\link{b}{o}{1}\link[double]{b}{w}{n}",
        Kind::Er,
    );
    assert!(select(&mut ed, "b").field("dependent").is_none());
}

// -- generalisation ---------------------------------------------------------------------------

fn isa_doc(constraint: &str) -> String {
    format!(
        "\\entity{{p}}(0,0){{Person}}\\attributes{{p}}{{*PID}}\n\\isa{{i}}(0,-2.2){{p}}{constraint}\n\\entity{{a}}(-3,-4){{Employee}}\\entity{{b}}(3,-4){{Student}}\n\\link{{a}}{{i}}\\link{{b}}{{i}}\n"
    )
}

fn constraint(ed: &Editor) -> Option<String> {
    ed.doc()
        .by_id("i")
        .and_then(|e| e.as_node())
        .and_then(|n| n.generalisation.clone())
        .and_then(|g| g.constraint)
}

#[test]
fn shows_name_and_abbreviation() {
    let mut ed = editor(&isa_doc("{cd}"), Kind::Er);
    let f = select(&mut ed, "i");
    let (o, oi) = choice(&f, "overlap");
    let (c, ci) = choice(&f, "coverage");
    assert_eq!((o.as_str(), c.as_str()), ("d", "c"));
    let texts = |v: &[(String, String, String)]| v.iter().map(|i| i.1.clone()).collect::<Vec<_>>();
    assert_eq!(texts(&oi), ["(not specified)", "disjoint (d)", "overlapping (o)"]);
    assert_eq!(texts(&ci), ["(not specified)", "complete (c)", "incomplete (i)"]);
    assert_eq!(f.title, "Generalisation (IS_A)");
}

#[test]
fn titles_name_the_element_in_full() {
    let mut ed = editor(
        "\\entity{e}(0,0){E}\n\\entity[weak]{w}(4,0){W}\n\\relationship{r}(2,2){r}\n\\link{e}{r}{1}\n",
        Kind::Er,
    );
    assert_eq!(select(&mut ed, "e").title, "Entity type");
    assert_eq!(select(&mut ed, "w").title, "Weak entity type");
    assert_eq!(select(&mut ed, "r").title, "Relationship type");
    let mut ed =
        editor("\\abstract{a}(0,0){A}\n\\representable{n}(3,0){N}\n\\attribute{a}{n}{name}\n", Kind::Gsm);
    assert_eq!(select(&mut ed, "a").title, "Abstract type");
    assert_eq!(select(&mut ed, "n").title, "Representable type");
}

#[test]
fn an_entity_lists_all_its_attributes_inherited_ones_with_their_trace() {
    let text = "\\entity{person}(0,0){Person}\n\\attributes{person}{*SSN, Name}\n\\isa{i}(0,-2){person}{cd}\n\
        \\entity{emp}(0,-4){Employee}\n\\attributes{emp}{Salary}\n\\link{emp}{i}\n";
    let mut ed = editor(text, Kind::Er);
    let f = select(&mut ed, "emp");
    let rows = match f.rows.last().map(|r| &r.field) {
        Some(Field::Lineage(rows)) => rows.clone(),
        other => panic!("no lineage row at the bottom: {other:?}"),
    };
    let names: Vec<(&str, bool, &str)> =
        rows.iter().map(|r| (r.name.as_str(), r.key, r.from.as_str())).collect();
    assert_eq!(names, vec![("Salary", false, ""), ("SSN", true, "Person"), ("Name", false, "Person")]);
    assert!(rows[1].trace.contains("Employee → Person (IS_A)"), "{}", rows[1].trace);
    assert!(rows[1].target.ends_with(":0"));
    // the supertype has only its own; a relationship has no such list
    let f = select(&mut ed, "person");
    assert!(matches!(f.rows.last().map(|r| &r.field), Some(Field::Lineage(rows)) if rows.len() == 2));
}

#[test]
fn explains_with_real_names() {
    let mut ed = editor(&isa_doc("{cd}"), Kind::Er);
    let text = select(&mut ed, "i").all_text();
    assert!(text.contains("So each Person is exactly one of: Employee, Student"), "{text}");
    assert!(text.contains("Also called total"));
    // coverage above overlap, as the letters (cd); without coverage, overlap explains itself
    let rows: Vec<String> = select(&mut ed, "i").rows.iter().map(|r| r.id.clone()).collect();
    let at = |id: &str| rows.iter().position(|r| r == id).unwrap();
    assert!(at("coverage") < at("overlap"), "{rows:?}");
    let mut ed = editor(&isa_doc("{d}"), Kind::Er);
    assert!(select(&mut ed, "i").all_text().contains("Each Person is at most one of: Employee, Student."));
    let mut ed = editor(&isa_doc("{co}"), Kind::Er);
    assert!(
        select(&mut ed, "i").all_text().contains("So each Person is one or several of: Employee, Student.")
    );
}

#[test]
fn choosing_writes_letters() {
    let mut ed = editor(&isa_doc("{cd}"), Kind::Er);
    select(&mut ed, "i");
    set(&mut ed, "overlap", "o");
    assert_eq!(constraint(&ed).as_deref(), Some("co"));
    set(&mut ed, "coverage", "i");
    assert_eq!(constraint(&ed).as_deref(), Some("io"));
    ed.undo();
    assert_eq!(constraint(&ed).as_deref(), Some("co"));
}

#[test]
fn one_answer_at_a_time() {
    let mut ed = editor(&isa_doc(""), Kind::Er);
    assert_eq!(choice(&select(&mut ed, "i"), "overlap").0, "");
    set(&mut ed, "overlap", "d");
    assert_eq!(constraint(&ed).as_deref(), Some("d"));
    assert!(text(&ed).contains(r"\isa{i}(0,-2.2){p}{d}"), "{}", text(&ed));
    set(&mut ed, "overlap", "");
    assert_eq!(constraint(&ed), None);
}

#[test]
fn free_text_is_understood_and_replaced_on_choice() {
    let mut ed = editor(&isa_doc("{vollständig, disjunkt}"), Kind::Er);
    let f = select(&mut ed, "i");
    assert_eq!(choice(&f, "coverage").0, "c");
    assert!(f.all_text().contains("Written as"));
    set(&mut ed, "overlap", "o");
    assert_eq!(constraint(&ed).as_deref(), Some("co"));
}

// -- the cardinality picker -------------------------------------------------------------------

const IMKER: &str = "\\entity{h}(0,0){Imker}\\attributes{h}{*ImkerID}\n\\relationship{x}(3,0){pflegt}\n\\entity{d}(6,0){Bienenvolk}\\attributes{d}{*VolkID}\n\\link{h}{x}{%a}\\link{x}{d}{%b}\n";

fn halter(a: &str, b: &str) -> String {
    IMKER.replace("%a", a).replace("%b", b)
}

fn minmax() -> String {
    format!("\\setup{{card notation=minmax}}{}", halter("(1,*)", "(1,1)"))
}

fn edge(ed: &mut Editor, k: usize) -> panel::CardField {
    let key = ed.doc().elements.iter().filter(|e| matches!(e.body, Body::Edge(_))).nth(k).unwrap().key;
    match select_key(ed, key).field("card") {
        Some(Field::Cardinality(c)) => c.clone(),
        other => panic!("{other:?}"),
    }
}

fn labels(ed: &Editor) -> Vec<Option<String>> {
    ed.doc().edges().map(|e| e.label.clone()).collect()
}

#[test]
fn n_notation_offers_exactly_four_choices() {
    let mut ed = editor(&halter("1", "n"), Kind::Er);
    let c = edge(&mut ed, 0);
    assert_eq!(c.choices.iter().map(|i| i.value.as_str()).collect::<Vec<_>>(), ["1", "c", "n", "cn"]);
    assert_eq!(c.current.as_deref(), Some("1"));
    assert_eq!(c.notation, "n");
    assert_eq!(c.sentence, "Each Bienenvolk has exactly one Imker.");
}

#[test]
fn choosing_a_value() {
    let mut ed = editor(&halter("1", "n"), Kind::Er);
    edge(&mut ed, 0);
    set(&mut ed, "card", "cn");
    assert_eq!(labels(&ed), [Some("cn".into()), Some("n".into())]);
    assert_eq!(edge(&mut ed, 0).sentence, "Each Bienenvolk has any number (also none) Imker.");
    ed.undo();
    assert_eq!(labels(&ed), [Some("1".into()), Some("n".into())]);
}

#[test]
fn min_max_offers_two_fields() {
    let mut ed = editor(&minmax(), Kind::Er);
    let c = edge(&mut ed, 0);
    assert!(c.choices.is_empty());
    assert_eq!((c.lo.as_str(), c.hi.as_str()), ("1", "*"));
    set(&mut ed, "card.minmax", "3,*");
    assert_eq!(labels(&ed)[0].as_deref(), Some("(3,*)"));
}

#[test]
fn min_max_rejects_impossible_bounds() {
    let mut ed = editor(&minmax(), Kind::Er);
    edge(&mut ed, 0);
    let out = set(&mut ed, "card.minmax", "3,2");
    assert!(matches!(&out, Outcome::Invalid(t) if t.contains("can't be smaller")), "{out:?}");
    assert_eq!(labels(&ed)[0].as_deref(), Some("(1,*)"));
}

#[test]
fn unknown_minimum() {
    let mut ed = editor(&minmax(), Kind::Er);
    edge(&mut ed, 0);
    set(&mut ed, "card.minmax", "·,*");
    assert_eq!(labels(&ed)[0].as_deref(), Some("(·,*)"));
    assert!(edge(&mut ed, 0).sentence.contains("minimum is unknown"));
}

#[test]
fn deviation_applies_to_the_whole_relationship_and_warns() {
    let mut ed = editor(&halter("1", "n"), Kind::Er);
    let c = edge(&mut ed, 0);
    assert!(c.deviation.as_ref().unwrap().0.contains("discouraged"));
    match panel::set(&mut ed, "card.deviate", "", false) {
        Outcome::Ask { text, .. } => assert!(text.contains("opposite ends")),
        other => panic!("{other:?}"),
    }
    assert_eq!(labels(&ed), [Some("1".into()), Some("n".into())], "nothing before the answer");
    set(&mut ed, "card.deviate", "");
    assert_eq!(labels(&ed), [Some("(1,*)".into()), Some("(1,1)".into())]);
    let c = edge(&mut ed, 0);
    assert_eq!(c.notation, "minmax");
    assert!(c.deviation.as_ref().unwrap().0.contains("Back to n-notation"));
    set(&mut ed, "card.deviate", "");
    assert_eq!(labels(&ed), [Some("1".into()), Some("n".into())]);
}

const BEGLEITEN: &str = r"\entity{p}(0,0){Ausbilder}\attributes{p}{*PersNr}
\entity{s}(6,0){Lehrlinge}\attributes{s}{*LehrlingNr}
\entity{t}(3,-3){Projektthemen}\attributes{t}{*ThemaNr}
\relationship{b}(3,0){begleiten}\attributes{b}{Note}
\link{p}{b}{1}\link{b}{s}{n}\link{b}{t}{1}
";

#[test]
fn deviation_on_n_ary_removes_the_labels() {
    let mut ed = editor(BEGLEITEN, Kind::Er);
    edge(&mut ed, 0);
    match panel::set(&mut ed, "card.deviate", "", false) {
        Outcome::Ask { text, .. } => assert!(text.contains("can't be converted")),
        other => panic!("{other:?}"),
    }
    set(&mut ed, "card.deviate", "");
    assert_eq!(labels(&ed), [None, None, None]);
}

#[test]
fn old_label_is_shown() {
    let mut ed = editor(&halter("m", "n"), Kind::Er);
    let c = edge(&mut ed, 0);
    assert_eq!(c.current, None);
    assert!(c.note.unwrap().contains("Written as “m”"));
}

#[test]
fn switching_the_diagram() {
    let mut ed = editor(&halter("1", "n"), Kind::Er);
    nothing(&mut ed);
    match panel::set(&mut ed, "notation", "minmax", false) {
        Outcome::Ask { text, .. } => {
            assert!(text.contains("opposite ends") && text.contains("1 relationship(s)"), "{text}");
        }
        other => panic!("{other:?}"),
    }
    set(&mut ed, "notation", "minmax");
    assert_eq!(diagram_core::cardinality::diagram_notation(ed.doc()), "minmax");
    assert_eq!(labels(&ed), [Some("(1,*)".into()), Some("(1,1)".into())]);
    ed.undo();
    assert_eq!(diagram_core::cardinality::diagram_notation(ed.doc()), "n");
    assert_eq!(labels(&ed), [Some("1".into()), Some("n".into())]);
}

#[test]
fn switching_lists_what_stays() {
    let mut ed = editor(BEGLEITEN, Kind::Er);
    match panel::set(&mut ed, "notation", "minmax", false) {
        Outcome::Ask { text, .. } => assert!(text.contains("begleiten"), "{text}"),
        other => panic!("{other:?}"),
    }
    set(&mut ed, "notation", "minmax");
    assert!(check(ed.doc()).iter().any(|p| p.message.contains("uses n-notation")));
}

#[test]
fn crow_toggle_only_for_n_notation() {
    let mut ed = editor(&halter("1", "n"), Kind::Er);
    assert_eq!(checked(&nothing(&mut ed), "crowsfoot"), (false, true));
    set(&mut ed, "crowsfoot", "true");
    assert!(text(&ed).contains("card symbols=crowsfoot"));
    set(&mut ed, "notation", "minmax");
    assert!(!text(&ed).contains("crowsfoot"));
    assert_eq!(checked(&nothing(&mut ed), "crowsfoot"), (false, false));
}

// -- edges ------------------------------------------------------------------------------------

#[test]
fn edge_kind_is_refused_where_the_rules_say_so() {
    let mut ed = editor(r"\abstract{a}(0,0){A}\representable{b}(4,0){B}\attribute{a}{b}{x}", Kind::Gsm);
    let key = ed.doc().elements[2].key;
    let f = select_key(&mut ed, key);
    assert!(matches!(f.field("reverse"), Some(Field::Text { button: Some(b), .. }) if b == "Reverse"));
    let out = set(&mut ed, "kind", "generalization");
    assert_eq!(out, Outcome::Refused("Not allowed: IS-A edges start at a subtype (circle)".into()));
    set(&mut ed, "label", " y ");
    assert!(text(&ed).contains(r"\attribute{a}{b}{y}"), "{}", text(&ed));
    set(&mut ed, "reverse", "");
    assert!(text(&ed).contains(r"\attribute{b}{a}{y}"), "{}", text(&ed));
}

#[test]
fn routing() {
    let mut ed = editor(r"\entity{a}(0,0){A}\entity{b}(4,2){B}\link{a}{b}", Kind::Er);
    let key = ed.doc().elements[2].key;
    assert_eq!(choice(&select_key(&mut ed, key), "routing").0, "straight");
    set(&mut ed, "routing", "curved");
    assert!(text(&ed).contains(r"\link[bend=30]{a}{b}"));
    assert!(
        matches!(select_key(&mut ed, key).field("bend"), Some(Field::Number { value, .. }) if *value == 30.0)
    );
    set(&mut ed, "bend", "-20");
    assert!(text(&ed).contains(r"\link[bend=-20]{a}{b}"), "{}", text(&ed));
    set(&mut ed, "routing", "via");
    assert!(text(&ed).contains(r"\link[via={(4,0)}]{a}{b}"), "{}", text(&ed));
    set(&mut ed, "routing", "-|");
    assert!(text(&ed).contains(r"\link[ortho=-|]{a}{b}"), "{}", text(&ed));
}

// -- attribute lists --------------------------------------------------------------------------

fn tree(attrs: &str) -> Editor {
    let mut ed = editor(&format!(r"\entity{{p}}(0,0){{P}}\attributes{{p}}{{{attrs}}}"), Kind::Er);
    let key = ed.doc().elements[1].key;
    ed.set_selection(vec![key]);
    ed
}

fn items(ed: &Editor) -> String {
    let t = ed.doc().elements.iter().find_map(|e| match &e.body {
        Body::Attributes(_) => Some(e.to_tex()),
        _ => None,
    });
    let t = t.unwrap();
    t[r"\attributes{p}{".len()..t.len() - 1].to_string()
}

fn rows(ed: &mut Editor) -> Vec<panel::AttrRow> {
    match panel::form(ed).field("attributes") {
        Some(Field::Attributes(rows)) => rows.clone(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn rows_show_the_parts() {
    let mut ed = tree("a{b, c}, d");
    let r = rows(&mut ed);
    assert_eq!(r.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["a", "b", "c", "d"]);
    assert_eq!(r.iter().map(|r| r.depth).collect::<Vec<_>>(), [0, 1, 1, 0]);
    assert!(r[0].composite && !r[1].composite);
    assert_eq!(panel::form(&mut ed).focus.as_deref(), Some("attributes"));
}

#[test]
fn editing_a_part_keeps_the_rest() {
    let mut ed = tree(r"*a{b : string, c}, \{d\}, /e = {1 + 1}, f@1/2");
    set(&mut ed, "attr.name.1", "B");
    assert_eq!(items(&ed), r"*a{B : string, c}, \{d\}, /e = {1 + 1}, f@1/2");
}

#[test]
fn make_part_and_back() {
    let mut ed = tree("a, b, c");
    set(&mut ed, "attr.indent.1", "");
    assert_eq!(items(&ed), "a{b}, c");
    assert_eq!(set(&mut ed, "attr.indent.2", ""), Outcome::Row { row: 2, edit: false });
    assert_eq!(items(&ed), "a{b, c}");
    set(&mut ed, "attr.outdent.1", ""); // b goes behind c, or c would be part of b
    assert_eq!(items(&ed), "a{c}, b");
    ed.undo();
    assert_eq!(items(&ed), "a{b, c}");
}

#[test]
fn first_row_cannot_be_a_part() {
    let mut ed = tree("a, b");
    set(&mut ed, "attr.indent.0", "");
    assert_eq!(items(&ed), "a, b");
    set(&mut ed, "attr.outdent.0", "");
    assert_eq!(items(&ed), "a, b");
}

#[test]
fn moving_takes_the_parts_along() {
    let mut ed = tree("a, b{c, d}, e");
    for (op, want) in [
        ("attr.up.1", "b{c, d}, a, e"),
        ("attr.down.0", "a, b{c, d}, e"),
        ("attr.down.1", "a, e, b{c, d}"),
        ("attr.up.4", "a, e, b{d, c}"),
        ("attr.up.3", "a, e, b{d, c}"), // already the first of the parts
    ] {
        set(&mut ed, op, "");
        assert_eq!(items(&ed), want, "{op}");
    }
}

#[test]
fn remove_takes_the_parts_along() {
    let mut ed = tree("a{b, c}, d");
    set(&mut ed, "attr.remove.0", "");
    assert_eq!(items(&ed), "d");
}

#[test]
fn add_goes_behind_the_parts() {
    let mut ed = tree("a{b}, c");
    set(&mut ed, "attr.add.0", "");
    assert_eq!(items(&ed), "a{b}, attribute, c");
    set(&mut ed, "attr.add.1", "");
    assert_eq!(items(&ed), "a{b, attribute}, attribute, c");
}

#[test]
fn attribute_kind_and_formula() {
    let mut ed = tree("a, b");
    assert_eq!(rows(&mut ed)[1].kind, "normal");
    assert_eq!(set(&mut ed, "attr.kind.1", "multi-valued"), Outcome::Row { row: 1, edit: false });
    assert_eq!(items(&ed), r"a, \{b\}");
    assert_eq!(rows(&mut ed)[1].kind, "multi-valued");
    set(&mut ed, "attr.kind.1", "derived");
    assert_eq!(items(&ed), "a, /b");
    set(&mut ed, "attr.formula.1", "1 + 1");
    assert_eq!(items(&ed), "a, /b = {1 + 1}");
    set(&mut ed, "attr.kind.1", "normal");
    assert_eq!(items(&ed), "a, b");
}

#[test]
fn kind_follows_the_row() {
    let mut ed = tree(r"a, \{b\}, /c = {x}");
    let r = rows(&mut ed);
    assert_eq!((r[1].kind, r[2].kind, r[2].formula.as_str()), ("multi-valued", "derived", "x"));
}

#[test]
fn keys_unique_and_types() {
    let mut ed = tree("a, b");
    set(&mut ed, "attr.key.0", "true");
    assert_eq!(items(&ed), "*a, b");
    set(&mut ed, "attr.unique.0", "true"); // one or the other
    assert_eq!(items(&ed), "+a, b");
    set(&mut ed, "attr.type.1", "integer");
    assert_eq!(items(&ed), "+a, b : integer");
    assert_eq!(rows(&mut ed)[0].guess, "string");
}

#[test]
fn shift_enter_types_a_whole_list() {
    // what the page sends: the name, then "next" (Shift+Enter), which adds a row to type in
    let mut ed = tree("first");
    let mut row = 0;
    for name in ["PID", "name", "born"] {
        set(&mut ed, &format!("attr.name.{row}"), name);
        match set(&mut ed, &format!("attr.next.{row}"), "") {
            Outcome::Row { row: r, edit: true } => row = r,
            other => panic!("{other:?}"),
        }
        assert_eq!(rows(&mut ed)[row].name, "attribute");
    }
    set(&mut ed, &format!("attr.name.{row}"), "last");
    assert_eq!(items(&ed), "PID, name, born, last");
}

#[test]
fn shift_enter_stays_among_the_parts() {
    let mut ed = tree("a{b}, c");
    let Outcome::Row { row, .. } = set(&mut ed, "attr.next.1", "") else { panic!() };
    set(&mut ed, &format!("attr.name.{row}"), "d");
    assert_eq!(items(&ed), "a{b, d}, c");
}

#[test]
fn an_empty_name_removes_the_row() {
    let mut ed = tree("a, b, c");
    set(&mut ed, "attr.name.1", " ");
    assert_eq!(items(&ed), "a, c");
}

const CHEN: &str = r"\setup{attribute style=chen}
\entity{p}(0,0){Person}
\attributes{p}{*PID, name@2/3, address{street, town}, \{email\}, /age}
";

#[test]
fn list_controls_only_for_compact_lists() {
    let mut ed = editor(CHEN, Kind::Er);
    let key = ed.doc().elements[2].key;
    let f = select_key(&mut ed, key);
    assert!(!has_row(&f, "Bar alignment") && !has_row(&f, "Direction"));
    let mut ed = tree("a");
    let f = panel::form(&mut ed);
    assert!(has_row(&f, "Bar alignment") && has_row(&f, "Direction"));
}

#[test]
fn place_automatically_again() {
    let mut ed = editor(CHEN, Kind::Er);
    let key = ed.doc().elements[2].key;
    assert!(select_key(&mut ed, key).field("ovals-auto").is_some());
    set(&mut ed, "ovals-auto", "");
    assert!(!text(&ed).contains('@'));
    assert!(select_key(&mut ed, key).field("ovals-auto").is_none());
}

#[test]
fn add_attribute_list_from_the_panel() {
    let mut ed = editor(r"\entity{p}(0,0){P}", Kind::Er);
    select(&mut ed, "p");
    assert_eq!(set(&mut ed, "add-tree", ""), Outcome::Row { row: 0, edit: true });
    assert!(text(&ed).contains(r"\attributes{p}{attribute}"), "{}", text(&ed));
    assert_eq!(panel::form(&mut ed).title, "Attribute list");
}

// -- notes and subdomains ---------------------------------------------------------------------

const BOXES: &str = r"\entity{a}(1,1){A}\attributes{a}{*x}
\relationship{r}(5,1){R}
\entity{b}(9,1){B}\attributes{b}{*y}
\link{a}{r}{n}\link{r}{b}{n}
\entity{c}(20,1){C}\attributes{c}{*z}
\subdomain{Outer}{(0,-1) (12,4)}
\subdomain{Inner}{(7,0) (11,2)}
\note(2,3){a note}
\point{p}(3,3)
";

#[test]
fn the_panel_says_what_a_box_contains() {
    let mut ed = editor(BOXES, Kind::Er);
    let key = ed
        .doc()
        .elements
        .iter()
        .find(|e| matches!(&e.body, Body::Subdomain(s) if s.label == "Inner"))
        .unwrap()
        .key;
    let f = select_key(&mut ed, key);
    assert!(
        f.rows
            .iter()
            .any(|r| r.label == "Contains" && matches!(&r.field, Field::Text { text, .. } if text == "B"))
    );
    set(&mut ed, "opt:color", "red!60");
    assert!(text(&ed).contains(r"\subdomain[color=red!60]{Inner}"), "{}", text(&ed));
}

#[test]
fn colours_show_their_swatches() {
    let mut ed = editor(r"\entity[color=orange!35]{a}(0,0){A}", Kind::Er);
    let f = select(&mut ed, "a");
    let Some(Field::Color { swatches, current, tinted }) = f.field("opt:color") else { panic!() };
    assert!(*tinted && current == "orange!35");
    assert!(swatches.iter().any(|s| s.name == "Custom (orange!35)"));
    let red = swatches.iter().find(|s| s.value == "red").unwrap();
    assert!(red.tip.contains("used as red!20"), "{}", red.tip);
    assert!(swatches[0].rgb.is_some(), "the default shows the package's fill");
    assert!(panel::color_rgb("blue!50!black", false).is_some());
    assert!(panel::color_rgb("nonsense", false).is_none());
    set(&mut ed, "opt:color", "");
    assert!(text(&ed).contains(r"\entity{a}(0,0){A}"));
    assert_eq!(ed.session().undo_label(), Some("Change colour"));
}

#[test]
fn note_text_and_anchor() {
    let mut ed = editor(r"\note(0,0){one}", Kind::Er);
    let key = ed.doc().elements[0].key;
    let f = select_key(&mut ed, key);
    assert_eq!(f.focus.as_deref(), Some("label"));
    set(&mut ed, "label", "one\ntwo");
    set(&mut ed, "note-anchor", "north");
    assert!(text(&ed).contains(r"\note[anchor=north](0,0){one\\two}"), "{}", text(&ed));
    set(&mut ed, "note-anchor", "center");
    assert!(text(&ed).contains(r"\note(0,0){one\\two}"), "{}", text(&ed));
}

// -- problems ---------------------------------------------------------------------------------

#[test]
fn problems_are_listed_with_a_summary_and_fixes() {
    let mut ed =
        editor(r"\entity{a}(0,0){A}\entity{a2}(4,0){B}\relationship{r}(2,2){R}\link{a}{r}{m}", Kind::Er);
    let (list, summary, _) = problems::listed(&mut ed, true);
    assert!(summary.contains("hint"), "{summary}");
    let fixable = list.iter().find(|p| p.fix.is_some()).expect("the old label m has a fix");
    assert!(fixable.text.starts_with("link a → r: "), "{}", fixable.text);
    let (without, _, _) = problems::listed(&mut ed, false);
    assert!(without.iter().all(|p| p.severity != Severity::Hint));
    let n = list.iter().position(|p| p.fix.is_some()).unwrap();
    assert!(problems::fix(&mut ed, n, true));
    assert!(text(&ed).contains(r"\link{a}{r}{n}"), "{}", text(&ed));
}

#[test]
fn an_elements_problems_are_in_its_panel_with_their_fix() {
    let mut ed = editor(r"\entity{a}(0,0){A}\relationship{r}(2,2){R}\link{a}{r}{m}", Kind::Er);
    let key = ed.doc().elements[2].key;
    let f = select_key(&mut ed, key);
    let (id, fix) = f
        .rows
        .iter()
        .find_map(|r| match &r.field {
            Field::Problem { fix: Some(fix), .. } => Some((r.id.clone(), fix.clone())),
            _ => None,
        })
        .unwrap();
    set(&mut ed, &id, "");
    assert!(text(&ed).contains(r"\link{a}{r}{n}"));
    assert_eq!(ed.session().undo_label(), Some(fix.as_str()));
}

#[test]
fn badges_mark_elements_with_problems() {
    let mut ed = editor(r"\entity{a}(0,0){A}", Kind::Er);
    assert!(ed.overlay().contains(r#"class="badge""#), "an entity without a key: a hint");
    ed.show_hints = false;
    assert!(!ed.overlay().contains(r#"class="badge""#));
    let key = ed.doc().elements[0].key;
    ed.show_hints = true;
    assert!(problems::tooltip(&mut ed, key, true).starts_with("Hint: "));
}
