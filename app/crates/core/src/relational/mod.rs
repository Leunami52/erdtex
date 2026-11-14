//! ER diagram -> relational model.
//!
//! One set of defaults.  `\setup{rm rules=db1}`, which versions up to 3.5 offered as a second
//! rule set, is still read and keeps such files' results: its defaults for generalisations (no
//! categorisation) and recursive relationships (a foreign key).  A default
//! only decides the choice a decision starts with; every decision can be overridden on its element.
//!
//! ```text
//! entity type                  -> relation; key attributes = primary key
//! complex-complex (n:n)        -> relation; key = keys of the related entities
//! single-complex (1:n)         -> the single side's key is a foreign key on the complex side
//!                                 (decision: own relation)
//! 1:c                          -> foreign key in the relation of the c side (decision: own relation)
//! 1:1, c:c                     -> foreign key on one side                               (decision)
//! recursive 1:n, 1:1           -> own relation (`rm rules=db1`: foreign key)            (decision)
//! n-ary relationship           -> own relation, key depends on the single sides:
//!     several single sides     -> for each single side, the keys of all other sides are a
//!                                 candidate key, in both rule sets; the keys of the complex sides,
//!                                 or folding into the only complex side, are choices     (decision)
//! relationship with key attributes -> always an own relation; they join its key
//! weak entity, Id dependency   -> owner key becomes part of the key
//! weak entity, E dependency    -> owner key as (mandatory) foreign key
//! generalisation               -> options (1)-(4)                                       (decision)
//!                                 with categorisation (`rm rules=db1`: without)
//! composite attribute          -> its components as columns
//! multi-valued attribute       -> own relation (owner key + value)
//! derived attribute            -> not stored; with a formula, computed by a view
//! ```
//!
//! Foreign key columns: a role on the edge names them (the role alone for a single-column key,
//! role + key name for a composite key).  Without a role, a clash is resolved with the entity's
//! name, or the relationship's name if two foreign keys refer to the same entity.
//!
//! The user's choices are stored as options of the diagram elements (`rm=...`, `rm key=...`,
//! `rm category=...`, `rm name=...`); the LaTeX package accepts and ignores them.

mod format;
mod md5;
pub mod names;
mod transform;

use std::collections::{BTreeMap, BTreeSet};

pub use format::{
    DIALECTS, FORMATS, MARKDOWN_LEGEND, Marks, Quote, dialect_name, md_escape, quote_ident, render, sql_type,
    tex_escape, to_latex, to_markdown, to_sql, to_text,
};
pub use names::{column_ident, guess_type, lower_ident, name_key, relation_ident};
pub use transform::transform;

use crate::rules::Severity;

/// Where a column comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Attribute,
    ForeignKey,
    /// the categorisation attribute of a generalisation
    Category,
    /// a Boolean flag per subtype (option 4)
    Flag,
    /// the value of a multi-valued attribute
    Value,
    /// a key column added to an entity type without key attributes
    Surrogate,
}

impl Origin {
    /// As the Python app names it.
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::Attribute => "attribute",
            Origin::ForeignKey => "fk",
            Origin::Category => "category",
            Origin::Flag => "flag",
            Origin::Value => "value",
            Origin::Surrogate => "surrogate",
        }
    }
}

/// A column of a relation.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub name: String,
    pub ty: String,
    pub pk: bool,
    pub nullable: bool,
    /// the type was guessed from the name
    pub guessed: bool,
    pub origin: Origin,
    /// the allowed values (categorisation)
    pub check_values: Option<Vec<String>>,
    /// a Boolean flag column (default false)
    pub flag: bool,
    /// foreign keys: the prefix used on name clashes
    pub qualifier: String,
    /// foreign keys: the entity referred to
    pub ref_ent: String,
    /// stays the same while the column is renamed and moved
    pub(crate) uid: usize,
}

/// What a relation was made from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationKind {
    Entity,
    Category,
    Relationship,
    Multivalued,
}

impl RelationKind {
    /// As the Python app names it.
    pub fn as_str(self) -> &'static str {
        match self {
            RelationKind::Entity => "entity",
            RelationKind::Category => "category",
            RelationKind::Relationship => "relationship",
            RelationKind::Multivalued => "multivalued",
        }
    }
}

/// A relation's handle in [`Schema::tables`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TableId(pub usize);

/// A foreign key.
#[derive(Clone, Debug, PartialEq)]
pub struct ForeignKey {
    pub columns: Vec<String>,
    pub table: TableId,
    pub ref_columns: Vec<String>,
    pub cascade: bool,
    pub reason: String,
}

