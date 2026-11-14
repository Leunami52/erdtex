//! A diagram file (.erd / .gsmd): parsing, access to its elements, editing and writing.
//!
//! Which commands exist, which arguments they take and what they become is described in one
//! table per kind of diagram (`ER_COMMANDS`, `GSM_COMMANDS`, `COMMON_COMMANDS`): a new
//! command is a new row.

use std::collections::BTreeSet;

use crate::model::{
    AttrItem, AttributeTree, Body, Coord, Edge, Element, ElementKey, Generalisation, Grid, Meta, Node, Note,
    Options, Point, Raw, Setup, Subdomain, parse_coord_list,
};
use crate::texscan::{Arg, ParseError, Scanner, split_tex, strip_braces};
use crate::text::{slice, strip};

/// The two kinds of diagrams.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Er,
    Gsm,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Er => "er",
            Kind::Gsm => "gsm",
        }
    }

    pub fn from_name(name: &str) -> Option<Kind> {
        match name {
            "er" => Some(Kind::Er),
            "gsm" => Some(Kind::Gsm),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Kind::Er => ".erd",
            Kind::Gsm => ".gsmd",
        }
    }

    fn commands(self) -> &'static [CommandSpec] {
        match self {
            Kind::Er => ER_COMMANDS,
            Kind::Gsm => GSM_COMMANDS,
        }
    }
}

/// What a command becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementType {
    /// a node with a label
    Node,
    /// a node without a label argument (PART_OF)
    NodeWithoutLabel,
    /// IS_A / CAN_BE: a node with a supertype and a type
    Generalisation,
    Edge,
    Attributes,
    Point,
    Note,
    Subdomain,
    Grid,
    Setup,
}

/// A command of the file format: its name, its arguments (xparse letters, see `Arg`) and
/// what it becomes.
#[derive(Clone, Copy, Debug)]
pub struct CommandSpec {
    pub name: &'static str,
    pub signature: &'static str,
    pub element: ElementType,
}

const fn spec(name: &'static str, signature: &'static str, element: ElementType) -> CommandSpec {
    CommandSpec { name, signature, element }
}

/// Commands of both kinds.
pub const COMMON_COMMANDS: &[CommandSpec] = &[
    spec("point", "md", ElementType::Point),
    spec("note", "Odm", ElementType::Note),
    spec("subdomain", "Omm", ElementType::Subdomain),
    spec("grid", "rr", ElementType::Grid),
    spec("setup", "m", ElementType::Setup),
];

pub const ER_COMMANDS: &[CommandSpec] = &[
    spec("entity", "Omdm", ElementType::Node),
    spec("relationship", "Omdm", ElementType::Node),
    spec("attribute", "Omdm", ElementType::Node),
    spec("partof", "Omd", ElementType::NodeWithoutLabel),
    spec("isa", "Omdmg", ElementType::Generalisation),
    spec("canbe", "Omdmg", ElementType::Generalisation),
    spec("attributes", "Omm", ElementType::Attributes),
    spec("link", "Ommg", ElementType::Edge),
];

pub const GSM_COMMANDS: &[CommandSpec] = &[
    spec("abstract", "Omdm", ElementType::Node),
    spec("representable", "Omdm", ElementType::Node),
    spec("subtype", "Omdm", ElementType::Node),
    spec("aggregation", "Omdm", ElementType::Node),
    spec("grouping", "Omdm", ElementType::Node),
    spec("attribute", "Ommg", ElementType::Edge),
    spec("multiattribute", "Ommg", ElementType::Edge),
    spec("component", "Ommg", ElementType::Edge),
    spec("generalization", "Ommg", ElementType::Edge),
    spec("specialization", "Ommg", ElementType::Edge),
    spec("link", "Ommg", ElementType::Edge),
];

/// Raw TikZ commands that end with ';'.
const TIKZ_PATH: &[&str] = &[
    "draw",
    "node",
    "path",
    "fill",
    "filldraw",
    "coordinate",
    "shade",
    "shadedraw",
    "clip",
    "pic",
    "matrix",
    "foreach",
];

