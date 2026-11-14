//! Output of a relational schema:
//!
//! * text: `Person : {(_PID_ : integer, name : string)}`, a set of tuples (script def. 5.2)
//! * latex: the same in the environment `relationalschema` of erdiagram.sty
//! * markdown: the same, with primary keys in bold (Markdown has no underline), foreign keys in
//!   italics, both in bold italics
//! * postgresql, oracle, sqlite: CREATE TABLE statements

use std::collections::{HashMap, HashSet};

use super::md5::hex_digest;
use super::{Relation, Schema, TableId};
use crate::rules::Severity;
use crate::text::{is_space, strip};

/// The formats: (key, title).
pub const FORMATS: &[(&str, &str)] = &[
    ("text", "Relational schema (plain text)"),
    ("latex", "Relational schema (LaTeX)"),
    ("markdown", "Relational schema (Markdown)"),
    ("postgresql", "SQL — PostgreSQL"),
    ("oracle", "SQL — Oracle"),
    ("sqlite", "SQL — SQLite"),
];

/// The SQL dialects.
pub const DIALECTS: &[&str] = &["postgresql", "oracle", "sqlite"];

/// A dialect's name as people write it.
pub fn dialect_name(dialect: &str) -> &'static str {
    match dialect {
        "postgresql" => "PostgreSQL",
        "oracle" => "Oracle",
        _ => "SQLite",
    }
}

/// How keys are marked in the plain text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Marks {
    /// `_key_`, `/foreign/`, `_/both/_`
    #[default]
    Underscore,
    /// `(PK)`, `(FK)`
    Tags,
}

/// Which SQL names are quoted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Quote {
    /// only where a name needs it
    #[default]
    Needed,
    All,
}

fn decision_lines(schema: &Schema) -> Vec<String> {
    schema
        .decisions
        .iter()
        .filter(|d| !(d.minor && d.current == d.default))
        .map(|d| {
            let title =
                d.options.get(d.current).map(|o| o.title.replace("  (default)", "")).unwrap_or_default();
            format!("{}: {title}", d.title)
        })
        .collect()
}

const OUTSIDE: &str = "Outside of any box";

/// Grouped under their subdomain boxes; one group without title if there are none.
fn by_box(schema: &Schema) -> Vec<(Option<String>, Vec<&Relation>)> {
    let relations: Vec<&Relation> = schema.relations().collect();
    let mut titles: Vec<&str> = Vec::new();
    for r in &relations {
        if let Some(t) = r.box_title.as_deref().filter(|t| !t.is_empty())
            && !titles.contains(&t)
        {
            titles.push(t);
        }
    }
    if titles.is_empty() {
        return vec![(None, relations)];
    }
    let in_box = |r: &Relation, t: &str| r.box_title.as_deref() == Some(t);
    let mut groups: Vec<(Option<String>, Vec<&Relation>)> = titles
        .iter()
        .map(|t| (Some(t.to_string()), relations.iter().copied().filter(|r| in_box(r, t)).collect()))
        .collect();
    let rest: Vec<&Relation> =
        relations.iter().copied().filter(|r| r.box_title.as_deref().is_none_or(str::is_empty)).collect();
    if !rest.is_empty() {
        groups.push((Some(OUTSIDE.into()), rest));
    }
    groups
}

fn tuple(cols: &[String]) -> String {
    match cols {
        [one] => one.clone(),
        _ => format!("({})", cols.join(", ")),
    }
}

fn table_name(schema: &Schema, id: TableId) -> &str {
    schema.table(id).map_or("", |t| t.name.as_str())
}

