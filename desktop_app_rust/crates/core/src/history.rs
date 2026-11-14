//! Undo and redo: snapshots of the file text, which round-trips losslessly (as the Python app
//! does with Qt's undo stack).  An entry holds the state before and after one edit.

/// How a selected element is found again in the document read back from a snapshot: by its
/// id, or as the n-th element of its kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelKey {
    Id(String),
    Nth(&'static str, usize),
}

/// The state of a document at one moment, and what was selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub text: String,
    pub selection: Vec<SelKey>,
}

#[derive(Clone, Debug)]
struct Entry {
    label: String,
    before: Snapshot,
    after: Snapshot,
}

/// The edits of one document.
#[derive(Clone, Debug, Default)]
pub struct History {
    entries: Vec<Entry>,
    /// how many entries are applied
    index: usize,
    /// the index at which the document equals its file; `None` once that state can't come back
    clean: Option<usize>,
}

impl History {
    /// An empty history; the document is as saved.
    pub fn new() -> History {
        History { entries: vec![], index: 0, clean: Some(0) }
    }

    /// Records an edit.  What could be redone is dropped.
    pub fn push(&mut self, label: &str, before: Snapshot, after: Snapshot) {
        self.entries.truncate(self.index);
        if self.clean.is_some_and(|c| c > self.index) {
            self.clean = None;
        }
        self.entries.push(Entry { label: label.to_string(), before, after });
        self.index += 1;
    }

    /// The state to go back to, if there is one.
    pub fn undo(&mut self) -> Option<&Snapshot> {
        let i = self.index.checked_sub(1)?;
        self.index = i;
        self.entries.get(i).map(|e| &e.before)
    }

    /// The state to go forward to, if there is one.
    pub fn redo(&mut self) -> Option<&Snapshot> {
        let e = self.entries.get(self.index)?;
        self.index += 1;
        Some(&e.after)
    }

    /// What Undo would undo.
    pub fn undo_label(&self) -> Option<&str> {
        self.index.checked_sub(1).and_then(|i| self.entries.get(i)).map(|e| e.label.as_str())
    }

    /// What Redo would redo.
    pub fn redo_label(&self) -> Option<&str> {
        self.entries.get(self.index).map(|e| e.label.as_str())
    }

    /// Whether the document is as saved.
    pub fn is_clean(&self) -> bool {
        self.clean == Some(self.index)
    }

    /// The document was just saved (or loaded).
    pub fn set_clean(&mut self) {
        self.clean = Some(self.index);
    }

    /// Forgets everything (another document, or reloaded).
    pub fn clear(&mut self) {
        *self = History::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(t: &str) -> Snapshot {
        Snapshot { text: t.into(), selection: vec![] }
    }

    #[test]
    fn as_qt_undo_stack() {
        let mut h = History::new();
        assert!(h.is_clean() && h.undo().is_none());
        h.push("A", s("0"), s("1"));
        h.push("B", s("1"), s("2"));
        assert!(!h.is_clean());
        assert_eq!(h.undo_label(), Some("B"));
        assert_eq!(h.undo(), Some(&s("1")));
        assert_eq!(h.redo_label(), Some("B"));
        assert_eq!(h.undo(), Some(&s("0")));
        assert!(h.is_clean());
        assert_eq!(h.redo(), Some(&s("1")));
        h.set_clean();
        h.undo();
        h.push("C", s("0"), s("3")); // the saved state can't come back
        assert!(!h.is_clean());
        assert!(h.redo().is_none());
        h.undo();
        assert!(!h.is_clean());
    }
}