/// The command `name` in a diagram of this kind.
pub fn command_spec(kind: Kind, name: &str) -> Option<&'static CommandSpec> {
    kind.commands().iter().chain(COMMON_COMMANDS).find(|c| c.name == name)
}

/// The commands that draw nodes (IS_A and PART_OF included), in table order.
pub fn node_commands(kind: Kind) -> impl Iterator<Item = &'static str> {
    kind.commands()
        .iter()
        .filter(|c| {
            matches!(
                c.element,
                ElementType::Node | ElementType::NodeWithoutLabel | ElementType::Generalisation
            )
        })
        .map(|c| c.name)
}

/// The commands that draw edges, in table order.
pub fn edge_commands(kind: Kind) -> impl Iterator<Item = &'static str> {
    kind.commands().iter().filter(|c| c.element == ElementType::Edge).map(|c| c.name)
}

/// The kind a file without a known extension most likely is.
pub fn guess_kind(text: &str) -> Kind {
    const GSM_WORDS: &[&str] = &[
        "\\abstract",
        "\\representable",
        "\\subtype",
        "\\aggregation",
        "\\grouping",
        "\\multiattribute",
        "\\component",
        "\\generalization",
        "\\specialization",
    ];
    if GSM_WORDS.iter().any(|w| text.contains(w)) { Kind::Gsm } else { Kind::Er }
}

/// The names of all `\command`s in `text` (ASCII letters, as `\\([A-Za-z]+)` finds them).
fn command_names(text: &str) -> BTreeSet<&str> {
    let mut out = BTreeSet::new();
    let b = text.as_bytes();
    let mut i = 0;
    while let Some(&c) = b.get(i) {
        if c == b'\\' {
            let n = b.iter().skip(i + 1).take_while(|x| x.is_ascii_alphabetic()).count();
            if n > 0 {
                out.insert(slice(text, i + 1, i + 1 + n));
            }
            i += 1 + n;
        } else {
            i += 1;
        }
    }
    out
}

/// Which kind of diagram this text can only come from; `None`: can't tell, or both.
pub fn kind_of_text(text: &str) -> Option<Kind> {
    let names = command_names(text);
    let only = |kind: Kind, other: Kind| {
        names.iter().any(|n| command_spec(kind, n).is_some() && command_spec(other, n).is_none())
    };
    match (only(Kind::Er, Kind::Gsm), only(Kind::Gsm, Kind::Er)) {
        (true, false) => Some(Kind::Er),
        (false, true) => Some(Kind::Gsm),
        _ => None,
    }
}

/// The kind of a file: from its extension (".erd", ".gsmd"), else from its text.
pub fn kind_for(extension: Option<&str>, text: &str) -> Kind {
    match extension {
        Some(".gsmd") => Kind::Gsm,
        Some(".erd") => Kind::Er,
        _ => guess_kind(text),
    }
}

/// Files are read with "\n" line breaks only.
pub fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

/// The text to write over a file that had "\r\n" line breaks (`crlf`), or not.
pub fn with_line_endings(text: &str, crlf: bool) -> String {
    if crlf { normalize_newlines(text).replace('\n', "\r\n") } else { text.to_string() }
}

/// A diagram.
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub kind: Kind,
    pub elements: Vec<Element>,
    /// white space and comments after the last element
    pub tail: String,
    next_key: u64,
}

impl Document {
    pub fn new(kind: Kind) -> Self {
        Document { kind, elements: Vec::new(), tail: "\n".into(), next_key: 1 }
    }

    fn new_key(&mut self) -> ElementKey {
        let key = ElementKey(self.next_key);
        self.next_key += 1;
        key
    }

