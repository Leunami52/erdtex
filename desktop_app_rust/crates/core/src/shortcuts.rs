//! All keyboard shortcuts of the app.
//!
//! A command has an id, a title and one or more default key combinations, written like
//! `Ctrl+Shift+1; Ctrl+Y`.  What the user changed is kept in the app's settings under
//! `shortcuts/<id>`, never in a diagram file.  Tools have a set per kind of diagram.
//!
//! Key combinations are written in Qt's portable form, as the Python app stores them;
//! [`canonical`] reads one as Qt does (modifiers in the order Meta, Ctrl, Alt, Shift; key names
//! such as `Del` and `Esc`).  The platform's standard keys (New, Save as, Zoom in, ...) are
//! written as Qt has them on Linux; `Ctrl` stands for Cmd on macOS.

use std::collections::{BTreeSet, HashMap};

/// The prefix of the settings that hold changed shortcuts.
pub const KEY: &str = "shortcuts/";

/// Where a command applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Group {
    General,
    Er,
    Gsm,
    /// handled by the canvas or a list, only while it has the focus
    Canvas,
}

impl Group {
    /// The heading in the preferences.
    pub fn title(self) -> &'static str {
        match self {
            Group::General => "General",
            Group::Er => "ER tools",
            Group::Gsm => "GSM tools",
            Group::Canvas => "On the canvas and in lists",
        }
    }
}

/// A command that can have a shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command {
    pub id: &'static str,
    pub title: &'static str,
    pub group: Group,
    /// key combinations separated by `; `
    pub default: &'static str,
}

const fn c(id: &'static str, title: &'static str, group: Group, default: &'static str) -> Command {
    Command { id, title, group, default }
}

use Group::{Canvas, Er, General, Gsm};

