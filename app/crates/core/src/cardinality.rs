//! Cardinalities: the two notations, what a label means, and converting between them.
//!
//! n-notation (1, c, n, cn) is written at the *opposite* end from the entity being counted;
//! min-max, (min,max), at the entity being counted (rule book NOT-1):
//!
//! ```text
//! Country  1 ---<has>--- n  City        each City has exactly one Country
//! Country (1,*) -<has>- (1,1) City      each City takes part exactly once
//! ```
//!
//! `1` means exactly one and `c` at most one.  The label says which
//! notation it uses, so files need no extra information; `\setup{card notation=minmax}` only
//! decides what the editor offers.  For binary relationships the notations convert exactly as
//! long as the bounds fit; for n-ary ones they say different things and cannot
//! be converted at all.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::document::Document;
use crate::latex_text::plain_text;
use crate::model::{Body, Edge, ElementKey};
use crate::text::{py_decimal, strip};

/// `\setup{card notation=...}`: n-notation.
pub const N_NOTATION: &str = "n";
/// `\setup{card notation=minmax}`.
pub const MIN_MAX: &str = "minmax";
/// The labels the editor offers in n-notation.
pub const N_CHOICES: &[&str] = &["1", "c", "n", "cn"];
/// What `·` in the min position stands for.
pub const UNKNOWN_MIN: &str = "·";
/// The commands that are relationships with sides.
pub const RELATIONSHIPS: &[&str] = &["relationship", "partof"];

/// The name of a notation, as the editor shows it.
pub fn notation_name(notation: &str) -> &'static str {
    if notation == MIN_MAX { "min-max notation" } else { "n-notation" }
}

/// A count as written in a label: any number of digits, like Python's integers.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Count(String);

impl Count {
    /// From decimal digits of any script (leading zeros are dropped).
    fn from_digits(digits: &[u32]) -> Count {
        let s: String = digits.iter().filter_map(|d| char::from_digit(*d, 10)).collect();
        let s = s.trim_start_matches('0');
        Count(if s.is_empty() { "0".into() } else { s.into() })
    }
}

impl From<u64> for Count {
    fn from(v: u64) -> Count {
        Count(v.to_string())
    }
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Ord for Count {
    fn cmp(&self, other: &Count) -> Ordering {
        self.0.len().cmp(&other.0.len()).then_with(|| self.0.cmp(&other.0))
    }
}

impl PartialOrd for Count {
    fn partial_cmp(&self, other: &Count) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq<u64> for Count {
    fn eq(&self, other: &u64) -> bool {
        self.0.len() <= 20 && self.0.parse::<u64>() == Ok(*other)
    }
}

impl PartialOrd<u64> for Count {
    fn partial_cmp(&self, other: &u64) -> Option<Ordering> {
        Some(self.cmp(&Count::from(*other)))
    }
}

/// A cardinality label: between `lo` and `hi` (`None` = no limit).
///
/// `here == false`: written at the opposite end from the entity being counted, so it says how
/// many of *this side's* entity one partner has (n-notation, [a,b]).  `here == true`:
/// (min,max), written at the entity being counted, so it says how often one entity of this
/// side takes part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    pub text: String,
    pub lo: Count,
    pub hi: Option<Count>,
    pub known: bool,
    pub here: bool,
    /// `·` in the min position
    pub unknown_min: bool,
}

impl Card {
    fn new(text: &str, lo: u64, hi: Option<u64>) -> Card {
        Card::counts(text, Count::from(lo), hi.map(Count::from))
    }

    fn counts(text: &str, lo: Count, hi: Option<Count>) -> Card {
        Card { text: text.into(), lo, hi, known: true, here: false, unknown_min: false }
    }

    /// A label that isn't understood (or no label: empty text).
    pub fn unknown(text: &str) -> Card {
        Card { known: false, ..Card::new(text, 1, None) }
    }

    /// At most one.
    pub fn single(&self) -> bool {
        self.hi.as_ref().is_some_and(|h| *h == 1)
    }

    /// May be none.
    pub fn optional(&self) -> bool {
        self.lo == 0
    }

    /// 1, c, m or mc.
    pub fn short(&self) -> &'static str {
        match (self.single(), self.optional()) {
            (true, true) => "c",
            (true, false) => "1",
            (false, true) => "mc",
            (false, false) => "m",
        }
    }
}

/// A run of decimal digits at the start of `chars`: their values.
fn digit_run(chars: &[char]) -> Vec<u32> {
    chars.iter().map_while(|c| py_decimal(*c)).collect()
}