    // ------------------------------------------------------------------ reading
    pub fn parse(text: &str, kind: Kind) -> Result<Document, ParseError> {
        let mut doc = Document::new(kind);
        let mut sc = Scanner::new(text);
        loop {
            let trivia = sc.skip_trivia();
            if sc.eof() {
                doc.tail = trivia.to_string();
                break;
            }
            let start = sc.pos;
            let body = parse_statement(kind, &mut sc)?;
            let source = slice(text, start, sc.pos).to_string();
            let trailing = sc.read_line_rest().to_string();
            let key = doc.new_key();
            doc.elements.push(Element {
                key,
                meta: Meta { source: Some(source), dirty: false, leading: trivia.to_string(), trailing },
                body,
            });
        }
        Ok(doc)
    }

    // ------------------------------------------------------------------ writing
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for e in &self.elements {
            out += &e.meta.leading;
            out += &e.text();
            out += &e.meta.trailing;
        }
        out += &self.tail;
        out
    }

    /// After saving: what was written becomes the elements' source.
    pub fn mark_saved(&mut self) {
        for e in &mut self.elements {
            e.meta.source = Some(e.text());
            e.meta.dirty = false;
        }
    }

    pub fn modified(&self) -> bool {
        self.elements.iter().any(|e| e.meta.dirty)
    }

    // ------------------------------------------------------------------ access
    pub fn get(&self, key: ElementKey) -> Option<&Element> {
        self.elements.iter().find(|e| e.key == key)
    }

    pub fn get_mut(&mut self, key: ElementKey) -> Option<&mut Element> {
        self.elements.iter_mut().find(|e| e.key == key)
    }

    fn index_of(&self, key: ElementKey) -> Option<usize> {
        self.elements.iter().position(|e| e.key == key)
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.elements.iter().filter_map(Element::as_node)
    }

    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.elements.iter().filter_map(Element::as_edge)
    }

    pub fn points(&self) -> impl Iterator<Item = &Point> {
        self.elements.iter().filter_map(|e| match &e.body {
            Body::Point(p) => Some(p),
            _ => None,
        })
    }

    /// The attribute lists, all or those of one node.
    pub fn attribute_trees<'a>(
        &'a self,
        node_id: Option<&'a str>,
    ) -> impl Iterator<Item = &'a AttributeTree> {
        self.elements.iter().filter_map(move |e| match &e.body {
            Body::Attributes(t) if node_id.is_none_or(|id| t.node == id) => Some(t),
            _ => None,
        })
    }

    /// The node or point with this id (an anchor suffix is ignored).
    pub fn by_id(&self, ident: &str) -> Option<&Element> {
        let ident = ident.split('.').next().unwrap_or("");
        self.elements.iter().find(|e| e.id() == Some(ident))
    }

    pub fn ids(&self) -> BTreeSet<&str> {
        self.elements.iter().filter_map(Element::id).collect()
    }

    /// An id not in use yet, made from `base`.
    pub fn unique_id(&self, base: &str) -> String {
        let base: String =
            base.to_lowercase().chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
        let base = if base.is_empty() { "n".to_string() } else { base };
        let ids = self.ids();
        let mut candidate = base.clone();
        let mut k = 1;
        while ids.contains(candidate.as_str()) {
            k += 1;
            candidate = format!("{base}{k}");
        }
        candidate
    }

    /// Whether the diagram prints data types (`\setup{show types}`).
    pub fn types_shown(&self) -> bool {
        self.elements
            .iter()
            .find_map(|e| match &e.body {
                Body::Setup(s) if s.opts.contains("show types") => {
                    Some(matches!(s.opts.get("show types"), None | Some("true")))
                }
                _ => None,
            })
            .unwrap_or(false)
    }

    pub fn set_types_shown(&mut self, on: bool) {
        let setup = self.elements.iter().find(|e| matches!(e.body, Body::Setup(_))).map(|e| e.key);
        match (on, setup) {
            (true, None) => {
                let key = self.new_key();
                let mut opts = Options::new();
                opts.set("show types", None);
                self.elements
                    .insert(0, Element { key, meta: Meta::default(), body: Body::Setup(Setup { opts }) });
            }
            (true, Some(key)) => {
                if let Some(e) = self.get_mut(key) {
                    if let Body::Setup(s) = &mut e.body {
                        s.opts.set("show types", None);
                    }
                    e.touch();
                }
            }
            (false, Some(key)) => {
                let mut empty = false;
                if let Some(e) = self.get_mut(key) {
                    if let Body::Setup(s) = &mut e.body {
                        s.opts.remove("show types");
                        empty = s.opts.is_empty();
                    }
                    e.touch();
                }
                if empty {
                    self.remove(key);
                }
            }
            (false, None) => {}
        }
    }

    // ------------------------------------------------------------------ editing
    /// Adds an element after `after` (default: at the end), on a line of its own.
    pub fn add(&mut self, body: Body, after: Option<ElementKey>) -> ElementKey {
        let index = after.and_then(|k| self.index_of(k)).or(self.elements.len().checked_sub(1));
        if let Some(prev) = index.and_then(|i| self.elements.get_mut(i))
            && !prev.meta.trailing.ends_with('\n')
        {
            prev.meta.trailing.push('\n'); // the new element starts on a new line
        }
        let key = self.new_key();
        let at = index.map_or(0, |i| i + 1);
        self.elements.insert(at, Element { key, meta: Meta::default(), body });
        key
    }

    /// Inserts a new element at a position (0: first), on a line of its own; the elements
    /// around it are not changed.
    pub fn insert(&mut self, index: usize, body: Body) -> ElementKey {
        let key = self.new_key();
        let at = index.min(self.elements.len());
        self.elements.insert(at, Element { key, meta: Meta::default(), body });
        key
    }

    /// Removes an element.  Removing a node also removes its edges and attribute lists; the
    /// rest of the file is untouched.  Comment lines above a removed element stay (they often
    /// describe a whole group).
    pub fn remove(&mut self, key: ElementKey) {
        let Some(element) = self.get(key) else { return };
        let mut doomed = vec![key];
        if let Some(ident) = element.id().map(str::to_string) {
            for e in &self.elements {
                let gone = match &e.body {
                    Body::Edge(edge) => edge.src_id() == ident || edge.dst_id() == ident,
                    Body::Attributes(t) => t.node == ident,
                    _ => false,
                };
                if gone {
                    doomed.push(e.key);
                }
            }
        }
        for k in doomed {
            let Some(i) = self.index_of(k) else { continue };
            let removed = self.elements.remove(i);
            match self.elements.get_mut(i) {
                Some(next) => next.meta.leading = removed.meta.leading + &next.meta.leading,
                None => self.tail = removed.meta.leading + &self.tail,
            }
        }
    }

    /// Changes a node's id and every reference to it.
    pub fn rename(&mut self, key: ElementKey, new_id: &str) -> Result<(), String> {
        let Some(old) = self.get(key).and_then(Element::id).map(str::to_string) else {
            return Ok(());
        };
        if new_id == old {
            return Ok(());
        }
        if self.ids().contains(new_id) {
            return Err(format!("id '{new_id}' already exists"));
        }
        let renamed = |reference: &str| -> Option<String> {
            (reference.split('.').next() == Some(old.as_str()))
                .then(|| format!("{new_id}{}", reference.get(old.len()..).unwrap_or("")))
        };
        for e in &mut self.elements {
            let mut changed = false;
            if e.key == key {
                match &mut e.body {
                    Body::Node(n) => n.id = new_id.to_string(),
                    Body::Point(p) => p.id = new_id.to_string(),
                    _ => {}
                }
                changed = true;
            }
            match &mut e.body {
                Body::Edge(edge) => {
                    for end in [&mut edge.src, &mut edge.dst] {
                        if let Some(r) = renamed(end) {
                            *end = r;
                            changed = true;
                        }
                    }
                }
                Body::Attributes(t) if t.node == old => {
                    t.node = new_id.to_string();
                    changed = true;
                }
                Body::Node(Node { generalisation: Some(g), .. }) => {
                    if let Some(r) = renamed(&g.supertype) {
                        g.supertype = r;
                        changed = true;
                    }
                }
                _ => {}
            }
            if changed {
                e.touch();
            }
        }
        Ok(())
    }
}