/// The plain text form, with the names as drawn.
pub fn to_text(schema: &Schema, marks: Marks, types: bool, source: &str) -> String {
    let schema = schema.drawn.as_deref().unwrap_or(schema);
    let mut lines = vec![format!(
        "Relational schema{}",
        if source.is_empty() { String::new() } else { format!(" of {source}") }
    )];
    if marks == Marks::Underscore {
        lines.push("Notation: _name_ primary key, /name/ foreign key, _/name/_ both".into());
    }
    lines.push(String::new());
    for (title, relations) in by_box(schema) {
        if let Some(t) = &title {
            lines.push(format!("{t}:"));
        }
        for rel in relations {
            let fkc = rel.fk_columns();
            let cols: Vec<String> = rel
                .columns
                .iter()
                .map(|c| {
                    let mut n = c.name.clone();
                    if marks == Marks::Underscore {
                        if fkc.contains(&n) {
                            n = format!("/{n}/");
                        }
                        if c.pk {
                            n = format!("_{n}_");
                        }
                    } else {
                        let mut tags = Vec::new();
                        if c.pk {
                            tags.push("PK");
                        }
                        if fkc.contains(&c.name) {
                            tags.push("FK");
                        }
                        if !tags.is_empty() {
                            n += &format!(" ({})", tags.join(", "));
                        }
                    }
                    if types { format!("{n} : {}", c.ty) } else { n }
                })
                .collect();
            let indent = if title.is_some() { "  " } else { "" };
            lines.push(format!("{indent}{} : {{({})}}", rel.name, cols.join(", ")));
        }
    }
    let fks: Vec<String> = schema
        .relations()
        .flat_map(|rel| {
            rel.fks.iter().map(move |fk| {
                format!(
                    "  {}.{} -> {}.{}",
                    rel.name,
                    tuple(&fk.columns),
                    table_name(schema, fk.table),
                    tuple(&fk.ref_columns)
                )
            })
        })
        .collect();
    if !fks.is_empty() {
        lines.extend(["".into(), "Foreign keys".into()]);
        lines.extend(fks);
    }
    let uniq: Vec<String> = schema
        .relations()
        .flat_map(|rel| rel.uniques.iter().map(move |u| format!("  {}.{}", rel.name, tuple(u))))
        .collect();
    if !uniq.is_empty() {
        lines.extend(["".into(), "Unique (alternate keys)".into()]);
        lines.extend(uniq);
    }
    let checks: Vec<String> = schema
        .relations()
        .flat_map(|rel| {
            rel.columns.iter().filter_map(move |c| {
                c.check_values
                    .as_ref()
                    .filter(|v| !v.is_empty())
                    .map(|v| format!("  {}.{} in {{{}}}", rel.name, c.name, v.join(", ")))
            })
        })
        .collect();
    if !checks.is_empty() {
        lines.extend(["".into(), "Allowed values".into()]);
        lines.extend(checks);
    }
    if !schema.views.is_empty() {
        lines.extend(["".into(), "Views (computing derived attributes)".into()]);
        for v in &schema.views {
            let cols: Vec<String> = v.columns.iter().map(|(n, f)| format!("{n} = {f}")).collect();
            lines.push(format!(
                "  {}: all of {}, and {}",
                v.name,
                table_name(schema, v.relation),
                cols.join(", ")
            ));
        }
    }
    if !schema.unenforced.is_empty() {
        lines.extend(["".into(), "Not enforced by the schema".into()]);
        lines.extend(schema.unenforced.iter().map(|t| format!("  {t}")));
    }
    let decisions = decision_lines(schema);
    if !decisions.is_empty() {
        lines.extend(["".into(), "Decisions".into()]);
        lines.extend(decisions.iter().map(|d| format!("  {d}")));
    }
    if !schema.notes.is_empty() {
        lines.extend(["".into(), "Notes".into()]);
        lines.extend(schema.notes.iter().map(|n| format!("  {n}")));
    }
    lines.join("\n") + "\n"
}

/// Markdown's special characters escaped, so that names and texts show as written (`a_b` stays
/// `a_b`, `*` doesn't start emphasis, `<` no HTML tag).
pub fn md_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if "\\`*_[]<#|~".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The legend of the Markdown form: it can't underline, so primary keys are bold.
pub const MARKDOWN_LEGEND: &str = "Primary keys in **bold** (Markdown has no underline), foreign keys in *italics*, \
                                   both in ***bold italics***.";

