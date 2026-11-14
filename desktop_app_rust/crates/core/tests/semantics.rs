//! Rules, the relational schema and GSM -> ER on worked examples: the diagrams in
//! `tests/data/rm_examples.json` and `test_diagrams/`, with the results they must give.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use diagram_core::cardinality::parse_card;
use diagram_core::document::{Document, Kind};
use diagram_core::generalisation::parse_constraint;
use diagram_core::gsm2er::{ConversionSettings, convert, names_key};
use diagram_core::model::{Body, Node};
use diagram_core::newdefaults::{apply_defaults, setting_name};
use diagram_core::relational::{
    Marks, Overrides, Quote, RelNaming, Schema, Settings, column_ident, guess_type, relation_ident, render,
    to_sql, to_text, transform,
};
use diagram_core::rules::{Severity, check};

fn examples() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/rm_examples.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn example(group: &str, key: &str) -> String {
    examples()[group][key].as_str().unwrap().to_string()
}

fn repo(path: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").join(path)).unwrap()
}

fn overrides(id: &str, pairs: &[(&str, &str)]) -> Overrides {
    let mut o = Overrides::new();
    o.insert(id.into(), pairs.iter().map(|(k, v)| (k.to_string(), Some(v.to_string()))).collect());
    o
}

fn schema(src: &str, naming: RelNaming, rules: Option<&str>, ov: &Overrides) -> Schema {
    let doc = Document::parse(src, Kind::Er).unwrap();
    transform(
        &doc,
        &Settings { rel_naming: naming, rules: rules.map(str::to_string), ..Settings::default() },
        ov,
    )
}

/// {name: "Name : {(_key_, col, /fk/)}"} without types, and the schema.
fn relations(src: &str, ov: &Overrides) -> (BTreeMap<String, String>, Schema) {
    let s = schema(src, RelNaming::Relationship, None, ov);
    let text = to_text(&s, Marks::Underscore, false, "");
    let body = text.split("\n\n").nth(1).unwrap_or("");
    let map = body.lines().map(|l| (l.split(" : ").next().unwrap().to_string(), l.to_string())).collect();
    (map, s)
}

fn check_example(key: &str, expected: &[(&str, &str)], ov: &Overrides) -> (BTreeMap<String, String>, Schema) {
    let (rels, s) = relations(&example("EX", key), ov);
    for (name, line) in expected {
        assert_eq!(rels.get(*name).map(String::as_str), Some(*line), "{key}: {rels:?}");
    }
    (rels, s)
}

fn none() -> Overrides {
    Overrides::new()
}

#[test]
fn names_cardinalities_and_types() {
    assert_eq!(column_ident("date of birth"), "dateOfBirth");
    assert_eq!(column_ident("flight hrs"), "flightHrs");
    assert_eq!(relation_ident("Größe"), "Groesse");
    for (text, short) in [
        ("1", "1"),
        ("c", "c"),
        ("m", "m"),
        ("n", "m"),
        ("mc", "mc"),
        ("nc", "mc"),
        ("cm", "mc"),
        ("[0,1]", "c"),
        ("[1,1]", "1"),
        ("[1,m]", "m"),
        ("[0,m]", "mc"),
        ("[3,4]", "m"),
        ("0..*", "mc"),
        ("*", "mc"),
        ("cn", "mc"),
        ("N", "m"),
        ("(0,1)", "c"),
        ("(1,1)", "1"),
        ("(1,*)", "m"),
        ("(0,*)", "mc"),
        ("(·,*)", "mc"),
        ("(2,4)", "m"),
    ] {
        assert_eq!(parse_card(Some(text)).short(), short, "{text}");
    }
    for bad in ["", "x", "(3,1)", "(1,0)", "(0,1]", "[1,*)", "(a,*)"] {
        assert!(!parse_card(Some(bad)).known, "{bad}");
    }
    // only min-max's parentheses count at the entity they are written at (NOT-1)
    for text in ["(1,*)", "(0,1)", "(·,*)", "(2,4)"] {
        assert!(parse_card(Some(text)).here, "{text}");
    }
    for text in ["1", "cn", "[1,*]", "[0,1]", "1..*"] {
        assert!(!parse_card(Some(text)).here, "{text}");
    }
    assert_eq!(parse_constraint(Some("cd")), (Some(true), Some(true)));
    assert_eq!(parse_constraint(Some("{incomplete, overlapping}")), (Some(false), Some(false)));
    assert_eq!(parse_constraint(None), (None, None));
    for (name, t) in [
        ("PID", "integer"),
        ("clubNo", "integer"),
        ("date of birth", "date"),
        ("salary", "decimal"),
        ("isStaff", "boolean"),
        ("name", "string"),
        ("from", "datetime"),
    ] {
        assert_eq!(guess_type(name), t, "{name}");
    }
}

