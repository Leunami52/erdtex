//! Every attribute of an entity type (ER) or an object type (GSM): its own and those it inherits through
//! generalisations, each with where it is defined and the chains it is inherited along.
//!
//! Each subtype inherits all the attributes of its supertypes: in ER a subtype inherits its supertypes' attributes through IS_A and CAN_BE, in GSM a subtype through
//! generalization and specialization edges (from the subtype to its supertype), transitively. A subtype of
//! several supertypes (a shared subclass) inherits from all of them; an attribute reached along several
//! chains is listed once, with all its chains.

use std::collections::HashSet;

use crate::document::{Document, Kind};
use crate::latex_text::plain_text;
use crate::model::{Body, ElementKey, endpoint_id, flatten_items};

/// One step of an inheritance chain: the supertype reached, and how (`IS_A`, `CAN_BE`, `generalization`,
/// `specialization`).
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub id: String,
    pub label: String,
    pub via: String,
}

/// Where an attribute is defined, for editing it there: an element, and in an attribute list the item's
/// row (top to bottom, nested ones included).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Source {
    pub element: ElementKey,
    pub row: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Attr {
    /// as written (LaTeX); in GSM the attribute line's label
    pub name: String,
    /// nesting in a composite attribute
    pub depth: usize,
    pub key: bool,
    /// the key of a weak entity type (drawn dashed)
    pub weak_key: bool,
    pub unique: bool,
    pub multivalued: bool,
    pub derived: bool,
    /// ER: the data type, if set; GSM: the type the attribute line leads to
    pub r#type: String,
    /// the element that defines it
    pub owner: String,
    pub owner_label: String,
    /// the chains it is inherited along, each from the element itself to `owner` (empty: its own)
    pub chains: Vec<Vec<Step>>,
    pub source: Source,
}

impl Attr {
    pub fn inherited(&self) -> bool {
        !self.chains.is_empty()
    }
}

/// Whether an element has attributes it could inherit: an entity type (ER), an abstract type or a subtype
/// (GSM).
pub fn eligible(doc: &Document, id: &str) -> bool {
    doc.nodes().any(|n| {
        n.id == id
            && n.generalisation.is_none()
            && match doc.kind {
                Kind::Er => n.command == "entity",
                Kind::Gsm => matches!(n.command.as_str(), "abstract" | "subtype"),
            }
    })
}

/// An element's name on one line.
pub fn label_of(doc: &Document, id: &str) -> String {
    doc.nodes()
        .find(|n| n.id == id)
        .and_then(|n| n.label.as_deref())
        .map(|l| plain_text(&l.replace("\\\\", " ")).split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| id.to_string())
}

/// The direct supertypes of an element, with how it inherits from each.
fn supertypes(doc: &Document, id: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    match doc.kind {
        Kind::Er => {
            for g in doc.nodes() {
                let Some(isa) = &g.generalisation else { continue };
                let sup = endpoint_id(&isa.supertype);
                let linked = doc.edges().any(|e| {
                    (e.src_id() == g.id && e.dst_id() == id) || (e.dst_id() == g.id && e.src_id() == id)
                });
                if linked && sup != id {
                    let via = if g.command == "canbe" { "CAN_BE" } else { "IS_A" };
                    out.push((sup.to_string(), via.to_string()));
                }
            }
        }
        Kind::Gsm => {
            for e in doc.edges() {
                if matches!(e.command.as_str(), "generalization" | "specialization") && e.src_id() == id {
                    out.push((e.dst_id().to_string(), e.command.clone()));
                }
            }
        }
    }
    out
}

/// Every chain from `id` up to each of its supertypes, direct or not (no cycles).
fn chains(doc: &Document, id: &str) -> Vec<Vec<Step>> {
    fn walk(
        doc: &Document,
        at: &str,
        path: &mut Vec<Step>,
        seen: &mut Vec<String>,
        out: &mut Vec<Vec<Step>>,
    ) {
        for (sup, via) in supertypes(doc, at) {
            if seen.contains(&sup) {
                continue;
            }
            path.push(Step { label: label_of(doc, &sup), id: sup.clone(), via });
            seen.push(sup.clone());
            out.push(path.clone());
            walk(doc, &sup, path, seen, out);
            seen.pop();
            path.pop();
        }
    }
    let mut out = Vec::new();
    walk(doc, id, &mut Vec::new(), &mut vec![id.to_string()], &mut out);
    out
}