/// Every command, in the order of the preferences.
pub const COMMANDS: &[Command] = &[
    // general: the attribute of the main window's action is the id
    c("a_new_er", "New ER diagram", General, "Ctrl+N"),
    c("a_new_gsm", "New GSM diagram", General, "Ctrl+Shift+N"),
    c("a_open", "Open", General, "Ctrl+O"),
    c("a_example_er", "Open example ER diagram", General, ""),
    c("a_example_gsm", "Open example GSM diagram", General, ""),
    c("a_save", "Save", General, "Ctrl+S"),
    c("a_save_as", "Save as", General, "Ctrl+Shift+S"),
    c("a_reload", "Reload from disk", General, "F5"),
    c("a_close_tab", "Close tab", General, "Ctrl+W"),
    c("a_next_tab", "Next tab", General, "Ctrl+Tab"),
    c("a_prev_tab", "Previous tab", General, "Ctrl+Shift+Tab"),
    // tabs side by side (version 3 only)
    c("a_split_right", "Split right", General, "Ctrl+Alt+Right"),
    c("a_split_down", "Split down", General, "Ctrl+Alt+Down"),
    c("a_next_group", "Next group", General, "Ctrl+Alt+PgDown"),
    c("a_prev_group", "Previous group", General, "Ctrl+Alt+PgUp"),
    c("a_move_tab_group", "Move tab to the other group", General, ""),
    c("a_export", "Export", General, "Ctrl+E"),
    c("a_relational", "Relational schema / SQL", General, "Ctrl+Shift+E"),
    c("a_convert", "Convert GSM to ER", General, ""),
    c("a_copy_input", "Copy \\input line", General, "Ctrl+Shift+C"),
    c("a_quit", "Quit", General, "Ctrl+Q"),
    c("a_undo", "Undo", General, "Ctrl+Z"),
    c("a_redo", "Redo", General, "Ctrl+Shift+Z; Ctrl+Y"),
    c("a_cut", "Cut", General, "Ctrl+X"),
    c("a_copy", "Copy", General, "Ctrl+C"),
    c("a_paste", "Paste", General, "Ctrl+V"),
    c("a_dup", "Duplicate", General, "Ctrl+D"),
    c("a_all", "Select all", General, "Ctrl+A"),
    c("a_add_attributes", "Add attribute list (to the selected element)", General, "Ctrl+L"),
    c("a_arrange", "Arrange diagram", General, ""),
    c("a_arrange_selection", "Arrange selection", General, ""),
    c("a_zoom_in", "Zoom in", General, "Ctrl++"),
    c("a_zoom_out", "Zoom out", General, "Ctrl+-"),
    c("a_zoom_100", "Actual size", General, "Ctrl+0"),
    c("a_fit", "Fit diagram", General, "Ctrl+F"),
    c("a_grid", "Show grid", General, "Ctrl+G"),
    c("a_types", "Show data types", General, ""),
    c("a_preview", "LaTeX preview", General, "F6"),
    c("a_preferences", "Preferences", General, "Ctrl+,"),
    c("a_about", "About", General, ""),
    c("a_keys", "Mouse and keyboard help", General, "F1"),
    c("a_rulebook", "Rule books", General, "Shift+F1"),
    c("a_save_packages", "Save LaTeX packages", General, ""),
    // the tools: the nine most used ones on Ctrl+1 to Ctrl+9
    c("er.select", "Select", Er, "Ctrl+1"),
    c("er.link", "Line", Er, "Ctrl+2"),
    c("er.entity", "Entity", Er, "Ctrl+3"),
    c("er.relationship", "Relationship", Er, "Ctrl+4"),
    c("er.attribute tree", "Attribute list", Er, "Ctrl+5"),
    c("er.is a", "IS_A", Er, "Ctrl+6"),
    c("er.weak entity", "Weak entity", Er, "Ctrl+7"),
    c("er.identification dependency", "Id dependency", Er, "Ctrl+8"),
    c("er.existence dependency", "E dependency", Er, "Ctrl+9"),
    c("er.weak relationship", "Weak relationship", Er, "Ctrl+Shift+1"),
    c("er.can be", "CAN_BE", Er, "Ctrl+Shift+2"),
    c("er.part of", "PART_OF", Er, "Ctrl+Shift+3"),
    c("er.chen attribute", "Chen attribute", Er, "Ctrl+Shift+4"),
    c("er.note", "Note", Er, "Ctrl+Shift+5"),
    c("er.subdomain", "Subdomain", Er, "Ctrl+Shift+6"),
    c("er.point", "Junction point", Er, "Ctrl+Shift+7"),
    c("gsm.select", "Select", Gsm, "Ctrl+1"),
    c("gsm.attribute", "Attribute line", Gsm, "Ctrl+2"),
    c("gsm.abstract type", "Abstract type", Gsm, "Ctrl+3"),
    c("gsm.representable type", "Representable type", Gsm, "Ctrl+4"),
    c("gsm.subtype", "Subtype", Gsm, "Ctrl+5"),
    c("gsm.aggregation", "Aggregation", Gsm, "Ctrl+6"),
    c("gsm.grouping", "Grouping", Gsm, "Ctrl+7"),
    c("gsm.generalization", "Generalization", Gsm, "Ctrl+8"),
    c("gsm.specialization", "Specialization", Gsm, "Ctrl+9"),
    c("gsm.multi-valued attribute", "Multi-valued attribute line", Gsm, "Ctrl+Shift+1"),
    c("gsm.component", "Component line", Gsm, "Ctrl+Shift+2"),
    c("gsm.link", "Plain line", Gsm, "Ctrl+Shift+3"),
    c("gsm.note", "Note", Gsm, "Ctrl+Shift+4"),
    c("gsm.subdomain", "Subdomain", Gsm, "Ctrl+Shift+5"),
    c("gsm.point", "Junction point", Gsm, "Ctrl+Shift+6"),
    // handled by the canvas or a list, only while it has the focus
    c("delete", "Delete the selection", Canvas, "Del; Backspace"),
    c("cancel", "Cancel, back to the select tool", Canvas, "Esc"),
    c("nudge left", "Nudge left (with Shift: 1 cm, Alt: 0.05 cm)", Canvas, "Left"),
    c("nudge right", "Nudge right (with Shift: 1 cm, Alt: 0.05 cm)", Canvas, "Right"),
    c("nudge up", "Nudge up (with Shift: 1 cm, Alt: 0.05 cm)", Canvas, "Up"),
    c("nudge down", "Nudge down (with Shift: 1 cm, Alt: 0.05 cm)", Canvas, "Down"),
    c("pan", "Hold to pan with the mouse", Canvas, "Space"),
    c("next attribute", "Next attribute, while typing a name", Canvas, "Shift+Return; Shift+Enter"),
];

/// A command by its id.
pub fn by_id(id: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.id == id)
}

/// Qt's names of keys: (the names it reads, lower case; the name it writes).
const KEY_NAMES: &[(&[&str], &str)] = &[
    (&["space"], "Space"),
    (&["esc", "escape"], "Esc"),
    (&["tab"], "Tab"),
    (&["backtab"], "Backtab"),
    (&["backspace"], "Backspace"),
    (&["return"], "Return"),
    (&["enter"], "Enter"),
    (&["ins", "insert"], "Ins"),
    (&["del", "delete"], "Del"),
    (&["pause"], "Pause"),
    (&["print"], "Print"),
    (&["sysreq"], "SysReq"),
    (&["home"], "Home"),
    (&["end"], "End"),
    (&["left"], "Left"),
    (&["up"], "Up"),
    (&["right"], "Right"),
    (&["down"], "Down"),
    (&["pgup"], "PgUp"),
    (&["pgdown"], "PgDown"),
    (&["capslock"], "CapsLock"),
    (&["numlock"], "NumLock"),
    (&["scrolllock"], "ScrollLock"),
    (&["menu"], "Menu"),
    (&["help"], "Help"),
];