/// `\d+|\*|[a-z]` at the start, as the maximum of a label: (count, length).
fn maximum(chars: &[char]) -> Option<(Option<Count>, usize)> {
    let run = digit_run(chars);
    if !run.is_empty() {
        return Some((Some(Count::from_digits(&run)), run.len()));
    }
    match chars.first() {
        Some('*') => Some((None, 1)),
        Some(c) if c.is_ascii_lowercase() => Some((None, 1)),
        _ => None,
    }
}

fn bounds(text: &str, lo: Count, hi: Option<Count>) -> Card {
    if let Some(h) = &hi
        && (*h == 0 || *h < lo)
    {
        return Card::unknown(text); // impossible bounds
    }
    Card::counts(text, lo, hi)
}

/// What a label says.
pub fn parse_card(text: Option<&str>) -> Card {
    let t = plain_text(text.unwrap_or("")).replace(' ', "");
    if t.is_empty() {
        return Card::unknown("");
    }
    let low = t.to_lowercase();
    let many = |c: char| matches!(c, 'm' | 'n' | 'k' | 'p' | 'q');
    let chars: Vec<char> = low.chars().collect();
    match low.as_str() {
        "1" => return Card::new(&t, 1, Some(1)),
        "c" | "?" | "0..1" => return Card::new(&t, 0, Some(1)),
        "+" | "1..*" | "1..n" | "1..m" => return Card::new(&t, 1, None),
        "*" | "0..*" | "0..n" | "0..m" => return Card::new(&t, 0, None),
        _ => {}
    }
    match chars.as_slice() {
        [a] if many(*a) => return Card::new(&t, 1, None),
        [a, 'c'] | ['c', a] if many(*a) => return Card::new(&t, 0, None),
        _ => {}
    }
    // (min,max) and [min,max]
    if let Some(open @ ('(' | '[')) = chars.first() {
        let rest = chars.get(1..).unwrap_or(&[]);
        let run = digit_run(rest);
        let (lo, unknown, used) = if !run.is_empty() {
            (Some(Count::from_digits(&run)), false, run.len())
        } else if rest.first().is_some_and(|c| matches!(c, '.' | '·' | '⋅')) {
            (None, true, 1)
        } else {
            return Card::unknown(&t);
        };
        let rest = rest.get(used..).unwrap_or(&[]);
        if !matches!(rest.first(), Some(',' | ';')) {
            return Card::unknown(&t);
        }
        let rest = rest.get(1..).unwrap_or(&[]);
        let Some((hi, n)) = maximum(rest) else { return Card::unknown(&t) };
        let close = rest.get(n..).unwrap_or(&[]);
        let fits = match close {
            [c @ (')' | ']')] => (*open == '(') == (*c == ')'),
            _ => false,
        };
        if !fits {
            return Card::unknown(&t);
        }
        let mut card = bounds(&t, lo.unwrap_or_else(|| Count::from(0)), hi);
        if card.known {
            card.here = *open == '(';
            card.unknown_min = unknown;
        }
        return card;
    }
    // min..max
    let run = digit_run(&chars);
    if !run.is_empty()
        && chars.get(run.len()..run.len() + 2) == Some(&['.', '.'])
        && let Some((hi, n)) = maximum(chars.get(run.len() + 2..).unwrap_or(&[]))
        && run.len() + 2 + n == chars.len()
    {
        return bounds(&t, Count::from_digits(&run), hi);
    }
    Card::unknown(&t)
}

// ---------------------------------------------------------------------------------------------
//  forms of a label
// ---------------------------------------------------------------------------------------------

/// 1, c, n or cn; `None` if the bounds have no n-notation form.
pub fn n_form(card: &Card) -> Option<&'static str> {
    if !card.known || card.unknown_min || card.lo > 1 {
        return None;
    }
    match (card.lo == 1, &card.hi) {
        (true, Some(h)) if *h == 1 => Some("1"),
        (false, Some(h)) if *h == 1 => Some("c"),
        (true, None) => Some("n"),
        (false, None) => Some("cn"),
        _ => None,
    }
}

/// `(min,max)`.
pub fn min_max_form(card: &Card) -> String {
    let lo = if card.unknown_min { UNKNOWN_MIN.to_string() } else { card.lo.to_string() };
    let hi = card.hi.as_ref().map_or("*".to_string(), Count::to_string);
    format!("({lo},{hi})")
}