/// The attribute list's items: `*Key, a{b, c}, ...`.
pub fn parse_items(text: &str) -> Result<Vec<AttrItem>, crate::model::ItemError> {
    split_tex(text, ',')
        .iter()
        .filter(|p| !strip_braces(strip(p)).is_empty())
        .map(|p| AttrItem::parse(p))
        .collect()
}

fn parse_statement(kind: Kind, sc: &mut Scanner<'_>) -> Result<Body, ParseError> {
    let text = sc.text;
    let start = sc.pos;
    if sc.peek() != Some(b'\\') {
        // stray text: keep the rest of the line
        let end =
            text.as_bytes().iter().skip(start).position(|&b| b == b'\n').map_or(text.len(), |k| start + k);
        sc.pos = end;
        return Ok(Body::Raw(Raw { command: String::new(), raw: slice(text, start, end).to_string() }));
    }
    let name = sc.read_name();
    let Some(spec) = command_spec(kind, name) else {
        skip_unknown(sc, name)?;
        return Ok(Body::Raw(Raw { command: name.to_string(), raw: slice(text, start, sc.pos).to_string() }));
    };
    let mut args = Vec::new();
    for letter in spec.signature.chars() {
        match Arg::from_letter(letter) {
            Some(kind) => args.push(sc.read_arg(kind)?),
            None => args.push(None),
        }
    }
    build(spec, &args).map_err(|e| ParseError::new(format!("malformed \\{name}: {e}"), text, start))
}

