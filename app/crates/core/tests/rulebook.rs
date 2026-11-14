//! The rule books (`rulebook/`, see its README): every page follows the template, ids are unique and
//! links lead somewhere, every example gives what it says (run through the checks, the drawing rules or
//! the conversions), and every rule id in the code has its page.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use diagram_core::document::{Document, Kind};
use diagram_core::gsm2er::{AttrNames, ConversionSettings, convert};
use diagram_core::latex_text::plain_text;
use diagram_core::model::{Body, endpoint_id};
use diagram_core::relational::{Marks, Overrides, Settings, to_text, transform};
use diagram_core::rules::{Severity, check, refusal};

const KINDS: &[&str] = &["error", "warning", "hint", "refused", "conversion", "meaning"];
/// The elements a rule may be about (the palette's names, and a few more).
const ELEMENTS: &[&str] = &[
    "entity type",
    "weak entity type",
    "relationship type",
    "weak relationship type",
    "attribute",
    "attribute list",
    "Chen attribute",
    "IS_A",
    "CAN_BE",
    "PART_OF",
    "line",
    "junction point",
    "note",
    "subdomain",
    "abstract type",
    "representable type",
    "subtype",
    "aggregation",
    "grouping",
    "attribute line",
    "multi-valued attribute line",
    "component line",
    "generalization",
    "specialization",
    "relation",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rulebook")
}

struct Example {
    lang: String,
    role: String,
    rule: String,
    /// after the role: the rule (violates, refused), or settings (converts: `pick=…`, `names=…`, `keys=…`)
    params: Vec<String>,
    text: String,
}

struct Page {
    file: PathBuf,
    fields: toml::Table,
    body: String,
    examples: Vec<Example>,
}

impl Page {
    fn str(&self, k: &str) -> &str {
        self.fields.get(k).and_then(|v| v.as_str()).unwrap_or("")
    }
    fn list(&self, k: &str) -> Vec<String> {
        self.fields
            .get(k)
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    }
}

fn read(file: &Path) -> Page {
    let text = fs::read_to_string(file).unwrap();
    let rest = text.strip_prefix("+++\n").unwrap_or_else(|| panic!("{}: no +++ block", file.display()));
    let (head, body) =
        rest.split_once("\n+++\n").unwrap_or_else(|| panic!("{}: unclosed +++", file.display()));
    let fields: toml::Table = head.parse().unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let mut examples = Vec::new();
    let mut lines = body.lines();
    while let Some(line) = lines.next() {
        let Some(info) = line.strip_prefix("```") else { continue };
        let words: Vec<&str> = info.split_whitespace().collect();
        let mut block = Vec::new();
        for l in lines.by_ref() {
            if l.starts_with("```") {
                break;
            }
            block.push(l);
        }
        if let [lang, role, rest @ ..] = words.as_slice()
            && matches!(*lang, "erd" | "gsmd" | "relations")
        {
            examples.push(Example {
                lang: lang.to_string(),
                role: role.to_string(),
                rule: rest.first().unwrap_or(&"").to_string(),
                params: rest.iter().map(|s| s.to_string()).collect(),
                text: block.join("\n") + "\n",
            });
        }
    }
    Page { file: file.to_path_buf(), fields, body: body.to_string(), examples }
}

/// Every book (`index.md`) and every rule, by folder.
fn books() -> BTreeMap<String, (Page, Vec<Page>)> {
    let mut out = BTreeMap::new();
    for dir in fs::read_dir(root()).unwrap().flatten().filter(|d| d.path().is_dir()) {
        let name = dir.file_name().to_string_lossy().to_string();
        let index = read(&dir.path().join("index.md"));
        let mut rules: Vec<Page> = fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|f| f.path())
            .filter(|p| p.extension().is_some_and(|e| e == "md") && !p.ends_with("index.md"))
            .map(|p| read(&p))
            .collect();
        rules.sort_by(|a, b| a.file.cmp(&b.file));
        out.insert(name, (index, rules));
    }
    out
}

fn all_rules() -> Vec<Page> {
    books().into_values().flat_map(|(_, r)| r).collect()
}