/// One key combination (without `, `) as Qt writes it; `None` if Qt can't read it.
fn canonical_key(text: &str) -> Option<String> {
    let text: String = text.chars().filter(|c| *c != ' ').collect();
    let lower = text.to_lowercase();
    let mut rest = text.as_str();
    let mut rest_lower = lower.as_str();
    let (mut meta, mut ctrl, mut alt, mut shift) = (false, false, false, false);
    loop {
        let mut found = false;
        for (name, flag) in
            [("meta+", &mut meta), ("ctrl+", &mut ctrl), ("alt+", &mut alt), ("shift+", &mut shift)]
        {
            if let Some(r) = rest_lower.strip_prefix(name)
                && !r.is_empty()
            {
                *flag = true;
                rest_lower = r;
                rest = rest.get(name.len()..).unwrap_or("");
                found = true;
            }
        }
        if !found {
            break;
        }
    }
    let mut chars = rest.chars();
    let key = match (chars.next(), chars.next()) {
        (None, _) => return None,
        (Some(c), None) => {
            let mut up = c.to_uppercase();
            match (up.next(), up.next()) {
                (Some(u), None) => u.to_string(),
                _ => c.to_string(),
            }
        }
        _ => {
            if let Some(n) = rest_lower
                .strip_prefix('f')
                .and_then(|n| n.parse::<u32>().ok())
                .filter(|n| (1..=35).contains(n))
            {
                format!("F{n}")
            } else {
                KEY_NAMES.iter().find(|(names, _)| names.contains(&rest_lower)).map(|(_, n)| n.to_string())?
            }
        }
    };
    let mut out = String::new();
    for (on, name) in [(meta, "Meta+"), (ctrl, "Ctrl+"), (alt, "Alt+"), (shift, "Shift+")] {
        if on {
            out.push_str(name);
        }
    }
    Some(out + &key)
}

/// A key sequence (up to four combinations, separated by `, `) as Qt writes it in portable
/// form; empty if Qt can't read it.
pub fn canonical(sequence: &str) -> String {
    let keys: Option<Vec<String>> =
        crate::text::strip(sequence).split(", ").take(4).map(canonical_key).collect();
    keys.map(|k| k.join(", ")).unwrap_or_default()
}

/// The combinations of a binding (`a; b`), each as Qt writes it.
pub fn parse_keys(text: &str) -> Vec<String> {
    text.split(';').map(crate::text::strip).filter(|t| !t.is_empty()).map(canonical).collect()
}

/// Could both apply at the same moment?  Only one set of tools is active at a time.
pub fn clashes_with(a: &Command, b: &Command) -> bool {
    let groups = [a.group, b.group];
    a.id != b.id && !(groups.contains(&Group::Er) && groups.contains(&Group::Gsm))
}

/// Which other command already uses one of these combinations?
pub fn find_conflict(
    bindings: &HashMap<String, String>,
    command_id: &str,
    keys: &str,
) -> Option<&'static Command> {
    let wanted: BTreeSet<String> = parse_keys(keys).into_iter().collect();
    let command = by_id(command_id)?;
    COMMANDS.iter().filter(|other| clashes_with(command, other)).find(|other| {
        let used: BTreeSet<String> =
            parse_keys(bindings.get(other.id).map_or("", String::as_str)).into_iter().collect();
        !wanted.is_disjoint(&used)
    })
}

/// The bindings with nothing changed.
pub fn defaults() -> HashMap<String, String> {
    COMMANDS.iter().map(|c| (c.id.to_string(), c.default.to_string())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_qt_reads_them() {
        assert_eq!(canonical("Shift+Ctrl+Alt+Meta+A"), "Meta+Ctrl+Alt+Shift+A");
        assert_eq!(canonical("ctrl+a"), "Ctrl+A");
        assert_eq!(canonical("Ctrl++"), "Ctrl++");
        assert_eq!(canonical("Delete"), "Del");
        assert_eq!(canonical("  Ctrl + A "), "Ctrl+A");
        assert_eq!(canonical("Ctrl+ä"), "Ctrl+Ä");
        assert_eq!(canonical("bogus"), "");
        assert_eq!(canonical("Ctrl+"), "");
        assert_eq!(canonical("f5"), "F5");
        assert_eq!(canonical("Ctrl+Shift+Z, Ctrl+Y"), "Ctrl+Shift+Z, Ctrl+Y");
        let b = defaults();
        assert_eq!(find_conflict(&b, "a_save", "Ctrl+O").map(|c| c.id), Some("a_open"));
        assert_eq!(find_conflict(&b, "er.select", "Ctrl+2").map(|c| c.id), Some("er.link"));
        assert_eq!(find_conflict(&b, "a_save", "Ctrl+K"), None);
    }
}