/// The Markdown form, with the names as drawn: the plain text form's parts as headings and lists.
pub fn to_markdown(schema: &Schema, types: bool, source: &str) -> String {
    let schema = schema.drawn.as_deref().unwrap_or(schema);
    let mut lines = vec![
        format!(
            "## Relational schema{}",
            if source.is_empty() { String::new() } else { format!(" of {}", md_escape(source)) }
        ),
        String::new(),
        MARKDOWN_LEGEND.to_string(),
        String::new(),
    ];
    let section = |lines: &mut Vec<String>, title: &str, items: Vec<String>| {
        if !items.is_empty() {
            lines.push(format!("### {title}"));
            lines.push(String::new());
            lines.extend(items.into_iter().map(|i| format!("- {i}")));
            lines.push(String::new());
        }
    };
    for (title, relations) in by_box(schema) {
        let items: Vec<String> = relations
            .iter()
            .map(|rel| {
                let fkc = rel.fk_columns();
                let cols: Vec<String> = rel
                    .columns
                    .iter()
                    .map(|c| {
                        let name = md_escape(&c.name);
                        let marked = match (c.pk, fkc.contains(&c.name)) {
                            (true, true) => format!("***{name}***"),
                            (true, false) => format!("**{name}**"),
                            (false, true) => format!("*{name}*"),
                            (false, false) => name,
                        };
                        if types { format!("{marked} : {}", md_escape(&c.ty)) } else { marked }
                    })
                    .collect();
                format!("{} : {{({})}}", md_escape(&rel.name), cols.join(", "))
            })
            .collect();
        match title {
            Some(t) => section(&mut lines, &md_escape(&t), items),
            None => {
                lines.extend(items.into_iter().map(|i| format!("- {i}")));
                lines.push(String::new());
            }
        }
    }
    let name = |id: TableId| md_escape(table_name(schema, id));
    let tup = |cols: &[String]| md_escape(&tuple(cols));
    let fks = schema
        .relations()
        .flat_map(|rel| {
            rel.fks.iter().map(move |fk| {
                format!(
                    "{}.{} → {}.{}",
                    md_escape(&rel.name),
                    tup(&fk.columns),
                    name(fk.table),
                    tup(&fk.ref_columns)
                )
            })
        })
        .collect();
    section(&mut lines, "Foreign keys", fks);
    let uniq = schema
        .relations()
        .flat_map(|rel| rel.uniques.iter().map(move |u| format!("{}.{}", md_escape(&rel.name), tup(u))))
        .collect();
    section(&mut lines, "Unique (alternate keys)", uniq);
    let checks = schema
        .relations()
        .flat_map(|rel| {
            rel.columns.iter().filter_map(move |c| {
                c.check_values.as_ref().filter(|v| !v.is_empty()).map(|v| {
                    let values: Vec<String> = v.iter().map(|x| md_escape(x)).collect();
                    format!("{}.{} in {{{}}}", md_escape(&rel.name), md_escape(&c.name), values.join(", "))
                })
            })
        })
        .collect();
    section(&mut lines, "Allowed values", checks);
    let views = schema
        .views
        .iter()
        .map(|v| {
            let cols: Vec<String> =
                v.columns.iter().map(|(n, f)| format!("{} = {}", md_escape(n), md_escape(f))).collect();
            format!("{}: all of {}, and {}", md_escape(&v.name), name(v.relation), cols.join(", "))
        })
        .collect();
    section(&mut lines, "Views (computing derived attributes)", views);
    section(
        &mut lines,
        "Not enforced by the schema",
        schema.unenforced.iter().map(|t| md_escape(t)).collect(),
    );
    section(&mut lines, "Decisions", decision_lines(schema).iter().map(|d| md_escape(d)).collect());
    section(&mut lines, "Notes", schema.notes.iter().map(|n| md_escape(n)).collect());
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines.join("\n") + "\n"
}

/// LaTeX's special characters escaped.
pub fn tex_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if "_&%$#{}".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out.replace('~', "\\textasciitilde{}")
}