#[test]
fn every_page_follows_the_template() {
    let mut ids = BTreeSet::new();
    let books = books();
    assert!(books.len() >= 6, "the books: {:?}", books.keys().collect::<Vec<_>>());
    for (dir, (index, rules)) in &books {
        let prefix = index.str("prefix");
        assert!(!index.str("book").is_empty() && !prefix.is_empty(), "{dir}/index.md: book and prefix");
        assert!(index.fields.get("order").and_then(|v| v.as_integer()).is_some(), "{dir}/index.md: order");
        for p in rules {
            let id = p.str("id");
            let f = p.file.display().to_string();
            assert!(p.file.file_stem().is_some_and(|s| s == id), "{f}: the file is named after its id {id}");
            let number = id
                .strip_prefix(&format!("{prefix}-"))
                .unwrap_or_else(|| panic!("{f}: id {id} not in {prefix}-"));
            assert!(number.parse::<u32>().is_ok_and(|n| n > 0), "{f}: {id} ends in a number");
            assert!(ids.insert(id.to_string()), "{f}: {id} twice");
            assert!(!p.str("title").is_empty(), "{f}: title");
            let kind = p.str("kind");
            assert!(KINDS.contains(&kind), "{f}: kind {kind:?}");
            let applies = p.list("applies");
            assert!(!applies.is_empty(), "{f}: applies");
            for a in &applies {
                assert!(ELEMENTS.contains(&a.as_str()), "{f}: unknown element {a:?} in applies");
            }
            let mut sections = vec!["**Rule.**"];
            match kind {
                "conversion" => sections.extend(["**Why.**", "**Examples.**"]),
                "meaning" => {}
                _ => sections.extend(["**What the program does.**", "**Why.**", "**Examples.**"]),
            }
            let mut at = 0;
            for s in sections {
                let pos = p.body.find(s).unwrap_or_else(|| panic!("{f}: section {s} missing"));
                assert!(pos >= at, "{f}: section {s} out of order");
                at = pos;
            }
            assert!(!p.body.to_lowercase().contains("slide"), "{f}: no references to outside material");
            let roles: Vec<&str> = p.examples.iter().map(|e| e.role.as_str()).collect();
            match kind {
                "refused" => assert!(
                    roles.contains(&"refused") && roles.contains(&"allowed"),
                    "{f}: refused and allowed examples"
                ),
                "conversion" => assert!(
                    roles.contains(&"converts") && roles.contains(&"result"),
                    "{f}: converts and result examples"
                ),
                "meaning" => {}
                // a hint may be information without a mistake behind it (no `follows` example then)
                "hint" => assert!(roles.contains(&"violates"), "{f}: a violates example"),
                _ => assert!(
                    roles.contains(&"violates") && roles.contains(&"follows"),
                    "{f}: violates and follows examples"
                ),
            }
        }
    }
    for p in all_rules() {
        for s in p.list("see") {
            assert!(ids.contains(&s), "{}: see {s}, which has no page", p.file.display());
        }
    }
}

fn kind_of(lang: &str) -> Kind {
    if lang == "gsmd" { Kind::Gsm } else { Kind::Er }
}

/// What an ER diagram says, positions aside: nodes by command and name (a generalisation by its supertype
/// and letters), attribute lists by owner and items, lines by the names of their ends and their labels.
fn signature(doc: &Document) -> BTreeSet<String> {
    let name = |id: &str| {
        doc.nodes()
            .find(|n| n.id == endpoint_id(id))
            .map(|n| plain_text(n.label.as_deref().unwrap_or(&n.id)))
            .unwrap_or_else(|| id.to_string())
    };
    let mut out = BTreeSet::new();
    for el in &doc.elements {
        match &el.body {
            Body::Node(n) => match &n.generalisation {
                Some(g) => {
                    out.insert(format!(
                        "{} of {} ({})",
                        n.command,
                        name(&g.supertype),
                        g.constraint.as_deref().unwrap_or("")
                    ));
                }
                None => {
                    out.insert(format!("{} {}", n.command, plain_text(n.label.as_deref().unwrap_or(""))));
                }
            },
            Body::Attributes(t) => {
                let items: Vec<String> = diagram_core::model::flatten_items(&t.items)
                    .iter()
                    .map(|(i, d)| format!("{}{}{}", "  ".repeat(*d), if i.key { "*" } else { "" }, i.text))
                    .collect();
                out.insert(format!("attributes of {}: {}", name(&t.node), items.join(", ")));
            }
            Body::Edge(e) => {
                let (a, b) = (name(e.src_id()), name(e.dst_id()));
                // a line is the same whichever way round it is written
                let (a, b) = if a <= b { (a, b) } else { (b, a) };
                out.insert(format!("{} {a} - {b} {}", e.command, e.label.as_deref().unwrap_or("")));
            }
            _ => {}
        }
    }
    out
}