/// A relation.
#[derive(Clone, Debug, PartialEq)]
pub struct Relation {
    pub name: String,
    pub kind: RelationKind,
    pub columns: Vec<Column>,
    pub fks: Vec<ForeignKey>,
    /// further candidate keys
    pub uniques: Vec<Vec<String>>,
    /// the ids of the diagram elements it was made from
    pub sources: BTreeSet<String>,
    pub comment: String,
    /// the title of the subdomain box it belongs to
    pub box_title: Option<String>,
}

impl Relation {
    fn new(name: String, kind: RelationKind, sources: &[&str], comment: String) -> Relation {
        Relation {
            name,
            kind,
            columns: vec![],
            fks: vec![],
            uniques: vec![],
            sources: sources.iter().map(|s| s.to_string()).collect(),
            comment,
            box_title: None,
        }
    }

    /// The column of this name (case-insensitive, as SQL compares unquoted names).
    pub fn col(&self, name: &str) -> Option<&Column> {
        self.col_index(name).and_then(|i| self.columns.get(i))
    }

    fn col_index(&self, name: &str) -> Option<usize> {
        let key = name_key(name);
        self.columns.iter().position(|c| name_key(&c.name) == key)
    }

    /// The names of the primary key's columns.
    pub fn pk(&self) -> Vec<String> {
        self.columns.iter().filter(|c| c.pk).map(|c| c.name.clone()).collect()
    }

    /// The names of all columns of foreign keys.
    pub fn fk_columns(&self) -> BTreeSet<String> {
        self.fks.iter().flat_map(|fk| fk.columns.iter().cloned()).collect()
    }
}

/// One way to decide a decision: the element options it sets (`None` removes one).
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub title: String,
    pub assign: Vec<(String, Option<String>)>,
    pub description: String,
    pub valid: bool,
    /// why it isn't valid, or a caveat
    pub reason: String,
}

impl Choice {
    fn new(
        title: impl Into<String>,
        assign: &[(&str, Option<&str>)],
        description: impl Into<String>,
    ) -> Choice {
        Choice {
            title: title.into(),
            assign: assign.iter().map(|(k, v)| (k.to_string(), v.map(str::to_string))).collect(),
            description: description.into(),
            valid: true,
            reason: String::new(),
        }
    }

    fn assigned(&self, key: &str) -> Option<&str> {
        self.assign.iter().find(|(k, _)| k == key).and_then(|(_, v)| v.as_deref())
    }
}

/// A choice the transformation offers for an element.
#[derive(Clone, Debug, PartialEq)]
pub struct Decision {
    pub element_id: String,
    pub title: String,
    pub question: String,
    pub options: Vec<Choice>,
    pub current: usize,
    pub default: usize,
    /// the elements it is about
    pub involved: BTreeSet<String>,
    /// a rarely wanted alternative (shown collapsed)
    pub minor: bool,
    /// the rule in the rule books that describes it
    pub rule: &'static str,
}

/// A problem found while transforming.
#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
    pub element_id: Option<String>,
}

/// A relation with the derived attributes computed by their formulas.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub name: String,
    pub relation: TableId,
    /// (name, SQL expression as written in the diagram)
    pub columns: Vec<(String, String)>,
}

/// The result of the transformation.
#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    /// every relation made on the way, also those merged into others later
    pub tables: Vec<Relation>,
    /// the relations of the schema, in order
    pub relations: Vec<TableId>,
    pub decisions: Vec<Decision>,
    pub issues: Vec<Issue>,
    pub notes: Vec<String>,
    /// constraints SQL cannot express
    pub unenforced: Vec<String>,
    pub rules: String,
    pub views: Vec<View>,
    /// the same schema with names as drawn, for its text forms
    pub drawn: Option<Box<Schema>>,
}

impl Schema {
    /// A relation by its handle.
    pub fn table(&self, id: TableId) -> Option<&Relation> {
        self.tables.get(id.0)
    }

    /// The relations of the schema, in order.
    pub fn relations(&self) -> impl Iterator<Item = &Relation> {
        self.relations.iter().filter_map(|id| self.table(*id))
    }
}

/// How own relations of relationships are named.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RelNaming {
    /// `A_rel_B`
    #[default]
    Entities,
    /// `rel`
    Relationship,
}

/// How foreign key columns are named.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FkNaming {
    /// the key's name
    #[default]
    Plain,
    /// `relKey`
    Relationship,
}

/// Names in SQL form or as drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Names {
    #[default]
    Sql,
    /// as in the diagram, without spaces
    Drawn,
}

/// What the transformation is asked for.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Settings {
    pub rel_naming: RelNaming,
    pub fk_naming: FkNaming,
    /// `db1` (from files of versions up to 3.5) or the default; `None`: as set in the diagram
    pub rules: Option<String>,
    pub names: Names,
}

/// Element options to use instead of the diagram's: element id -> key -> value.
pub type Overrides = BTreeMap<String, BTreeMap<String, Option<String>>>;