#[test]
fn lecture_examples_binary() {
    check_example(
        "grows_mn",
        &[("export", "export : {(_/regionReference/_, _/articleNo/_, value)}")],
        &none(),
    );
    check_example("hosts_1m", &[("Member", "Member : {(_memberNo_, name, /clubNo/)}")], &none());
    let (_, s) = check_example(
        "rent_mc",
        &[("Bicycle", "Bicycle : {(_frameNo_, title, publisher, year, /riderNo/, returnDate)}")],
        &none(),
    );
    let book = s.relations().find(|r| r.name == "Bicycle").unwrap();
    assert!(book.col("riderNo").unwrap().nullable); // c: books may be available
    let (_, s) = check_example("coach_c1", &[("Club", "Club : {(_clubNo_, name, /memberNo/)}")], &none());
    let dep = s.relations().next().unwrap();
    assert!(!dep.col("memberNo").unwrap().nullable);
    assert_eq!(dep.uniques, vec![vec!["memberNo".to_string()]]);
    assert!(s.decisions.iter().all(|d| d.minor)); // 1:c is determined
    let (_, s) =
        check_example("dock_cc", &[("Charger", "Charger : {(_chargerNo_, label, /bayNo/)}")], &none());
    assert_eq!(s.decisions.len(), 1);
    assert_eq!(s.decisions[0].options.len(), 3); // A, B, own relation
    check_example(
        "dock_cc",
        &[("Bay", "Bay : {(_bayNo_, type, /chargerNo/)}")],
        &overrides("x", &[("rm", "b")]),
    );
    check_example("recursive", &[("isMentorOf", "isMentorOf : {(_/supervisor/_, _/employee/_)}")], &none());
}

#[test]
fn generalisation_options() {
    check_example(
        "gen_cd",
        &[
            ("Member", "Member : {(_memberNo_, name, category)}"),
            ("Referee", "Referee : {(_/memberNo/_, licence)}"),
        ],
        &none(),
    );
    check_example(
        "gen_cd",
        &[("MemberCategory", "MemberCategory : {(_/memberNo/_, _category_)}")],
        &overrides("i", &[("rm", "1"), ("rm category", "relation")]),
    );
    let (rels, _) = check_example(
        "gen_cd",
        &[("Coach", "Coach : {(_memberNo_, name, experience)}")],
        &overrides("i", &[("rm", "2")]),
    );
    assert!(!rels.contains_key("Member"));
    let (rels, _) = check_example(
        "gen_cd",
        &[("Member", "Member : {(_memberNo_, name, experience, licence, machines, category)}")],
        &overrides("i", &[("rm", "3")]),
    );
    assert_eq!(rels.len(), 1);
    check_example(
        "gen_cd",
        &[(
            "Member",
            "Member : {(_memberNo_, name, experience, licence, machines, isCoach, isReferee, isGroundskeeper)}",
        )],
        &overrides("i", &[("rm", "4")]),
    );
    let (_, s) = relations(&example("EX", "gen_cd").replace("{cd}", "{io}"), &none());
    let valid: Vec<bool> = s.decisions[0].options.iter().map(|o| o.valid).collect();
    assert_eq!(valid, [false, true, true, false, false, true]);
    assert_eq!(s.decisions[0].current, 1); // io -> categorisation relation
}