/// The environment `relationalschema` of erdiagram.sty, with the names as drawn.
pub fn to_latex(schema: &Schema, source: &str) -> String {
    let schema = schema.drawn.as_deref().unwrap_or(schema);
    let mut out = vec![
        format!(
            "% Relational schema{} (generated by ERDTeX); needs \\usepackage{{erdiagram}}",
            if source.is_empty() { String::new() } else { format!(" of {source}") }
        ),
        "\\begin{relationalschema}".to_string(),
    ];
    for (title, relations) in by_box(schema) {
        if let Some(t) = title {
            out.push(format!("  \\relationgroup{{{}}}", tex_escape(&t)));
        }
        for rel in relations {
            let fkc = rel.fk_columns();
            let cols: Vec<String> = rel
                .columns
                .iter()
                .map(|c| {
                    let fk = fkc.contains(&c.name);
                    let macro_name = match (c.pk, fk) {
                        (true, true) => "pfkey",
                        (true, false) => "pkey",
                        (false, true) => "fkey",
                        (false, false) => "col",
                    };
                    format!("\\{macro_name}{{{}}}{{{}}}", tex_escape(&c.name), tex_escape(&c.ty))
                })
                .collect();
            out.push(format!("  \\relation{{{}}}{{{}}}", tex_escape(&rel.name), cols.join(", ")));
        }
    }
    out.push("\\end{relationalschema}".into());
    for rel in schema.relations() {
        for u in &rel.uniques {
            out.push(format!("% unique in {}: {}", rel.name, u.join(", ")));
        }
    }
    for v in &schema.views {
        let cols: Vec<String> = v.columns.iter().map(|(n, f)| format!("{n} = {f}")).collect();
        out.push(format!(
            "% view {}: all of {}, and {}",
            v.name,
            table_name(schema, v.relation),
            cols.join(", ")
        ));
    }
    out.join("\n") + "\n"
}

// ---------------------------------------------------------------------------------------------
//  SQL
// ---------------------------------------------------------------------------------------------

const RESERVED: &str = "ACCESS ADD ALL ALTER ANALYSE ANALYZE AND ANY ARRAY AS ASC ASYMMETRIC AUDIT AUTHORIZATION \
AUTOINCREMENT BETWEEN BINARY BOTH BY CASE CAST CHAR CHECK CLUSTER COLLATE COLUMN COMMENT \
COMPRESS CONCURRENTLY CONNECT CONSTRAINT CREATE CROSS CURRENT CURRENT_CATALOG CURRENT_DATE \
CURRENT_ROLE CURRENT_SCHEMA CURRENT_TIME CURRENT_TIMESTAMP CURRENT_USER DATE DECIMAL DEFAULT \
DEFERRABLE DELETE DESC DISTINCT DO DROP ELSE END ESCAPE EXCEPT EXCLUSIVE EXISTS FALSE FETCH \
FILE FLOAT FOR FOREIGN FREEZE FROM FULL GLOB GRANT GROUP HAVING IDENTIFIED ILIKE IMMEDIATE IN \
INCREMENT INDEX INITIAL INITIALLY INNER INSERT INTEGER INTERSECT INTO IS ISNULL JOIN LATERAL \
LEADING LEFT LEVEL LIKE LIMIT LOCALTIME LOCALTIMESTAMP LOCK LONG MAXEXTENTS MINUS MLSLABEL MODE \
MODIFY NATURAL NOAUDIT NOCOMPRESS NOT NOTNULL NOWAIT NULL NUMBER OF OFFLINE OFFSET ON ONLINE \
ONLY OPTION OR ORDER OUTER OVERLAPS PCTFREE PLACING PRIMARY PRIOR PRIVILEGES PUBLIC RAW \
REFERENCES REGEXP RENAME RESOURCE RETURNING REVOKE RIGHT ROW ROWID ROWNUM ROWS SELECT SESSION \
SESSION_USER SET SHARE SIZE SIMILAR SMALLINT SOME START SUCCESSFUL SYMMETRIC SYNONYM SYSDATE \
TABLE TABLESAMPLE THEN TO TRAILING TRANSACTION TRIGGER TRUE UID UNION UNIQUE UPDATE USER USING \
VALIDATE VALUES VARCHAR VARCHAR2 VARIADIC VERBOSE VIEW WHEN WHENEVER WHERE WINDOW WITH";

fn reserved(name: &str) -> bool {
    let up = name.to_uppercase();
    RESERVED.split(' ').any(|w| w == up)
}