/// The conversion's settings an example names: `names=type|edge|auto`, `keys=false`, and `pick=value`,
/// which answers every question that offers `value` with it.
fn conversion_settings(gsm: &Document, params: &[String], label: &str) -> ConversionSettings {
    let mut s = ConversionSettings::default();
    for p in params {
        match p.split_once('=') {
            Some(("names", v)) => {
                s.attr_names = match v {
                    "type" => AttrNames::Type,
                    "edge" => AttrNames::Edge,
                    _ => AttrNames::Auto,
                }
            }
            Some(("keys", v)) => s.add_keys = v != "false",
            Some(("pick", v)) => {
                // an answer can bring up further questions: answered too, until none is new
                let mut any = false;
                loop {
                    let asked = convert(gsm, &s, None).unwrap().choices;
                    let new: Vec<_> = asked
                        .iter()
                        .filter(|c| c.options.iter().any(|o| o.0 == v) && !s.choices.contains_key(&c.key))
                        .map(|c| c.key.clone())
                        .collect();
                    if new.is_empty() {
                        break;
                    }
                    any = true;
                    for k in new {
                        s.choices.insert(k, v.to_string());
                    }
                }
                assert!(any, "{label}: no question offers {v}");
            }
            _ => panic!("{label}: unknown setting {p}"),
        }
    }
    s
}

#[test]
fn every_example_gives_what_it_says() {
    for p in all_rules() {
        let id = p.str("id");
        let f = p.file.display().to_string();
        let mut previous: Option<&Example> = None;
        for ex in &p.examples {
            let label = format!("{f}: example `{} {}`", ex.lang, ex.role);
            match ex.role.as_str() {
                "follows" | "violates" => {
                    let doc = Document::parse(&ex.text, kind_of(&ex.lang))
                        .unwrap_or_else(|e| panic!("{label}: {e:?}"));
                    let problems = check(&doc);
                    let ruled: Vec<(&str, &str)> = problems
                        .iter()
                        .filter(|q| !q.rule.is_empty())
                        .map(|q| (q.rule, q.message.as_str()))
                        .collect();
                    if ex.role == "follows" {
                        // not this rule, and no errors or warnings of any (other rules' hints may stay:
                        // an entity type depending on itself always gets one, ER-28)
                        let bad: Vec<&(&str, &str)> = problems
                            .iter()
                            .zip(&ruled)
                            .filter(|(q, (r, _))| *r == id || q.severity != Severity::Hint)
                            .map(|(_, r)| r)
                            .collect();
                        assert!(bad.is_empty(), "{label}: gives {bad:?}");
                    } else {
                        assert_eq!(ex.rule, id, "{label}: names its own rule");
                        assert!(
                            ruled.iter().any(|(r, _)| *r == id),
                            "{label}: doesn't give {id}, gives {ruled:?}"
                        );
                    }
                }
                "allowed" | "refused" => {
                    let mut doc = Document::parse(&ex.text, kind_of(&ex.lang))
                        .unwrap_or_else(|e| panic!("{label}: {e:?}"));
                    let at =
                        doc.elements.iter().rposition(|e| matches!(e.body, Body::Edge(_))).expect("a line");
                    let Body::Edge(e) = doc.elements.remove(at).body else { unreachable!() };
                    let why = refusal(&doc, &e.command, e.src_id(), e.dst_id());
                    if ex.role == "allowed" {
                        assert!(why.is_none(), "{label}: refused: {why:?}");
                    } else {
                        assert_eq!(ex.rule, id, "{label}: names its own rule");
                        assert!(
                            matches!(&why, Some((r, _)) if *r == id),
                            "{label}: not refused by {id}: {why:?}"
                        );
                    }
                }
                // drawn only (the symbols of a notation)
                "shows" => {
                    Document::parse(&ex.text, kind_of(&ex.lang)).unwrap_or_else(|e| panic!("{label}: {e:?}"));
                }
                "converts" => {}
                "result" => {
                    let given = previous
                        .filter(|g| g.role == "converts")
                        .unwrap_or_else(|| panic!("{label}: no `converts` before it"));
                    match (given.lang.as_str(), ex.lang.as_str()) {
                        ("gsmd", "erd") => {
                            let gsm = Document::parse(&given.text, Kind::Gsm).unwrap();
                            let got = convert(&gsm, &conversion_settings(&gsm, &given.params, &label), None)
                                .unwrap()
                                .doc;
                            let want = Document::parse(&ex.text, Kind::Er).unwrap();
                            assert_eq!(signature(&got), signature(&want), "{label}");
                        }
                        ("erd", "relations") => {
                            let er = Document::parse(&given.text, Kind::Er).unwrap();
                            let schema = transform(&er, &Settings::default(), &Overrides::new());
                            let text = to_text(&schema, Marks::Underscore, false, "");
                            let lines: BTreeSet<&str> = text.lines().map(str::trim).collect();
                            for want in ex.text.lines().map(str::trim).filter(|l| !l.is_empty()) {
                                assert!(lines.contains(want), "{label}: no line {want:?} in\n{text}");
                            }
                        }
                        other => panic!("{label}: no conversion {other:?}"),
                    }
                }
                other => panic!("{label}: unknown role {other:?}"),
            }
            previous = Some(ex);
        }
    }
}

