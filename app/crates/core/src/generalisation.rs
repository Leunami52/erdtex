//! The type of a generalisation: disjoint or overlapping, complete or incomplete.
//!
//! A file stores the letters: cd, co, id, io (or just one of the two letters
//! while the other is still open).  Free text such as {incomplete, overlapping} or
//! "total, disjunkt" is understood too, and printed as written.
//!
//! "Complete" is also called total or vollständig; "incomplete" partial.

use crate::latex_text::plain_text;
use crate::text::strip;

/// The words for the letters.
pub const WORDS: &[(char, &str)] =
    &[('c', "complete"), ('i', "incomplete"), ('d', "disjoint"), ('o', "overlapping")];

/// `[ci][do]|[cido]`
fn letters_only(t: &str) -> bool {
    let chars: Vec<char> = t.chars().collect();
    match chars.as_slice() {
        [a] => "cido".contains(*a),
        [a, b] => "ci".contains(*a) && "do".contains(*b),
        _ => false,
    }
}

/// What a constraint says: (complete, disjoint); `None` where it says nothing.
pub fn parse_constraint(text: Option<&str>) -> (Option<bool>, Option<bool>) {
    let t = plain_text(text.unwrap_or("")).to_lowercase();
    let any = |words: &[&str]| words.iter().any(|w| t.contains(w));
    let mut complete = if any(&["incomplete", "partial", "partiell", "nicht vollst", "unvollst"]) {
        Some(false)
    } else if any(&["complete", "total", "vollst"]) {
        Some(true)
    } else {
        None
    };
    let mut disjoint = if any(&["overlap", "nicht disjunkt", "überlapp"]) {
        Some(false)
    } else if any(&["disjoint", "disjunkt"]) {
        Some(true)
    } else {
        None
    };
    if complete.is_none() && disjoint.is_none() {
        let letters: String = t.chars().filter(char::is_ascii_lowercase).collect();
        if letters_only(&letters) {
            for l in letters.chars() {
                if l == 'c' || l == 'i' {
                    complete = Some(l == 'c');
                } else {
                    disjoint = Some(l == 'd');
                }
            }
        }
    }
    (complete, disjoint)
}

/// The letters for a type: `cd`, `io`, ... (`None` if nothing is known).
pub fn letters(complete: Option<bool>, disjoint: Option<bool>) -> Option<String> {
    let mut s = String::new();
    if let Some(c) = complete {
        s.push(if c { 'c' } else { 'i' });
    }
    if let Some(d) = disjoint {
        s.push(if d { 'd' } else { 'o' });
    }
    (!s.is_empty()).then_some(s)
}

/// `[ci][do]|[cido]`, around spaces.
pub fn is_letters(text: Option<&str>) -> bool {
    letters_only(strip(text.unwrap_or("")))
}

/// `cd` -> `complete, disjoint`; anything but the letters stays as it is.
pub fn words(text: Option<&str>) -> Option<String> {
    if !is_letters(text) {
        return text.map(str::to_string);
    }
    let w: Vec<&str> = strip(text.unwrap_or(""))
        .chars()
        .filter_map(|c| WORDS.iter().find(|(l, _)| *l == c).map(|(_, w)| *w))
        .collect();
    Some(w.join(", "))
}

fn subs_text(subtypes: &[String]) -> String {
    if subtypes.is_empty() { "its subtypes".into() } else { subtypes.join(", ") }
}

/// What disjoint or overlapping means here, with the real names.
pub fn explain_overlap(disjoint: Option<bool>, supertype: &str, subtypes: &[String]) -> String {
    let subs = subs_text(subtypes);
    match disjoint {
        None => "Not specified yet: can one entity belong to several subtypes?".into(),
        Some(true) => format!("Each {supertype} is at most one of: {subs}."),
        Some(false) => format!("Each {supertype} can be several of: {subs}."),
    }
}

/// What disjoint or overlapping means here, read after the coverage (the panel shows coverage first,
/// as the letters do: `cd`): with a complete generalisation, "exactly one" or "one or several".
pub fn explain_overlap_after(
    disjoint: Option<bool>,
    complete: Option<bool>,
    supertype: &str,
    subtypes: &[String],
) -> String {
    let subs = subs_text(subtypes);
    match (disjoint, complete) {
        (Some(true), Some(true)) => {
            format!(
                "So each {supertype} is exactly one of: {subs} (at least one, as it is complete, and at most one)."
            )
        }
        (Some(false), Some(true)) => format!("So each {supertype} is one or several of: {subs}."),
        _ => explain_overlap(disjoint, supertype, subtypes),
    }
}

/// What complete or incomplete means here, with the real names.
pub fn explain_coverage(complete: Option<bool>, supertype: &str, subtypes: &[String]) -> String {
    let subs = subs_text(subtypes);
    match complete {
        None => "Not specified yet: does every entity belong to a subtype?".into(),
        Some(true) => {
            format!("Each {supertype} is at least one of: {subs}. Also called total (vollständig).")
        }
        Some(false) => format!("A {supertype} need not be any of them. Also called partial."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_and_words() {
        assert_eq!(words(Some("cd")).as_deref(), Some("complete, disjoint"));
        assert_eq!(words(Some("total")).as_deref(), Some("total"));
        assert_eq!(words(None), None);
        assert!(is_letters(Some(" o ")));
        assert_eq!(parse_constraint(Some("{incomplete, overlapping}")), (Some(false), Some(false)));
        assert_eq!(parse_constraint(Some("total, disjunkt")), (Some(true), Some(true)));
        assert_eq!(parse_constraint(Some("c")), (Some(true), None));
        assert_eq!(letters(Some(true), Some(false)).as_deref(), Some("co"));
    }
}