/// A name as SQL needs it written: quoted if it isn't a plain name, is reserved, or all are.
pub fn quote_ident(name: &str, dialect: &str, quote: Quote) -> String {
    let mut chars = name.chars();
    let first_ok = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || (c == '_' && dialect != "oracle"));
    let plain_ok = first_ok && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if quote == Quote::All || !plain_ok || reserved(name) {
        format!("\"{}\"", name.replace('"', "\"\""))
    } else {
        name.to_string()
    }
}

/// `\s*([A-Za-z][A-Za-z0-9_ ]*?)\s*(?:\(\s*([^)]*?)\s*\))?\s*`, read as Python's `re.fullmatch`:
/// (name, arguments).
fn type_parts(t: &str) -> Option<(String, Option<String>)> {
    let s = t.trim_start_matches(is_space);
    let mut chars = s.char_indices();
    let (_, first) = chars.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    let run_end = s
        .char_indices()
        .skip(1)
        .find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '_' || *c == ' '))
        .map_or(s.len(), |(i, _)| i);
    let run = s.get(..run_end)?;
    let name = run.trim_end_matches(' ');
    let rest = s.get(name.len()..)?.trim_start_matches(is_space);
    if rest.is_empty() {
        return Some((name.to_string(), None));
    }
    let inner = rest.strip_prefix('(')?;
    let close = inner.find(')')?;
    let after = inner.get(close + 1..)?;
    if !after.chars().all(is_space) {
        return None;
    }
    let args = inner.get(..close)?.trim_matches(is_space);
    Some((name.to_string(), Some(args.to_string())))
}

/// A data type in a dialect: (SQL type, Boolean emulated with 0/1, known).
pub fn sql_type(t: &str, dialect: &str, strlen: usize) -> (String, bool, bool) {
    let Some((raw, args)) = type_parts(t) else { return (t.to_string(), false, false) };
    let name = raw.to_lowercase().replace([' ', '_'], "");
    let args = args.filter(|a| !a.is_empty());
    let (p, o) = (dialect == "postgresql", dialect == "oracle");
    let pick = |pg: &str, ora: &str, lite: &str| {
        if p {
            pg.to_string()
        } else if o {
            ora.to_string()
        } else {
            lite.to_string()
        }
    };
    let known = |s: String| (s, false, true);
    match name.as_str() {
        "integer" | "int" | "int4" | "long" | "serial" => known(pick("INTEGER", "NUMBER(10)", "INTEGER")),
        "smallint" | "tinyint" | "int2" | "short" | "byte" => known(pick("SMALLINT", "NUMBER(5)", "INTEGER")),
        "bigint" | "int8" | "bigserial" => known(pick("BIGINT", "NUMBER(19)", "INTEGER")),
        "string" | "varchar" | "varchar2" | "nvarchar" | "charactervarying" => {
            let n = args.unwrap_or_else(|| strlen.to_string());
            known(pick(&format!("VARCHAR({n})"), &format!("VARCHAR2({n})"), &format!("VARCHAR({n})")))
        }
        "char" | "character" | "nchar" => known(format!("CHAR({})", args.unwrap_or_else(|| "1".into()))),
        "text" | "clob" | "longtext" | "memo" | "longstring" => known(pick("TEXT", "CLOB", "TEXT")),
        "decimal" | "numeric" | "money" | "currency" | "number" | "fixed" => {
            let a = args.unwrap_or_else(|| {
                if matches!(name.as_str(), "decimal" | "money" | "currency") {
                    "12,2".into()
                } else {
                    String::new()
                }
            });
            let a = if a.is_empty() { a } else { format!("({a})") };
            known(pick(&format!("NUMERIC{a}"), &format!("NUMBER{a}"), &format!("NUMERIC{a}")))
        }
        "float" | "double" | "doubleprecision" | "float8" | "real" | "float4" | "single" => {
            let small = matches!(name.as_str(), "real" | "float4" | "single");
            known(pick(
                if small { "REAL" } else { "DOUBLE PRECISION" },
                if small { "BINARY_FLOAT" } else { "BINARY_DOUBLE" },
                "REAL",
            ))
        }
        "boolean" | "bool" | "bit" | "flag" | "yesno" => (pick("BOOLEAN", "NUMBER(1)", "INTEGER"), !p, true),
        "date" => known("DATE".into()),
        "time" | "timeofday" => known(pick("TIME", "INTERVAL DAY(0) TO SECOND(0)", "TIME")),
        "datetime" | "timestamp" | "timestamptz" | "datetime2" | "smalldatetime" => {
            known(pick("TIMESTAMP", "TIMESTAMP", "DATETIME"))
        }
        "interval" | "duration" => known(pick("INTERVAL", "INTERVAL DAY TO SECOND", "TEXT")),
        "blob" | "binary" | "bytea" | "varbinary" | "image" | "bytes" => known(pick("BYTEA", "BLOB", "BLOB")),
        "uuid" => known(pick("UUID", "RAW(16)", "TEXT")),
        _ => (strip(t).to_string(), false, false),
    }
}