/// Whether a weak entity type depends through an existence dependency (E) only: every weak relationship
/// linked to it by a double line is an E.
fn existence_only(doc: &Document, entity: &str) -> bool {
    let deps: Vec<&str> = doc
        .edges()
        .filter(|e| e.opts.is_on("double") && (e.src_id() == entity || e.dst_id() == entity))
        .filter_map(|e| {
            let other = if e.src_id() == entity { e.dst_id() } else { e.src_id() };
            doc.nodes().find(|n| n.id == other && n.command == "relationship")
        })
        .map(|n| n.opts.get("dependency").map(crate::text::strip).unwrap_or(""))
        .collect();
    !deps.is_empty() && deps.iter().all(|d| *d == "E")
}

/// The attributes an element defines itself, in the order of the file.
fn own(doc: &Document, owner: &str) -> Vec<Attr> {
    let owner_label = label_of(doc, owner);
    // dashed as in the drawing: the key of a weak entity type, unless it depends by existence alone
    // (then its key is unique on its own)
    let weak = doc.nodes().any(|n| n.id == owner && n.opts.is_on("weak")) && !existence_only(doc, owner);
    let blank = |element: ElementKey| Attr {
        name: String::new(),
        depth: 0,
        key: false,
        weak_key: false,
        unique: false,
        multivalued: false,
        derived: false,
        r#type: String::new(),
        owner: owner.to_string(),
        owner_label: owner_label.clone(),
        chains: vec![],
        source: Source { element, row: None },
    };
    let mut out = Vec::new();
    match doc.kind {
        Kind::Er => {
            for el in &doc.elements {
                match &el.body {
                    Body::Attributes(t) if endpoint_id(&t.node) == owner => {
                        for (row, (it, depth)) in flatten_items(&t.items).into_iter().enumerate() {
                            out.push(Attr {
                                name: it.text.clone(),
                                depth,
                                key: it.key,
                                weak_key: it.key && weak,
                                unique: it.unique,
                                multivalued: it.multivalued,
                                derived: it.derived,
                                r#type: it.r#type.clone().unwrap_or_default(),
                                source: Source { element: el.key, row: Some(row) },
                                ..blank(el.key)
                            });
                        }
                    }
                    _ => {}
                }
            }
            // Chen attributes (`of=`), their parts nested below them
            fn chen(
                doc: &Document,
                of: &str,
                depth: usize,
                weak: bool,
                blank: &dyn Fn(ElementKey) -> Attr,
                out: &mut Vec<Attr>,
            ) {
                for el in &doc.elements {
                    let Body::Node(n) = &el.body else { continue };
                    if n.command != "attribute" || n.opts.get("of").map(endpoint_id) != Some(of) {
                        continue;
                    }
                    let key = n.opts.contains("key");
                    out.push(Attr {
                        name: n.label.clone().unwrap_or_default(),
                        depth,
                        key,
                        weak_key: key && weak,
                        multivalued: n.opts.contains("multivalued"),
                        derived: n.opts.contains("derived"),
                        r#type: n.opts.get("type").unwrap_or("").to_string(),
                        ..blank(el.key)
                    });
                    if depth < 20 {
                        chen(doc, &n.id, depth + 1, weak, blank, out);
                    }
                }
            }
            chen(doc, owner, 0, weak, &blank, &mut out);
        }
        Kind::Gsm => {
            for el in &doc.elements {
                let Body::Edge(e) = &el.body else { continue };
                if matches!(e.command.as_str(), "attribute" | "multiattribute") && e.src_id() == owner {
                    out.push(Attr {
                        name: e.label.clone().unwrap_or_default(),
                        multivalued: e.command == "multiattribute",
                        r#type: label_of(doc, e.dst_id()),
                        ..blank(el.key)
                    });
                }
            }
        }
    }
    out
}

/// Every attribute of an element: its own first, then those it inherits, nearest supertypes first; each
/// supertype's attributes once, with every chain it is reached along.
pub fn all_attributes(doc: &Document, id: &str) -> Vec<Attr> {
    let mut out = own(doc, id);
    let mut all = chains(doc, id);
    all.sort_by_key(Vec::len); // stable: the nearest supertypes first, in the file's order
    let mut done: HashSet<String> = HashSet::new();
    for chain in &all {
        let Some(owner) = chain.last().map(|s| s.id.clone()) else { continue };
        if !done.insert(owner.clone()) {
            continue;
        }
        let along: Vec<Vec<Step>> =
            all.iter().filter(|c| c.last().is_some_and(|s| s.id == owner)).cloned().collect();
        for mut a in own(doc, &owner) {
            a.chains = along.clone();
            out.push(a);
        }
    }
    out
}