/// `N_NOTATION`, `MIN_MAX`, `old` (works, but not offered any more), `invalid` or empty.
pub fn label_notation(card: &Card) -> &'static str {
    if card.text.is_empty() {
        ""
    } else if !card.known {
        "invalid"
    } else if card.here {
        MIN_MAX
    } else if N_CHOICES.contains(&card.text.as_str()) {
        N_NOTATION
    } else {
        "old"
    }
}

/// The crow's foot symbol a label stands for: letters only, as in the package
/// (`[mnkpq]|[mnkpq]c|c[mnkpq]|1|c`).
pub fn crow_kind(label: Option<&str>) -> Option<&'static str> {
    let t = strip(label.unwrap_or("")).to_lowercase();
    let many = |c: char| matches!(c, 'm' | 'n' | 'k' | 'p' | 'q');
    let chars: Vec<char> = t.chars().collect();
    let valid = match chars.as_slice() {
        ['1'] | ['c'] => true,
        [a] => many(*a),
        [a, 'c'] | ['c', a] => many(*a),
        _ => false,
    };
    if !valid {
        return None;
    }
    Some(match t.as_str() {
        "1" => "1",
        "c" => "c",
        _ if t.contains('c') => "cn",
        _ => "n",
    })
}

// ---------------------------------------------------------------------------------------------
//  a relationship and its sides
// ---------------------------------------------------------------------------------------------

/// One side of a relationship.
#[derive(Clone, Debug, PartialEq)]
pub struct Side {
    pub entity: String,
    /// the edge that holds (or would hold) the label
    pub edge: ElementKey,
    pub role: String,
    pub card: Card,
}

/// The `\link`s of a document, both ways: id -> (edge, the other end).
pub struct Links<'a> {
    adj: HashMap<&'a str, Vec<(ElementKey, &'a str)>>,
    edges: HashMap<ElementKey, &'a Edge>,
}

impl<'a> Links<'a> {
    /// The links of `doc`.
    pub fn new(doc: &'a Document) -> Links<'a> {
        let mut adj: HashMap<&str, Vec<(ElementKey, &str)>> = HashMap::new();
        let mut edges = HashMap::new();
        for el in &doc.elements {
            if let Body::Edge(e) = &el.body
                && e.command == "link"
            {
                adj.entry(e.src_id()).or_default().push((el.key, e.dst_id()));
                adj.entry(e.dst_id()).or_default().push((el.key, e.src_id()));
                edges.insert(el.key, e);
            }
        }
        Links { adj, edges }
    }

    /// The links at a node: (edge, the other end).
    pub fn at(&self, ident: &str) -> &[(ElementKey, &'a str)] {
        self.adj.get(ident).map_or(&[], Vec::as_slice)
    }

    /// The edge of a link.
    pub fn edge(&self, key: ElementKey) -> Option<&'a Edge> {
        self.edges.get(&key).copied()
    }
}

/// Walks from `start` along links, through junction points: for each node reached, the path
/// of edges that led there (in the order Python's stack visits them).
pub(crate) fn walk_ends<'a>(
    links: &Links<'a>,
    points: &HashSet<&str>,
    start: &str,
    mut visit: impl FnMut(&'a str, &[ElementKey]),
) {
    for (first, other) in links.at(start) {
        let mut stack: Vec<(&str, Vec<ElementKey>)> = vec![(other, vec![*first])];
        let mut seen: HashSet<&str> = HashSet::from([start]);
        while let Some((node, path)) = stack.pop() {
            if points.contains(node) {
                if seen.insert(node) {
                    for (e, o) in links.at(node) {
                        if path.last() != Some(e) {
                            let mut p = path.clone();
                            p.push(*e);
                            stack.push((o, p));
                        }
                    }
                }
                continue;
            }
            visit(node, &path);
        }
    }
}

/// The label and the role of a path of edges: the last edge that has one.
pub(crate) fn label_and_role<'a>(
    links: &Links<'a>,
    path: &[ElementKey],
) -> (Option<ElementKey>, Option<&'a str>) {
    let labelled = path
        .iter()
        .rev()
        .find(|k| links.edge(**k).and_then(|e| e.label.as_deref()).is_some_and(|l| !strip(l).is_empty()));
    let role = path
        .iter()
        .rev()
        .find_map(|k| links.edge(*k).and_then(|e| e.opts.get("role")).filter(|r| !r.is_empty()));
    (labelled.copied(), role)
}

