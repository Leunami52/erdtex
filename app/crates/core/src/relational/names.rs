//! Names in a relational schema: sanitised for SQL (`dateOfBirth`, `AirplaneType`), or as drawn
//! without spaces; and data types guessed from names.

use unicode_normalization::UnicodeNormalization;

use crate::text::{cap_first, is_space, lower_first, py_isupper};

const TRANSLIT: &[(char, &str)] =
    &[('ä', "ae"), ('ö', "oe"), ('ü', "ue"), ('Ä', "Ae"), ('Ö', "Oe"), ('Ü', "Ue"), ('ß', "ss")];

/// German umlauts written out, then what is left of the text in ASCII (as Python's
/// `normalize("NFKD", t).encode("ascii", "ignore")`).
fn ascii(text: &str) -> String {
    let mut t = String::new();
    for c in text.chars() {
        match TRANSLIT.iter().find(|(k, _)| *k == c) {
            Some((_, v)) => t.push_str(v),
            None => t.push(c),
        }
    }
    t.nfkd().filter(char::is_ascii).collect()
}

fn with_umlauts_written_out(text: &str) -> String {
    let mut t = String::new();
    for c in text.chars() {
        match TRANSLIT.iter().find(|(k, _)| *k == c) {
            Some((_, v)) => t.push_str(v),
            None => t.push(c),
        }
    }
    t
}

/// The words of a name, in ASCII letters and digits.
pub fn words(text: &str) -> Vec<String> {
    ascii(text)
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

fn starts_with_digit(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_digit())
}

/// `date of birth` -> `dateOfBirth`, `PID` -> `PID`, `flight hrs` -> `flightHrs`.
pub fn column_ident(text: &str) -> String {
    let ws = words(text);
    let Some((first, rest)) = ws.split_first() else { return "col".into() };
    let first = if !rest.is_empty() && !py_isupper(first) { lower_first(first) } else { first.clone() };
    let s = first + &rest.iter().map(|w| cap_first(w)).collect::<String>();
    if starts_with_digit(&s) { format!("_{s}") } else { s }
}

/// `Airplane type` -> `AirplaneType`.
pub fn relation_ident(text: &str) -> String {
    let s: String = words(text).iter().map(|w| cap_first(w)).collect();
    let s = if s.is_empty() { "Relation".to_string() } else { s };
    if starts_with_digit(&s) { format!("R{s}") } else { s }
}

/// `assign to` -> `assignTo`, `Works for` -> `worksFor`.
pub fn lower_ident(text: &str) -> String {
    let ws = words(text);
    let Some((first, rest)) = ws.split_first() else { return "rel".into() };
    let first = if py_isupper(first) { first.clone() } else { lower_first(first) };
    let s = first + &rest.iter().map(|w| cap_first(w)).collect::<String>();
    if starts_with_digit(&s) { format!("_{s}") } else { s }
}

/// Whether two names are the same one: as SQL would see them once sanitised, whichever way
/// they are written.  So the schema with names as drawn takes the same decisions.
pub fn name_key(name: &str) -> String {
    let written = with_umlauts_written_out(name);
    let key: String = ascii(name).chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    let key = key.to_lowercase();
    if key.is_empty() { written.to_lowercase() } else { key }
}

fn split_words(text: &str) -> Vec<&str> {
    text.split(is_space).filter(|w| !w.is_empty()).collect()
}

/// Schemas keep a name as drawn, without spaces: `Raum-Id` -> `Raum-Id`,
/// `date of birth` -> `dateOfBirth`.
pub fn drawn_ident(text: &str) -> String {
    let ws = split_words(text);
    let Some((first, rest)) = ws.split_first() else { return "col".into() };
    first.to_string() + &rest.iter().map(|w| cap_first(w)).collect::<String>()
}

/// The same as the start of a longer name: `Works for` -> `worksFor`.
pub fn drawn_lower(text: &str) -> String {
    let s = drawn_ident(text);
    match split_words(text).first() {
        Some(first) if py_isupper(first) => s,
        _ => lower_first(&s),
    }
}

/// `prefix` + `Name`, or the name alone without a prefix.
pub fn qualify(prefix: &str, name: &str) -> String {
    if prefix.is_empty() { name.to_string() } else { format!("{prefix}{}", cap_first(name)) }
}

/// The data types the editor offers.
pub const TYPE_PRESETS: &[&str] =
    &["integer", "string", "text", "decimal", "float", "boolean", "date", "time", "datetime"];

fn ends_with_any(s: &str, words: &[&str]) -> bool {
    words.iter().any(|w| s.ends_with(w))
}

/// A data type from an attribute's name.
pub fn guess_type(name: &str) -> &'static str {
    let n = column_ident(name);
    let low = n.to_lowercase();
    let boolean = ["is", "has", "can"].iter().any(|p| {
        n.strip_prefix(p)
            .and_then(|r| r.chars().next())
            .is_some_and(|c| c.is_ascii_uppercase() || c == '_' || c == ' ')
    });
    if boolean {
        return "boolean";
    }
    if ends_with_any(&low, &["date", "birth", "day"]) || low.starts_with("date") || low.contains("birthday") {
        return "date";
    }
    let moments = [
        "from",
        "to",
        "since",
        "until",
        "start",
        "end",
        "begin",
        "validfrom",
        "valid from",
        "validto",
        "valid to",
    ];
    if moments.contains(&low.as_str()) || low.contains("timestamp") {
        return "datetime";
    }
    if low.ends_with("time") {
        return "time";
    }
    if ends_with_any(
        &low,
        &[
            "price", "salary", "cost", "amount", "total", "balance", "fee", "wage", "budget", "bonus",
            "payment",
        ],
    ) {
        return "decimal";
    }
    let measures = [
        "length", "speed", "weight", "height", "width", "diameter", "range", "size", "capacity", "rate",
        "distance", "area", "volume", "duration", "hours", "hrs",
    ];
    if ends_with_any(&low, &measures) {
        return "float";
    }
    let counts = [
        "no", "nr", "num", "number", "id", "count", "qty", "quantity", "seats", "year", "age", "level",
        "rank", "semester", "sem", "floor", "room", "code",
    ];
    if ends_with_any(&low, &counts) || ends_with_any(&n, &["ID", "No", "Nr"]) {
        return "integer";
    }
    "string"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(column_ident("date of birth"), "dateOfBirth");
        assert_eq!(column_ident("PID"), "PID");
        assert_eq!(column_ident("Vorgänger"), "Vorgaenger");
        assert_eq!(column_ident("1st"), "_1st");
        assert_eq!(relation_ident("Airplane type"), "AirplaneType");
        assert_eq!(lower_ident("Works for"), "worksFor");
        assert_eq!(drawn_ident("Raum-Id"), "Raum-Id");
        assert_eq!(drawn_lower("Works for"), "worksFor");
        assert_eq!(name_key("Raum-Id"), "raumid");
        assert_eq!(name_key("Café"), "cafe");
        assert_eq!(guess_type("date of birth"), "date");
        assert_eq!(guess_type("isActive"), "boolean");
        assert_eq!(guess_type("PID"), "integer");
        assert_eq!(guess_type("name"), "string");
    }
}