/// A chain as text: `Pilot → Employee (IS_A) → Person (IS_A)`.
pub fn chain_text(doc: &Document, id: &str, chain: &[Step]) -> String {
    let mut s = label_of(doc, id);
    for step in chain {
        s.push_str(&format!(" → {} ({})", step.label, step.via));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn er(text: &str) -> Document {
        Document::parse(text, Kind::Er).unwrap()
    }

    const AIRLINE_LIKE: &str = "\\entity{person}(0,0){Person}\n\\attributes{person}{*SSN, Name{First, Last}}\n\
        \\isa{isa1}(3,0){person}{co}\n\\entity{employee}(6,0){Employee}\n\\attributes{employee}{Salary}\n\
        \\link{employee}{isa1}\n\\isa{isa2}(9,0){employee}{cd}\n\\entity{pilot}(12,0){Pilot}\n\\link{pilot}{isa2}\n\
        \\entity{customer}(0,-4){Customer}\n\\attributes{customer}{*CustNo}\n\\isa{isa3}(3,-4){customer}{co}\n\
        \\link{pilot}{isa3}\n";

    #[test]
    fn a_subtype_inherits_along_the_chain_nearest_first() {
        let d = er(AIRLINE_LIKE);
        let all = all_attributes(&d, "pilot");
        let names: Vec<(&str, &str, usize)> =
            all.iter().map(|a| (a.name.as_str(), a.owner.as_str(), a.chains.len())).collect();
        // the pilot has none of its own; Employee and Customer are its direct supertypes, Person above
        assert_eq!(
            names,
            vec![
                ("Salary", "employee", 1),
                ("CustNo", "customer", 1),
                ("SSN", "person", 1),
                ("Name", "person", 1),
                ("First", "person", 1),
                ("Last", "person", 1),
            ]
        );
        let ssn = &all[2];
        assert!(ssn.key && ssn.inherited());
        assert_eq!(chain_text(&d, "pilot", &ssn.chains[0]), "Pilot → Employee (IS_A) → Person (IS_A)");
        assert_eq!(all[4].depth, 1);
        assert_eq!(ssn.source.row, Some(0));
        assert_eq!(all[5].source.row, Some(3)); // Last: the fourth row of Person's list
    }

    #[test]
    fn a_supertype_reached_twice_is_listed_once_with_both_chains() {
        let text =
            format!("{AIRLINE_LIKE}\\isa{{isa4}}(6,-4){{customer}}{{co}}\n\\link{{employee}}{{isa4}}\n");
        let d = er(&text);
        let all = all_attributes(&d, "pilot");
        let custno: Vec<&Attr> = all.iter().filter(|a| a.name == "CustNo").collect();
        assert_eq!(custno.len(), 1);
        assert_eq!(custno[0].chains.len(), 2);
    }

    #[test]
    fn own_attributes_come_first_chen_ones_included_and_cycles_end() {
        let text = "\\entity[weak]{e}(0,0){E}\n\\attributes{e}{*Nr}\n\\attribute[of=e, key]{a}(2,2){Code}\n\
            \\isa{g}(0,3){e}{co}\n\\entity{f}(0,6){F}\n\\link{f}{g}\n\\isa{h}(0,9){f}{co}\n\\link{e}{h}\n";
        let d = er(text);
        let all = all_attributes(&d, "e");
        assert_eq!(all.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), vec!["Nr", "Code"]);
        assert!(all.iter().all(|a| a.key && a.weak_key && !a.inherited()));
        // F inherits from E; E from F (a cycle, refused by the rules) gives nothing more
        assert_eq!(all_attributes(&d, "f").len(), 2);
    }

    #[test]
    fn a_gsm_subtype_inherits_its_supertypes_attribute_lines() {
        let text = "\\abstract{person}(0,0){Person}\n\\representable{pname}(3,0){PName}\n\
            \\attribute{person}{pname}{has name}\n\\subtype{student}(0,-3){Student}\n\
            \\representable{field}(3,-3){Field}\n\\attribute{student}{field}{main field}\n\
            \\specialization{student}{person}\n\\subtype{phd}(0,-6){PhD student}\n\\generalization{phd}{student}\n";
        let d = Document::parse(text, Kind::Gsm).unwrap();
        assert!(eligible(&d, "phd") && eligible(&d, "person") && !eligible(&d, "pname"));
        let all = all_attributes(&d, "phd");
        let rows: Vec<(&str, &str, &str)> =
            all.iter().map(|a| (a.name.as_str(), a.r#type.as_str(), a.owner.as_str())).collect();
        assert_eq!(rows, vec![("main field", "Field", "student"), ("has name", "PName", "person")]);
        assert_eq!(
            chain_text(&d, "phd", &all[1].chains[0]),
            "PhD student → Student (generalization) → Person (specialization)"
        );
    }
}