/// A constraint name, at most 30 characters (portable to old Oracle versions).
fn constraint_name(kind: &str, table: &str, extra: &str, used: &mut HashSet<String>) -> String {
    let raw = format!("{kind}_{table}{}", if extra.is_empty() { String::new() } else { format!("_{extra}") });
    let mut base: String =
        raw.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if base.len() > 30 {
        let h = hex_digest(base.as_bytes());
        base = format!("{}_{}", base.get(..23).unwrap_or(&base), h.get(..6).unwrap_or(&h));
    }
    let mut name = base.clone();
    let mut k = 2;
    while used.contains(&name.to_lowercase()) {
        name = format!("{}_{k}", base.get(..27).unwrap_or(&base));
        k += 1;
    }
    used.insert(name.to_lowercase());
    name
}

/// Referenced relations first; returns (order, foreign keys to add later: (table, index)).
fn creation_order(schema: &Schema) -> (Vec<TableId>, HashSet<(TableId, usize)>) {
    fn visit(
        schema: &Schema,
        r: TableId,
        state: &mut HashMap<TableId, u8>,
        order: &mut Vec<TableId>,
        late: &mut HashSet<(TableId, usize)>,
    ) {
        if state.get(&r) == Some(&2) {
            return;
        }
        state.insert(r, 1);
        let fks = schema.table(r).map(|t| t.fks.clone()).unwrap_or_default();
        for (i, fk) in fks.iter().enumerate() {
            let t = fk.table;
            if t == r {
                continue;
            }
            if state.get(&t) == Some(&1) {
                // a cycle: declare this one later
                late.insert((r, i));
                continue;
            }
            visit(schema, t, state, order, late);
        }
        state.insert(r, 2);
        order.push(r);
    }
    let (mut order, mut state, mut late) = (Vec::new(), HashMap::new(), HashSet::new());
    for &r in &schema.relations {
        visit(schema, r, &mut state, &mut order, &mut late);
    }
    (order, late)
}

fn ljust(s: &str, width: usize) -> String {
    let n = s.chars().count();
    if n >= width { s.to_string() } else { format!("{s}{}", " ".repeat(width - n)) }
}