#[test]
fn lecture_examples_n_ary_and_weak() {
    check_example(
        "aggregation",
        &[
            ("Sailboat", "Sailboat : {(_boatNo_, label, /hullNo/)}"),
            ("Sail", "Sail : {(_sailNo_, diameter, /boatNo/)}"),
        ],
        &none(),
    );
    check_example(
        "ternary_mmm",
        &[("Vaccination", "Vaccination : {(_/vaccineNo/_, _/nurseNo/_, _/travellerNo/_, date)}")],
        &none(),
    );
    // several single sides: a candidate key by default, in both rule sets (REL-13, PY-19); the
    // folding into the only complex side, and the key from the complex sides, are choices
    let (rels, _) = check_example("ternary_11m", &[], &none());
    assert!(rels.contains_key("pass"), "{rels:?}");
    check_example(
        "ternary_11m",
        &[("Audition", "Audition : {(_auditionNo_, courses, date, grade, /judgeNo/, /traineeID/)}")],
        &overrides("r", &[("rm", "fold")]),
    );
    let (rels, s) = check_example("chess", &[], &none());
    assert_eq!(s.decisions[0].options.len(), 3);
    assert_eq!(s.decisions[0].current, 1, "{rels:?}");
    check_example(
        "chess",
        &[(
            "playsChess",
            "playsChess : {(/entrantNo_1/, /entrantNo_2/, _/championshipNo/_, _/round/_, result)}",
        )],
        &overrides("x", &[("rm", "table")]),
    );
    check_example(
        "chess",
        &[(
            "playsChess",
            "playsChess : {(_/entrantNo_1/_, /entrantNo_2/, _/championshipNo/_, _/round/_, result)}",
        )],
        &overrides("x", &[("rm key", "p#1")]),
    );
    let (_, s) = check_example("weak_E", &[("Club", "Club : {(_clubID_, name, /marinaID/)}")], &none());
    let dept = s.relations().nth(1).unwrap();
    assert!(dept.fks[0].cascade);
    assert!(!dept.col("marinaID").unwrap().nullable);
    check_example(
        "weak_Id",
        &[("BookingItem", "BookingItem : {(_/bookingNo/_, _bookingItemNo_, product, price)}")],
        &none(),
    );
}

#[test]
fn university_and_airline() {
    let (rels, _) = check_example(
        "workshops",
        &[
            ("Workshop", "Workshop : {(_workshopID_, title, hours, /persID/)}"),
            ("Personnel", "Personnel : {(_persID_, name, salary, jobCategory)}"),
            ("Instructor", "Instructor : {(_/persID/_, position, room)}"),
            ("Technician", "Technician : {(_/persID/_, specialty, /InstructorPersID/)}"),
            ("attend", "attend : {(_/traineeID/_, _/workshopID/_)}"),
            ("require", "require : {(_/previous/_, _/next/_)}"),
            ("exam", "exam : {(_/traineeID/_, _/workshopID/_, /persID/, mark)}"),
        ],
        &none(),
    );
    assert_eq!(rels.len(), 8);
    let s = schema(&repo("desktop_app_rust/test_diagrams/railway.erd"), RelNaming::Entities, None, &none());
    let text = to_text(&s, Marks::Underscore, false, "");
    let rels: HashMap<&str, &str> =
        text.split("\n\n").nth(1).unwrap().lines().map(|l| (l.split(" : ").next().unwrap(), l)).collect();
    assert_eq!(rels["Member"], "Member : {(_/PID/_, empID, salary, category)}");
    assert_eq!(rels["PersonCategory"], "PersonCategory : {(_/PID/_, _category_)}");
    assert_eq!(rels["Journey"], "Journey : {(_/TID/_, _date_, /PID/, /serID/)}");
    assert_eq!(
        rels["Commuter_book_Journey"],
        "Commuter_book_Journey : {(_/PID/_, _/TID/_, _/date/_, class, seat, price)}"
    );
    assert!(rels["Train"].contains("/departureStID/, /arrivalStID/"));
    assert!(rels.contains_key("Technician_qualified_LocomotiveType"));
    assert!(s.issues.iter().all(|i| i.severity == Severity::Hint));
}

/// (primary key, foreign key columns, other columns)
type KeyShape = (BTreeSet<String>, BTreeSet<String>, BTreeSet<String>);