fn skip_unknown(sc: &mut Scanner<'_>, name: &str) -> Result<(), ParseError> {
    if TIKZ_PATH.contains(&name) {
        return sc.read_until_semicolon();
    }
    if name == "begin" || name == "end" {
        sc.read_arg(Arg::Mandatory)?;
        if name == "begin" {
            sc.read_arg(Arg::Options)?;
        }
        return Ok(());
    }
    // \foo[..]{..}{..}: take the groups that follow directly
    loop {
        if sc.lookahead(b'[').is_some() {
            sc.read_arg(Arg::Options)?;
        } else if sc.lookahead(b'{').is_some() {
            sc.read_arg(Arg::Mandatory)?;
        } else {
            return Ok(());
        }
    }
}

fn build(spec: &CommandSpec, a: &[Option<&str>]) -> Result<Body, crate::model::ItemError> {
    let arg = |i: usize| a.get(i).copied().flatten();
    let text = |i: usize| arg(i).unwrap_or("");
    let id = |i: usize| strip(text(i)).to_string();
    let opts = |i: usize| Options::parse(arg(i));
    let coord = |i: usize| arg(i).map(Coord::parse);
    let command = spec.name.to_string();
    Ok(match spec.element {
        ElementType::Generalisation => Body::Node(Node {
            command,
            opts: opts(0),
            id: id(1),
            pos: coord(2),
            label: None,
            generalisation: Some(Generalisation {
                supertype: id(3),
                constraint: arg(4).filter(|c| !c.is_empty()).map(str::to_string),
            }),
        }),
        ElementType::NodeWithoutLabel => Body::Node(Node {
            command,
            opts: opts(0),
            id: id(1),
            pos: coord(2),
            label: None,
            generalisation: None,
        }),
        ElementType::Node => Body::Node(Node {
            command,
            opts: opts(0),
            id: id(1),
            pos: coord(2),
            label: Some(text(3).to_string()),
            generalisation: None,
        }),
        ElementType::Edge => Body::Edge(Edge {
            command,
            opts: opts(0),
            src: id(1),
            dst: id(2),
            label: arg(3).map(str::to_string),
        }),
        ElementType::Attributes => {
            Body::Attributes(AttributeTree { opts: opts(0), node: id(1), items: parse_items(text(2))? })
        }
        ElementType::Point => Body::Point(Point { id: id(0), pos: coord(1) }),
        ElementType::Note => Body::Note(Note { opts: opts(0), pos: coord(1), content: text(2).to_string() }),
        ElementType::Subdomain => Body::Subdomain(Subdomain {
            opts: opts(0),
            label: text(1).to_string(),
            points: parse_coord_list(text(2)),
        }),
        ElementType::Grid => Body::Grid(Grid { a: Coord::parse(text(0)), b: Coord::parse(text(1)) }),
        ElementType::Setup => Body::Setup(Setup { opts: opts(0) }),
    })
}