/// CREATE TABLE statements (with the SQL names).
pub fn to_sql(
    schema: &Schema,
    dialect: &str,
    quote: Quote,
    drop: bool,
    source: &str,
    strlen: usize,
) -> String {
    let q = |n: &str| quote_ident(n, dialect, quote);
    let mut used = HashSet::new();
    let (order, late) = creation_order(schema);
    let mut out = vec![format!(
        "-- {} schema{}, generated by ERDTeX",
        dialect_name(dialect),
        if source.is_empty() { String::new() } else { format!(" of {source}") }
    )];
    let errors: Vec<&str> =
        schema.issues.iter().filter(|i| i.severity == Severity::Error).map(|i| i.message.as_str()).collect();
    if !errors.is_empty() {
        out.push("-- THIS DIAGRAM HAS ERRORS, so the following is not a valid schema:".into());
        out.extend(errors.iter().map(|e| format!("--   {e}")));
    }
    for d in decision_lines(schema) {
        out.push(format!("-- decision: {d}"));
    }
    for n in &schema.notes {
        out.push(format!("-- note: {n}"));
    }
    if dialect == "oracle" {
        out.push("-- Booleans are NUMBER(1) with 0/1 (BOOLEAN needs Oracle 23ai).".into());
    }
    let mut unknown: Vec<String> = schema
        .relations()
        .flat_map(|r| r.columns.iter())
        .filter(|c| !sql_type(&c.ty, dialect, 255).2)
        .map(|c| c.ty.clone())
        .collect();
    unknown.sort();
    unknown.dedup();
    if !unknown.is_empty() {
        out.push(format!("-- note: unknown data type(s) written as given: {}", unknown.join(", ")));
    }
    out.push(String::new());
    if dialect == "sqlite" {
        out.extend(["PRAGMA foreign_keys = ON;".into(), String::new()]);
    }
    let oracle_drop = |stmt: String| {
        vec![
            format!("BEGIN EXECUTE IMMEDIATE '{}';", stmt.replace('\'', "''")),
            "EXCEPTION WHEN OTHERS THEN IF SQLCODE != -942 THEN RAISE; END IF;".to_string(),
            "END;".to_string(),
            "/".to_string(),
        ]
    };
    if drop {
        for v in &schema.views {
            if dialect == "oracle" {
                out.extend(oracle_drop(format!("DROP VIEW {}", q(&v.name))));
            } else {
                out.push(format!("DROP VIEW IF EXISTS {};", q(&v.name)));
            }
        }
        for id in order.iter().rev() {
            let name = q(table_name(schema, *id));
            match dialect {
                "postgresql" => out.push(format!("DROP TABLE IF EXISTS {name} CASCADE;")),
                "sqlite" => out.push(format!("DROP TABLE IF EXISTS {name};")),
                _ => out.extend(oracle_drop(format!("DROP TABLE {name} CASCADE CONSTRAINTS"))),
            }
        }
        out.push(String::new());
    }
    let mut alters = Vec::new();
    for &id in &order {
        let Some(rel) = schema.table(id) else { continue };
        if !rel.comment.is_empty() {
            out.push(format!("-- {}", rel.comment));
        }
        let mut defs = Vec::new();
        let width = rel.columns.iter().map(|c| q(&c.name).chars().count()).max().unwrap_or(0);
        let mut checks = Vec::new();
        for c in &rel.columns {
            let (t, emulated, _) = sql_type(&c.ty, dialect, strlen);
            let mut d = format!("    {}  {t}", ljust(&q(&c.name), width));
            if c.flag {
                d += &format!(" DEFAULT {}", if dialect == "postgresql" { "FALSE" } else { "0" });
            }
            if !c.nullable || c.pk {
                d += " NOT NULL";
            }
            defs.push(d);
            if emulated {
                checks.push((c.name.clone(), format!("{} IN (0, 1)", q(&c.name))));
            }
            if let Some(values) = c.check_values.as_ref().filter(|v| !v.is_empty()) {
                let vals: Vec<String> =
                    values.iter().map(|v| format!("'{}'", v.replace('\'', "''"))).collect();
                checks.push((c.name.clone(), format!("{} IN ({})", q(&c.name), vals.join(", "))));
            }
        }
        let pk = rel.pk();
        if !pk.is_empty() {
            let cols: Vec<String> = pk.iter().map(|c| q(c)).collect();
            defs.push(format!(
                "    CONSTRAINT {} PRIMARY KEY ({})",
                constraint_name("pk", &rel.name, "", &mut used),
                cols.join(", ")
            ));
        }
        for u in &rel.uniques {
            let cols: Vec<String> = u.iter().map(|c| q(c)).collect();
            defs.push(format!(
                "    CONSTRAINT {} UNIQUE ({})",
                constraint_name("uq", &rel.name, &u.join("_"), &mut used),
                cols.join(", ")
            ));
        }
        for (col, expr) in &checks {
            defs.push(format!(
                "    CONSTRAINT {} CHECK ({expr})",
                constraint_name("ck", &rel.name, col, &mut used)
            ));
        }
        for (i, fk) in rel.fks.iter().enumerate() {
            let target = table_name(schema, fk.table);
            let is_late = late.contains(&(id, i));
            let cols: Vec<String> = fk.columns.iter().map(|c| q(c)).collect();
            let refs: Vec<String> = fk.ref_columns.iter().map(|c| q(c)).collect();
            let stmt = format!(
                "CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({}){}{}",
                constraint_name("fk", &rel.name, target, &mut used),
                cols.join(", "),
                q(target),
                refs.join(", "),
                if fk.cascade { " ON DELETE CASCADE" } else { "" },
                // mutually referencing tables could never be filled otherwise
                if is_late { " DEFERRABLE INITIALLY DEFERRED" } else { "" }
            );
            if is_late && dialect != "sqlite" {
                alters.push(format!("ALTER TABLE {} ADD {stmt};", q(&rel.name)));
            } else {
                defs.push(format!("    {stmt}"));
            }
        }
        if let Some(b) = rel.box_title.as_deref().filter(|b| !b.is_empty()) {
            // only a comment: the order has to stay safe for foreign keys (an & would make
            // Oracle's SQL*Plus ask for a value, even in a comment)
            out.push(format!("-- box: {}", b.replace('&', "and")));
        }
        out.push(format!("CREATE TABLE {} (", q(&rel.name)));
        out.push(defs.join(",\n"));
        out.push(");".into());
        out.push(String::new());
    }
    if !alters.is_empty() {
        out.push("-- foreign keys of mutually referencing relations".into());
        out.extend(alters);
        out.push(String::new());
    }
    if !schema.views.is_empty() {
        out.push("-- derived attributes; the formulas are written as in the diagram".into());
        for v in &schema.views {
            let cols: String = v.columns.iter().map(|(n, f)| format!(", ({f}) AS {}", q(n))).collect();
            out.push(format!(
                "CREATE VIEW {} AS SELECT t.*{cols} FROM {} t;",
                q(&v.name),
                q(table_name(schema, v.relation))
            ));
        }
        out.push(String::new());
    }
    if !schema.unenforced.is_empty() {
        out.push("-- not enforced by this schema (needs triggers or the application):".into());
        out.extend(schema.unenforced.iter().map(|t| format!("--   {t}")));
        out.push(String::new());
    }
    out.join("\n")
}