/// {relation: (primary key, foreign key columns, other columns)}, names without case and
/// punctuation, so that the tests state the expected schemas directly.
fn shape(src: &str) -> (HashMap<String, KeyShape>, Schema) {
    let norm = |n: &str| n.to_lowercase().chars().filter(char::is_ascii_alphanumeric).collect::<String>();
    let s = schema(src, RelNaming::Relationship, Some("db1"), &none());
    let mut out = HashMap::new();
    for r in s.relations() {
        let fk: BTreeSet<String> = r.fk_columns().iter().map(|c| norm(c)).collect();
        let pk: BTreeSet<String> = r.pk().iter().map(|c| norm(c)).collect();
        let others =
            r.columns.iter().map(|c| norm(&c.name)).filter(|c| !pk.contains(c) && !fk.contains(c)).collect();
        out.insert(norm(&r.name), (pk, fk, others));
    }
    (out, s)
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn db1_solutions() {
    // 'pflegt' in min-max means the same as 1:n
    let (rels, s) = shape(&example("DE", "imker_minmax"));
    assert_eq!(rels["bienenvolk"], (set(&["volkid"]), set(&["imkerid"]), set(&["name"])));
    assert_eq!(rels["imker"], (set(&["imkerid"]), set(&[]), set(&["name"])));
    assert!(!s.relations().find(|r| r.name == "Bienenvolk").unwrap().col("ImkerID").unwrap().nullable);
    assert_eq!(s.unenforced, ["each Imker has, in pflegt, Bienenvolk: at least 1"]);
    let n_notation = example("DE", "imker_minmax").replace("{(1,*)}", "{1}").replace("{(1,1)}", "{n}");
    assert_eq!(shape(&n_notation).0, rels);
    // Werkstatt (1,1), Meister (0,1): the foreign key avoids NULLs
    let (rels, s) = shape(&example("DE", "werkstatt_minmax"));
    assert_eq!(rels["werkstatt"], (set(&["werkstattname"]), set(&["persnr"]), set(&[])));
    assert_eq!(
        s.relations().find(|r| r.name == "Werkstatt").unwrap().uniques,
        vec![vec!["PersNr".to_string()]]
    );
    // bounds that cannot be enforced are named
    let (_, s) = shape(&example("DE", "mosaik_minmax"));
    assert_eq!(s.unenforced, ["each Mosaik has, in Belag, Steine: at least 4"]);
    let sql = to_sql(&s, "postgresql", Default::default(), false, "", 255);
    assert!(sql.contains("-- not enforced by this schema") && sql.contains("at least 4"));
}

fn messages(src: &str, kind: Kind, severity: Option<Severity>) -> String {
    let doc = Document::parse(src, kind).unwrap();
    check(&doc)
        .into_iter()
        .filter(|p| severity.is_none_or(|s| p.severity == s))
        .map(|p| p.message)
        .collect::<Vec<_>>()
        .join(" | ")
}

#[test]
fn rules() {
    let good = "\\entity{a}(0,0){A}\n\\relationship{r}(3,0){R}\n\\entity[weak]{b}(6,0){B}\n\\relationship[dependency=Id]{d}(6,-2){has}\n\
                \\entity{c}(9,-2){C}\n\\link{a}{r}{1}\\link{r}{b}{n}\\link[double]{b}{d}{n}\\link{d}{c}{1}\n\
                \\attributes{a}{*k}\\attributes{b}{*k}\\attributes{c}{*k}\n\\isa{i}(0,-3){a}{cd}\n\
                \\entity{s1}(-2,-5){S1}\\entity{s2}(2,-5){S2}\n\\link{s1}{i}\\link{s2}{i}\n";
    assert_eq!(messages(good, Kind::Er, None), "");
    let er = |src: &str, sev| messages(src, Kind::Er, sev);
    assert!(
        er("\\entity{a}(0,0){A}\\entity{b}(3,0){B}\\link{a}{b}", Some(Severity::Error))
            .contains("two entities")
    );
    assert!(
        er("\\entity{a}(0,0){A}\\relationship{r}(3,0){R}\\link{a}{r}{1}", None).contains("fewer than two")
    );
    assert!(
        !er("\\entity{a}(0,0){A}\\relationship{r}(3,0){R}\\link{a}{r}{1}\\link{a}{r}{n}", None)
            .contains("fewer than two")
    );
    assert!(er("\\entity{a}(0,0){A}\\isa{i}(0,-2){a}", None).contains("no subtypes"));
    assert!(er("\\isa{i}(0,-2){nobody}", Some(Severity::Error)).contains("does not exist"));
    assert!(er("\\entity{a}(0,0){A}\\link{a}{zzz}", Some(Severity::Error)).contains("unknown element"));
    assert!(er("\\attributes{zzz}{x}", Some(Severity::Error)).contains("unknown element"));
    assert!(er("\\entity{a}(0,0){A}\\entity{a}(1,0){B}", Some(Severity::Error)).contains("used 2 times"));
    assert!(er("\\entity{a}(0,0){A}", Some(Severity::Hint)).contains("no key attribute"));
    let gsm = |src: &str| messages(src, Kind::Gsm, None);
    assert!(
        gsm("\\subtype{s}(0,0){S}\\subtype{t}(2,0){T}\\generalization{s}{t}\\generalization{t}{s}")
            .contains("cycle")
    );
    assert!(gsm("\\grouping{g}(0,0){G}").contains("exactly one component"));
    // a fix: the double line goes where it belongs
    let mut doc =
        Document::parse("\\entity[weak]{b}(0,0){B}\\relationship[weak]{r}(3,0){R}\\link{r}{b}{n}", Kind::Er)
            .unwrap();
    let p = check(&doc).into_iter().find(|p| p.fix.is_some()).unwrap();
    p.fix.unwrap().apply(&mut doc, p.element);
    assert!(doc.to_text().contains("\\link[double]{r}{b}{n}"));
}

/// A supertype above a complete IS_A needs no key of its own when its subtypes have keys: each of its
/// entities is in one of them (README, "Keys and attributes").
#[test]
fn a_supertype_identified_by_its_subtypes() {
    let doc = |constraint: &str, student: &str, command: &str| {
        format!(
            "\\entity{{p}}(0,0){{Person}}\\attributes{{p}}{{Name}}\n\\{command}{{i}}(0,-2){{p}}{{{constraint}}}\n\
             \\entity{{s}}(-2,-4){{Trainee}}\\attributes{{s}}{{{student}}}\n\\entity{{e}}(2,-4){{Member}}\\attributes{{e}}{{*EmpNo}}\n\
             \\link{{s}}{{i}}\\link{{e}}{{i}}\n"
        )
    };
    let hints = |src: &str| messages(src, Kind::Er, Some(Severity::Hint));
    assert_eq!(hints(&doc("cd", "*MatNr", "isa")), "", "complete, every subtype with a key");
    assert_eq!(hints(&doc("co", "*MatNr", "isa")), "", "overlapping as well");
    assert!(
        hints(&doc("id", "*MatNr", "isa")).contains("no key attribute"),
        "incomplete: the supertype needs one"
    );
    assert!(hints(&doc("", "*MatNr", "isa")).contains("no key attribute"), "coverage not said");
    assert!(hints(&doc("cd", "MatNr", "isa")).contains("no key attribute"), "a subtype without a key");
    assert!(hints(&doc("cd", "*MatNr", "canbe")).contains("no key attribute"), "CAN_BE: unchanged");
    // a subtype without a key of its own, identified in turn by a complete IS_A below it
    let nested = doc("cd", "MatNr", "isa")
        + "\\isa{j}(-2,-6){s}{cd}\n\\entity{b}(-4,-8){Bachelor}\\attributes{b}{*BNr}\n\\entity{m}(0,-8){Master}\\attributes{m}{*MNr}\n\\link{b}{j}\\link{m}{j}\n";
    assert_eq!(hints(&nested), "", "{}", hints(&nested));
    // a supertype with its own key, as before
    assert_eq!(hints(&doc("id", "*MatNr", "isa").replace("{Name}", "{*PID, Name}")), "");
    // a subtype of two supertypes (a consumer is a person and a customer): the key it inherits from
    // Person counts for the complete IS_A below Customer
    let shared = "\\entity{person}(1,5){Person}\\attributes{person}{*Person ID, Name}\n\
                  \\entity{employee}(-3,2){Member}\\attributes{employee}{Function}\n\
                  \\entity{customer}(0,-1){Customer}\\attributes{customer}{Credit rating}\n\
                  \\entity{consumer}(5,1){Consumer}\n\\entity{retailer}(6,-3){Retailer}\\attributes{retailer}{*Retailer ID}\n\
                  \\isa{isa}(1,2){person}{co}\n\\isa{isa2}(5,-1){customer}{cd}\n\
                  \\link{isa}{employee}\\link{isa}{consumer}\\link{isa2}{consumer}\\link{isa2}{retailer}\n";
    assert_eq!(hints(shared), "", "{}", hints(shared));
    // the user's diagram (an empty attribute list on Consumer, a composite attribute elsewhere)
    let users = r#"\setup{sans serif}
\entity{employee}(-2.75,1.75){Member}
\attributes[anchor=west]{employee}{Social security nbr, Postal address{House nbr, Street, Postal code, Region}, Function, Password, Authorization}
\entity{person}(1.125,5){Person}
\attributes{person}{*Person ID, First name, Last name}
\entity{customer}(0,-1.25){Customer}
\attributes[anchor=north]{customer}{Credit rating, Email address}
\entity{consumer}(5,1.25){Consumer}
\attributes{consumer}{}
\entity{retailer}(5.75,-3.25){Retailer}
\attributes{retailer}{*Retailer ID, Sector of trade}
\isa{isa}(1.125,1.75){person}{co}
\isa{isa2}(5.25,-1.25){customer}{cd}
\link{isa}{employee}
\link{isa}{consumer}
\link{isa2}{consumer}
\link{isa2}{retailer}

"#;
    assert_eq!(hints(users), "", "{}", hints(users));
    // without Person's key, Consumer has none, and both supertypes get the hint
    let none = shared.replace("*Person ID, ", "");
    let h = hints(&none);
    assert!(h.contains("entity 'person'") || h.matches("no key attribute").count() == 2, "{h}");
    assert_eq!(h.matches("no key attribute").count(), 2, "{h}");
}

/// Chapters in chapters: a weak relationship between an entity type and itself (README, "Keys and
/// attributes").  The Python app's check and transformation differ here on purpose.
#[test]
fn an_entity_type_depending_on_itself() {
    let book = "\\entity{b}(0,3){Buch}\\attributes{b}{*Nummer}\\entity[weak]{k}(0,0){Kapitel}\\attributes{k}{*KapitelNr, Titel}\n\
                \\relationship[dependency=Id]{e}(0,1.5){enthält}\\link{e}{b}{1}\\link[double]{e}{k}{n}\n";
    let chapters = |dep: &str, upper: &str| {
        format!(
            "{book}\\relationship[{dep}]{{h}}(0,-3){{hat}}\n\
             \\link[role=Oberkapitel, bend=56, double]{{k}}{{h}}{{{upper}}}\\link[role=Unterkapitel, bend=50, double]{{h}}{{k}}{{cn}}\n"
        )
    };
    let er = |src: &str, sev| messages(src, Kind::Er, sev);
    // E: accepted, with a hint about the topmost chapters; no complaint about the cardinalities
    let e = chapters("dependency=E", "c");
    assert_eq!(er(&e, Some(Severity::Warning)), "");
    assert_eq!(er(&e, Some(Severity::Error)), "");
    let hint = er(&e, Some(Severity::Hint));
    assert!(hint.contains("the topmost 'Kapitel' has no 'Oberkapitel'"), "{hint}");
    // the same in min-max: (0,1) at the end of those below
    let mm = e.replace("{c}", "{(0,*)}").replace("{cn}", "{(0,1)}");
    assert!(er(&mm, Some(Severity::Hint)).contains("topmost"), "{}", er(&mm, None));
    assert_eq!(er(&mm, Some(Severity::Warning)), "");
    // every chapter needing one above: there can't be a topmost one
    assert!(er(&chapters("dependency=E", "1"), Some(Severity::Warning)).contains("can't be a topmost"));
    // n:m: one of the ends has to allow at most one
    assert!(er(&chapters("dependency=E", "n"), Some(Severity::Warning)).contains("at most one"));
    // Id, or weak without E/Id: impossible, with a fix that makes it a normal relationship
    for dep in ["dependency=Id", "weak"] {
        let src = chapters(dep, "c");
        let w = er(&src, Some(Severity::Warning));
        assert!(w.contains("can't be an identification dependency") && !w.contains("exactly one"), "{w}");
        let mut doc = Document::parse(&src, Kind::Er).unwrap();
        let p = check(&doc).into_iter().find(|p| p.fix.is_some()).unwrap();
        p.fix.unwrap().apply(&mut doc, p.element);
        let text = doc.to_text();
        assert!(text.contains("\\relationship{h}(0,-3){hat}"), "{text}");
        assert!(text.contains("\\link[role=Oberkapitel, bend=56]{k}{h}{c}"), "{text}");
        assert!(text.contains("\\link[double]{e}{k}{n}"), "the book's dependency stays: {text}");
        assert_eq!(messages(&text, Kind::Er, Some(Severity::Warning)), "");
    }
    // the panel's explanation
    let doc = Document::parse(&e, Kind::Er).unwrap();
    let explain = diagram_core::rules::explain_dependency(&doc, "h", "E");
    assert!(
        explain.contains("only exist with the Oberkapitel") && explain.contains("topmost Kapitel"),
        "{explain}"
    );
    assert!(diagram_core::rules::explain_dependency(&doc, "h", "Id").contains("not possible"));
    // the schema: a foreign key in Kapitel that deletes those below, by default in both rule sets
    for rules in [None, Some("db1")] {
        let s = schema(&e, RelNaming::Relationship, rules, &none());
        assert_eq!(s.relations().count(), 2, "no relation of its own for 'hat'");
        let k = s.relations().find(|r| r.name == "Kapitel").unwrap();
        let fk = k
            .fks
            .iter()
            .find(|f| f.cascade && f.columns.iter().any(|c| c.starts_with("oberkapitel")))
            .unwrap();
        assert!(fk.columns.iter().all(|c| k.col(c).unwrap().nullable), "the topmost chapters have none");
        assert!(!s.issues.iter().any(|i| i.message.contains("itself")));
        let sql = to_sql(&s, "postgresql", Default::default(), false, "", 255);
        assert_eq!(sql.matches("ON DELETE CASCADE").count(), 2, "{sql}");
    }
    // an own relation can't delete those below, and says so
    let own = schema(&e, RelNaming::Relationship, None, &overrides("h", &[("rm", "table")]));
    assert_eq!(own.relations().count(), 3);
    let d = own.decisions.iter().find(|d| d.element_id == "h").unwrap();
    assert!(d.options[0].description.contains("stay, although they depend on it"));
    // Id: treated as a normal relationship, with a warning
    let s = schema(&chapters("dependency=Id", "c"), RelNaming::Relationship, None, &none());
    assert!(s.issues.iter().any(|i| i.message.contains("identification dependent on itself")));
}

/// (entities, relationships (id, label), attribute lists, links (from, to, label))
type Parts =
    (BTreeSet<String>, Vec<(String, String)>, HashMap<String, Vec<String>>, Vec<(String, String, String)>);

fn parts(doc: &Document) -> Parts {
    let ents = doc.nodes().filter(|n| n.command == "entity").map(|n| n.id.clone()).collect();
    let rels = doc
        .nodes()
        .filter(|n| n.command == "relationship")
        .map(|n| (n.id.clone(), n.label.clone().unwrap_or_default()))
        .collect();
    let mut trees = HashMap::new();
    for el in &doc.elements {
        if let Body::Attributes(t) = &el.body {
            trees.insert(
                t.node.clone(),
                t.items.iter().map(|i| format!("{}{}", if i.key { "*" } else { "" }, i.text)).collect(),
            );
        }
    }
    let links = doc
        .edges()
        .map(|e| (e.src_id().to_string(), e.dst_id().to_string(), e.label.clone().unwrap_or_default()))
        .collect();
    (ents, rels, trees, links)
}

fn isas(doc: &Document) -> Vec<(String, String)> {
    doc.nodes()
        .filter_map(|n: &Node| {
            n.generalisation.as_ref().map(|g| (g.supertype.clone(), g.constraint.clone().unwrap_or_default()))
        })
        .collect()
}

#[test]
fn gsm_to_er() {
    // a GSM diagram -> the ER diagram it should give
    let text = repo("desktop_app_rust/test_diagrams/ensemble.gsmd");
    let gsm = Document::parse(&text, Kind::Gsm).unwrap();
    let r = convert(&gsm, &ConversionSettings::default(), None).unwrap();
    let (_, rels, trees, links) = parts(&r.doc);
    assert_eq!(trees["person"], ["*Person ID", "PName"]);
    assert_eq!(trees["address"], ["*Address ID", "Street", "Zip code", "City"]);
    assert_eq!(trees["instr"], ["*Name"]);
    for l in [
        ("person", "plays", "cn"),
        ("instr", "plays", "cn"),
        ("company", "assists", "1"),
        ("agent", "assists", "cn"),
    ] {
        assert!(links.contains(&(l.0.into(), l.1.into(), l.2.into())), "{l:?}");
    }
    assert_eq!(rels.iter().filter(|(_, l)| l.contains("lives")).count(), 1); // inverses merged
    assert_eq!(isas(&r.doc), [("person".to_string(), "io".to_string())]);
    assert_eq!(gsm.to_text(), text); // the GSM diagram is never changed
    // choices change the result
    let mut st = ConversionSettings::default();
    st.choices.insert("multi:person:instr:plays".into(), "attribute".into());
    st.choices.insert("inv:person:address".into(), "separate".into());
    let separate = convert(&gsm, &st, None).unwrap();
    let (ents, rels, _, _) = parts(&separate.doc);
    assert!(!ents.contains("instr"));
    assert_eq!(rels.iter().filter(|(_, l)| l == "lives in" || l == "home of").count(), 2);
    // two relationships: the pair is decided, the line back isn't asked about again (PY-17)
    assert!(!separate.choices.iter().any(|c| c.key == "inv:address:person"), "asked again");
    // a line from a representable type no object type has: left out, and said so (PY-18)
    let lone = Document::parse(
        "\\representable{n}(0,0){Name}\\abstract{p}(5,0){Person}\\attribute{n}{p}{of}",
        Kind::Gsm,
    )
    .unwrap();
    let r = convert(&lone, &ConversionSettings::default(), None).unwrap();
    assert!(
        r.report.iter().any(|m| m.starts_with("the line from Name to Person was left out")),
        "{:?}",
        r.report
    );
    assert!(!r.report.iter().any(|m| m.contains("became a derived attribute")), "{:?}", r.report);
    // year(s) of employment belongs to the relationship assists
    let company = Document::parse(&repo("desktop_app_rust/test_diagrams/company.gsmd"), Kind::Gsm).unwrap();
    let (_, _, trees, _) = parts(&convert(&company, &ConversionSettings::default(), None).unwrap().doc);
    assert_eq!(trees["assists"], ["year(s) of employment"]);
    // an aggregation of object types nothing refers to is a relationship
    let src = "\\abstract{p}(0,0){Person}\\abstract{c}(4,0){Company}\n\\aggregation{job}(2,2){Job}\\component{job}{p}\\component{job}{c}\n\
               \\representable{n}(2,4){Number}\\attribute{job}{n}{years}\n";
    let gsm = Document::parse(src, Kind::Gsm).unwrap();
    let (_, rels, trees, _) = parts(&convert(&gsm, &ConversionSettings::default(), None).unwrap().doc);
    assert_eq!(rels, [("job".to_string(), "Job".to_string())]);
    assert_eq!(trees["job"], ["years"]);
    let mut st = ConversionSettings::default();
    st.choices.insert("agg:job".into(), "entity".into());
    assert!(parts(&convert(&gsm, &st, None).unwrap().doc).0.contains("job"));
    for n in ["Person Id", "personID", "bookingNo", "ID"] {
        assert!(names_key(n, ""), "{n}");
    }
    for n in ["Name", "Video", "Number"] {
        assert!(!names_key(n, ""), "{n}");
    }
    assert!(names_key("Code", "has Id"));
}

#[test]
fn new_diagrams_start_with_the_kept_defaults() {
    let kept: HashMap<String, String> = [
        (setting_name(Kind::Er, "isa shape"), "diamond"),
        (setting_name(Kind::Er, "entity fill"), "red!20"),
        (setting_name(Kind::Er, "card placement"), "nonsense"),
        (setting_name(Kind::Er, "rm rules"), "cdm"),
    ]
    .into_iter()
    .map(|(k, v)| (k, v.to_string()))
    .collect();
    let mut doc = Document::new(Kind::Er);
    apply_defaults(&mut doc, &|name: &str| kept.get(name).cloned());
    // what differs from the packages, nothing invalid, nothing that is the default anyway
    // (a new document ends with an empty line, as in the Python app)
    assert_eq!(doc.to_text(), "\\setup{isa shape=diamond, entity fill=red!20}\n\n");
    let mut doc = Document::parse("\\setup{entity fill=blue}\n\\entity{a}(0,0){A}\n", Kind::Er).unwrap();
    apply_defaults(&mut doc, &|_: &str| None);
    assert_eq!(doc.to_text(), "\\entity{a}(0,0){A}\n");
}

#[test]
fn markdown_marks_keys_by_emphasis() {
    // primary key bold, foreign key italic, both bold italic; Markdown's characters escaped
    let src = "\\entity{b}(0,0){Building}\\attributes{b}{*Name, floor_count}\
               \\relationship[dependency=Id]{c}(4,0){contains}\\entity[weak]{r}(8,0){Room}\\attributes{r}{*No}\
               \\link{b}{c}{1}\\link[double]{c}{r}{n}";
    let s = schema(src, RelNaming::Entities, None, &none());
    let md = render(&s, "markdown", "x_y.erd", Marks::Underscore, true, Quote::Needed, false);
    let lines: Vec<&str> = md.lines().collect();
    assert_eq!(lines.first(), Some(&"## Relational schema of x\\_y.erd"));
    assert!(lines.contains(&diagram_core::relational::MARKDOWN_LEGEND), "{md}");
    assert!(lines.contains(&"- Building : {(**Name** : string, floor\\_count : integer)}"), "{md}");
    assert!(lines.contains(&"- Room : {(***Name*** : string, **No** : integer)}"), "{md}");
    assert!(lines.contains(&"### Foreign keys"), "{md}");
    assert!(lines.contains(&"- Room.Name → Building.Name"), "{md}");
    assert!(!md.ends_with("\n\n"), "no empty lines at the end");
    // without types; a foreign key alone is italic
    let src = "\\entity{s}(0,0){Trainee}\\attributes{s}{*SId}\\relationship{a}(4,0){advised by}\
               \\entity{p}(8,0){Instructor}\\attributes{p}{*PId}\\link{s}{a}{cn}\\link{a}{p}{1}";
    let s = schema(src, RelNaming::Entities, None, &none());
    let md = render(&s, "markdown", "", Marks::Underscore, false, Quote::Needed, false);
    assert!(md.starts_with("## Relational schema\n"), "{md}");
    assert!(md.lines().any(|l| l == "- Trainee : {(**SId**, *PId*)}"), "{md}");
}