/// The entities connected to relationship `rid` (also through junction points).
pub fn relationship_sides(doc: &Document, rid: &str) -> Vec<Side> {
    let links = Links::new(doc);
    let points: HashSet<&str> = doc.points().map(|p| p.id.as_str()).collect();
    let mut out = Vec::new();
    walk_ends(&links, &points, rid, |node, path| {
        let Some(n) = doc.by_id(node).and_then(|e| e.as_node()) else { return };
        if n.command != "entity" {
            return;
        }
        let (labelled, role) = label_and_role(&links, path);
        let Some(holder) = labelled.or_else(|| path.last().copied()) else { return };
        let label = links.edge(holder).and_then(|e| e.label.as_deref());
        out.push(Side {
            entity: node.to_string(),
            edge: holder,
            role: plain_text(role.unwrap_or("")),
            card: parse_card(label),
        });
    });
    out
}

/// The relationship whose side `edge` belongs to, if any.
pub fn relationship_of(doc: &Document, edge: ElementKey) -> Option<String> {
    doc.nodes()
        .filter(|n| RELATIONSHIPS.contains(&n.command.as_str()))
        .find(|n| relationship_sides(doc, &n.id).iter().any(|s| s.edge == edge))
        .map(|n| n.id.clone())
}

/// The notation the diagram asks for.
pub fn diagram_notation(doc: &Document) -> &'static str {
    let minmax = doc.elements.iter().any(|el| match &el.body {
        Body::Setup(s) => s.opts.get("card notation") == Some(MIN_MAX),
        _ => false,
    });
    if minmax { MIN_MAX } else { N_NOTATION }
}

/// The notation in use at this relationship: the diagram's unless its labels say otherwise;
/// `mixed` if they use both.
pub fn relationship_notation(doc: &Document, rid: &str) -> &'static str {
    let mut found: Vec<&str> = Vec::new();
    for s in relationship_sides(doc, rid) {
        let f = match label_notation(&s.card) {
            "" | "invalid" => continue,
            "old" => N_NOTATION,
            f => f,
        };
        if !found.contains(&f) {
            found.push(f);
        }
    }
    match found.as_slice() {
        [one] => one,
        [] => diagram_notation(doc),
        _ => "mixed",
    }
}

// ---------------------------------------------------------------------------------------------
//  what a label means
// ---------------------------------------------------------------------------------------------

fn how_many(card: &Card) -> String {
    if card.hi.as_ref() == Some(&card.lo) {
        return if card.lo == 1 { "exactly one".into() } else { format!("exactly {}", card.lo) };
    }
    match &card.hi {
        None if card.unknown_min => "any number (the minimum is unknown)".into(),
        None if card.lo == 0 => "any number (also none)".into(),
        None if card.lo == 1 => "one or more".into(),
        None => format!("at least {}", card.lo),
        Some(h) if card.lo == 0 && *h == 1 => "at most one".into(),
        Some(h) if card.unknown_min => format!("at most {h} (the minimum is unknown)"),
        Some(h) => format!("between {} and {h}", card.lo),
    }
}

fn times(card: &Card) -> String {
    how_many(card)
        .replace("exactly one", "exactly once")
        .replace("at most one", "at most once")
        .replace("one or more", "at least once")
}

fn entity_name(doc: &Document, s: &Side) -> String {
    let label = doc.by_id(&s.entity).and_then(|e| e.as_node()).and_then(|n| n.label.as_deref());
    let name = plain_text(label.unwrap_or(""));
    if name.is_empty() { s.entity.clone() } else { name }
}

/// The name of a relationship as the user sees it.
pub fn relationship_name(doc: &Document, rid: &str) -> String {
    match doc.by_id(rid).and_then(|e| e.as_node()) {
        Some(n) if n.command == "partof" => "PART_OF".into(),
        Some(n) => {
            let name = plain_text(n.label.as_deref().unwrap_or(""));
            if name.is_empty() { rid.to_string() } else { name }
        }
        None => rid.to_string(),
    }
}