/// Rule ids written in the core's sources: `"ER-1"` and the like.
fn ids_in_code() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut dirs = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    while let Some(dir) = dirs.pop() {
        for f in fs::read_dir(dir).unwrap().flatten().map(|e| e.path()) {
            if f.is_dir() {
                dirs.push(f);
                continue;
            }
            if f.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = fs::read_to_string(&f).unwrap();
            for part in text.split('"').skip(1).step_by(2) {
                if let Some((prefix, number)) = part.split_once('-')
                    && ["ER", "GSM", "DRAW", "G2E", "REL", "NOT"].contains(&prefix)
                    && !number.is_empty()
                    && number.chars().all(|c| c.is_ascii_digit())
                {
                    out.insert(part.to_string());
                }
            }
        }
    }
    out
}

#[test]
fn every_rule_in_the_code_has_its_page_and_every_check_is_in_the_code() {
    let pages: BTreeMap<String, String> =
        all_rules().iter().map(|p| (p.str("id").to_string(), p.str("kind").to_string())).collect();
    let used = ids_in_code();
    for id in &used {
        assert!(pages.contains_key(id), "the code names rule {id}, which has no page");
    }
    for (id, kind) in &pages {
        if matches!(kind.as_str(), "error" | "warning" | "hint" | "refused") {
            assert!(used.contains(id), "rule {id} ({kind}) isn't named by any check in the code");
        }
    }
}

/// The examples are drawn in the window: nothing in them overlaps, and no line runs through an element
/// (the arranging figures, measured on the drawing).
#[test]
fn every_example_is_drawn_cleanly() {
    let mut bad = Vec::new();
    for p in all_rules() {
        // two elements of one id are drawn where the layout finds that id: overlapping, by nature
        if matches!(p.str("id"), "ER-2" | "GSM-2") {
            continue;
        }
        for ex in p.examples.iter().filter(|e| e.lang != "relations") {
            let doc = Document::parse(&ex.text, kind_of(&ex.lang)).unwrap();
            let f = diagram_core::arrange::figures(&doc, diagram_core::style::LabelLanguage::English, None);
            if f.overlaps > 0 || f.through > 0 || f.close > 0 {
                bad.push(format!(
                    "{} `{} {}`: {} overlaps, {} lines through elements, {} lines on top of one another",
                    p.str("id"),
                    ex.lang,
                    ex.role,
                    f.overlaps,
                    f.through,
                    f.close
                ));
            }
        }
    }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}