/// A schema in one of [`FORMATS`].
pub fn render(
    schema: &Schema,
    format: &str,
    source: &str,
    marks: Marks,
    types: bool,
    quote: Quote,
    drop: bool,
) -> String {
    match format {
        "text" => to_text(schema, marks, types, source),
        "latex" => to_latex(schema, source),
        "markdown" => to_markdown(schema, types, source),
        dialect => to_sql(schema, dialect, quote, drop, source, 255),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_and_names() {
        assert_eq!(sql_type("string", "oracle", 255).0, "VARCHAR2(255)");
        assert_eq!(sql_type(" decimal ( 8, 2 ) ", "postgresql", 255).0, "NUMERIC(8, 2)");
        assert_eq!(sql_type("double precision", "sqlite", 255).0, "REAL");
        assert_eq!(sql_type("boolean", "sqlite", 255), ("INTEGER".into(), true, true));
        assert_eq!(sql_type("geo(1)x", "sqlite", 255), ("geo(1)x".into(), false, false));
        assert_eq!(sql_type(" foo ", "sqlite", 255), ("foo".into(), false, false));
        assert_eq!(quote_ident("_a", "oracle", Quote::Needed), "\"_a\"");
        assert_eq!(quote_ident("_a", "sqlite", Quote::Needed), "_a");
        assert_eq!(quote_ident("order", "sqlite", Quote::Needed), "\"order\"");
        assert_eq!(tex_escape("a_b~"), "a\\_b\\textasciitilde{}");
        assert_eq!(md_escape("a_b*c<d"), "a\\_b\\*c\\<d");
        let mut used = HashSet::new();
        assert_eq!(constraint_name("pk", "A", "", &mut used), "pk_A");
        assert_eq!(constraint_name("pk", "A", "", &mut used), "pk_A_2");
    }
}