/// One sentence about `card` on `side`, with the real names.
pub fn meaning(doc: &Document, rid: &str, side: &Side, card: &Card) -> String {
    let with_role = |s: &Side| {
        entity_name(doc, s) + &if s.role.is_empty() { String::new() } else { format!(" ({})", s.role) }
    };
    let rel_name = relationship_name(doc, rid);
    let sides = relationship_sides(doc, rid);
    let others: Vec<&Side> = sides.iter().filter(|s| s.edge != side.edge).collect();
    let here = with_role(side);
    if card.here {
        let mut text = format!("Each {here} takes part in “{rel_name}” {}.", times(card));
        if sides.len() > 2 {
            text += " This says nothing about combinations of the other sides.";
        }
        return text;
    }
    match others.as_slice() {
        [] => format!("How many {here} one partner has: {}.", how_many(card)),
        [other] if sides.len() == 2 => format!("Each {} has {} {here}.", with_role(other), how_many(card)),
        _ => {
            let combo: Vec<String> = others.iter().map(|s| entity_name(doc, s)).collect();
            format!("For each combination of {} there is {} {here}.", combo.join(", "), how_many(card))
        }
    }
}

// ---------------------------------------------------------------------------------------------
//  conversion
// ---------------------------------------------------------------------------------------------

/// New labels for edges: (edge, label; `None` removes it).
pub type LabelChanges = Vec<(ElementKey, Option<String>)>;

/// The label changes that put relationship `rid` into notation `target`, or why it cannot be
/// converted.
pub fn plan_relationship(doc: &Document, rid: &str, target: &str) -> (LabelChanges, Option<String>) {
    let sides = relationship_sides(doc, rid);
    if let Some(bad) = sides.iter().find(|s| !s.card.text.is_empty() && !s.card.known) {
        return (vec![], Some(format!("the label {} is not valid", bad.card.text)));
    }
    let current = relationship_notation(doc, rid);
    if !sides.iter().any(|s| !s.card.text.is_empty()) || current == target {
        return (vec![], None);
    }
    if current == "mixed" {
        return (vec![], Some("it mixes both notations".into()));
    }
    let [a, b] = sides.as_slice() else {
        return (vec![], Some("the notations say different things for n-ary relationships".into()));
    };
    let labelled = |s: &Side| !s.card.text.is_empty();
    if target == MIN_MAX {
        return (
            vec![
                (a.edge, labelled(b).then(|| min_max_form(&b.card))),
                (b.edge, labelled(a).then(|| min_max_form(&a.card))),
            ],
            None,
        );
    }
    for s in [a, b] {
        if labelled(s) && n_form(&s.card).is_none() {
            return (vec![], Some(format!("{} has no n-notation form", s.card.text)));
        }
    }
    let form = |s: &Side| if labelled(s) { n_form(&s.card).map(str::to_string) } else { None };
    (vec![(a.edge, form(b)), (b.edge, form(a))], None)
}

/// All label changes for switching the diagram to `target`, and what stays as it is
/// (name, reason).
pub fn plan_diagram(doc: &Document, target: &str) -> (LabelChanges, Vec<(String, String)>) {
    let (mut changes, mut skipped) = (Vec::new(), Vec::new());
    for n in doc.nodes().filter(|n| RELATIONSHIPS.contains(&n.command.as_str())) {
        let (ch, why) = plan_relationship(doc, &n.id, target);
        if let Some(why) = why {
            skipped.push((relationship_name(doc, &n.id), why));
        }
        changes.extend(ch);
    }
    (changes, skipped)
}

/// Writes the new labels.
pub fn apply_changes(doc: &mut Document, changes: &LabelChanges) {
    for (key, label) in changes {
        if let Some(el) = doc.get_mut(*key) {
            if let Body::Edge(e) = &mut el.body {
                e.label = label.clone();
            }
            el.touch();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        let c = parse_card(Some("(0,*)"));
        assert!(c.known && c.here && c.optional() && !c.single());
        assert!(!parse_card(Some("(3,1)")).known);
        assert_eq!(parse_card(Some("[2,4]")).hi, Some(Count::from(4)));
        assert!(parse_card(Some("(·,5)")).unknown_min);
        assert_eq!(parse_card(Some("2..*")).lo, Count::from(2));
        assert_eq!(
            parse_card(Some("(0012,99999999999999999999999)")).hi.map(|h| h.to_string()).as_deref(),
            Some("99999999999999999999999")
        );
        assert_eq!(parse_card(Some("(١,٣)")).hi, Some(Count::from(3)));
        assert_eq!(n_form(&parse_card(Some("mc"))), Some("cn"));
        assert_eq!(min_max_form(&parse_card(Some("n"))), "(1,*)");
        assert_eq!(crow_kind(Some(" N ")), Some("n"));
        assert_eq!(crow_kind(Some("(1,1)")), None);
    }
}