/// Whether a file is a diagram: a diagram extension, or at least one diagram command inside.
/// Any text loads (what isn't understood is kept as it is), so loading alone proves nothing.
pub fn is_diagram(extension: Option<&str>, doc: &Document) -> bool {
    let ext = extension.map(str::to_lowercase);
    matches!(ext.as_deref(), Some(".erd" | ".gsmd")) || doc.elements.iter().any(|e| e.editable())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "% header comment\n\\entity{a}(0,0){A}   % keep me\n\\relationship{r}(3,0){R}\n\
                       \\entity{b}(6,0){B}\n\\link{a}{r}{1}\n\\link[bend=10]{r}{b.west}{m}\n\\attributes{a}{*x, y}\n";

    fn doc(text: &str) -> Document {
        Document::parse(text, Kind::Er).unwrap()
    }

    fn key_of(d: &Document, id: &str) -> ElementKey {
        d.by_id(id).unwrap().key
    }

    #[test]
    fn entity_fields() {
        let d = doc("\\entity[weak, color=blue!20]{s}(1.5,-2){Student\\\\x}");
        let n = d.elements[0].as_node().unwrap();
        assert_eq!(
            (n.command.as_str(), n.id.as_str(), n.label.as_deref()),
            ("entity", "s", Some("Student\\\\x"))
        );
        assert_eq!(n.pos, Some(Coord::numeric(1.5, -2.0)));
        assert!(n.opts.contains("weak"));
        assert_eq!(n.opts.get("color"), Some("blue!20"));
    }

    #[test]
    fn optional_trailing_argument_and_par() {
        let d = doc("\\link{a}{r} % comment\n  {mc}\n\\link{b}{r}\n\n{x}");
        let edges: Vec<&Edge> = d.edges().collect();
        assert_eq!(edges[0].label.as_deref(), Some("mc")); // found across a line break
        assert_eq!(edges[1].label, None); // a blank line stops the search
        assert!(matches!(&d.elements[2].body, Body::Raw(r) if r.raw == "{x}"));
    }

    #[test]
    fn nested_and_relative_positions() {
        let d = doc("\\entity{c}($(a)+(2,1)$){C}\n\\entity[right=2cm of a]{b}{B}");
        let n = d.elements[0].as_node().unwrap();
        assert_eq!(n.pos.as_ref().map(Coord::to_tex).as_deref(), Some("($(a)+(2,1)$)"));
        let m = d.elements[1].as_node().unwrap();
        assert_eq!(m.pos, None);
        assert_eq!(m.opts.get("right"), Some("2cm of a"));
    }

    #[test]
    fn attribute_items_and_gsm_attributes() {
        let mut d = doc("\\attributes[anchor=north]{e}{*Key, {a, b}, \\textit{c}}");
        assert!(
            matches!(&d.elements[0].body, Body::Attributes(t) if t.items.len() == 3 && t.items[1].text == "a, b")
        );
        d.elements[0].touch();
        assert_eq!(d.elements[0].text(), "\\attributes[anchor=north]{e}{*Key, {a, b}, \\textit{c}}");
        let g = Document::parse("\\attribute[bend=10]{p}{n}{has name}", Kind::Gsm).unwrap();
        assert!(g.elements[0].as_edge().is_some());
    }

    #[test]
    fn generalisations() {
        let d = doc("\\isa[sub card=m]{i}(0,-2){person}{cd}\n\\canbe{j}(0,0){p}\n\\link{x}{j}");
        let g = d.elements[0].as_node().and_then(|n| n.generalisation.as_ref()).unwrap();
        assert_eq!((g.supertype.as_str(), g.constraint.as_deref()), ("person", Some("cd")));
        let h = d.elements[1].as_node().and_then(|n| n.generalisation.as_ref()).unwrap();
        assert_eq!(h.constraint, None);
    }

    #[test]
    fn raw_tikz_is_kept() {
        let src = "\\draw[red] (0,0) -- (1,{2}) node {;};\n\\entity{a}(0,0){A}";
        let d = doc(src);
        assert!(
            matches!(&d.elements[0].body, Body::Raw(r) if r.raw == "\\draw[red] (0,0) -- (1,{2}) node {;};")
        );
        assert!(!d.elements[0].editable());
        assert_eq!(d.to_text(), src);
    }

    #[test]
    fn errors_have_positions() {
        let e = Document::parse("\\entity{a}(0,0){A}\n\\entity{b}(1,2{B}", Kind::Er).unwrap_err();
        assert_eq!(e.line, 2);
    }

    #[test]
    fn moving_changes_only_that_line() {
        let mut d = doc(SRC);
        let k = key_of(&d, "a");
        if let Some(Body::Node(n)) = d.get_mut(k).map(|e| &mut e.body) {
            n.pos = Some(Coord::numeric(1.25, -0.5));
        }
        d.get_mut(k).unwrap().touch();
        assert_eq!(d.to_text(), SRC.replace("\\entity{a}(0,0){A}", "\\entity{a}(1.25,-0.5){A}"));
    }

    #[test]
    fn rename_updates_references() {
        let mut d = doc(SRC);
        let b = key_of(&d, "b");
        d.rename(b, "bee").unwrap();
        let text = d.to_text();
        assert!(text.contains("\\entity{bee}(6,0){B}"));
        assert!(text.contains("\\link[bend=10]{r}{bee.west}{m}"));
        let a = key_of(&d, "a");
        assert!(d.rename(a, "r").is_err());
    }

    #[test]
    fn removing_a_node_removes_its_edges_and_attributes() {
        let mut d = doc(SRC);
        let a = key_of(&d, "a");
        d.remove(a);
        assert_eq!(
            d.to_text(),
            "% header comment\n\\relationship{r}(3,0){R}\n\\entity{b}(6,0){B}\n\\link[bend=10]{r}{b.west}{m}\n"
        );
    }

    #[test]
    fn adding_after_a_file_without_final_newline() {
        let mut d = doc("\\entity{a}(0,0){A}");
        let n = Node {
            command: "entity".into(),
            id: "b".into(),
            pos: Some(Coord::numeric(1.0, 0.0)),
            label: Some("B".into()),
            ..Default::default()
        };
        d.add(Body::Node(n), None);
        assert_eq!(d.to_text(), "\\entity{a}(0,0){A}\n\\entity{b}(1,0){B}\n");
    }

    #[test]
    fn unique_ids() {
        let d = doc(SRC);
        assert_eq!(d.unique_id("A"), "a2");
        assert_eq!(d.unique_id("!!"), "n");
    }

    #[test]
    fn crlf_is_kept_when_saving() {
        assert_eq!(with_line_endings("a\nb\n", true), "a\r\nb\r\n");
        assert_eq!(normalize_newlines("a\r\nb"), "a\nb");
    }

    #[test]
    fn kinds() {
        assert_eq!(kind_of_text("\\abstract{a}{A}"), Some(Kind::Gsm));
        assert_eq!(kind_of_text("\\link{a}{b}"), None);
        assert_eq!(kind_for(Some(".gsmd"), ""), Kind::Gsm);
        assert_eq!(node_commands(Kind::Er).count(), 6);
        assert_eq!(edge_commands(Kind::Gsm).count(), 6);
    }

    #[test]
    fn types_shown_toggle() {
        let mut d = doc("\\entity{a}(0,0){A}\n");
        d.set_types_shown(true);
        assert!(d.types_shown());
        assert!(d.to_text().starts_with("\\setup{show types}\n"));
        d.set_types_shown(false);
        assert!(!d.types_shown());
        assert_eq!(d.to_text(), "\\entity{a}(0,0){A}\n");
    }
}
