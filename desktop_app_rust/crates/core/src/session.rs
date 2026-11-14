//! One open diagram: its document, its undo history, and its last layout.  The window keeps
//! one session per tab (inside an [`Editor`](crate::editor::Editor)); every change is one
//! undoable edit, made with [`Session::edit`], or between [`Session::begin`] and
//! [`Session::end`] when it takes several events (dragging).

use std::collections::HashMap;

use crate::document::{Document, Kind};
use crate::fonts::Metrics;
use crate::geometry::Shape;
use crate::history::{History, SelKey, Snapshot};
use crate::layout::{FontEngine, Gfx, Layout};
use crate::style::LabelLanguage;
use crate::texscan::ParseError;

/// What the layout made: the drawing of each element, and the shapes of the nodes.
#[derive(Clone, Debug, Default)]
pub struct Laid {
    pub gfx: Vec<Gfx>,
    pub shapes: HashMap<String, Shape>,
}

impl Laid {
    /// The drawing of an element.
    pub fn gfx_for(&self, key: crate::model::ElementKey) -> Option<&Gfx> {
        self.gfx.iter().find(|g| g.key == Some(key))
    }
}

/// A document with its history.
#[derive(Clone, Debug)]
pub struct Session {
    doc: Document,
    history: History,
    language: LabelLanguage,
    /// the last layout, until the document changes
    laid: Option<Laid>,
    /// the state before the edit in progress
    pending: Option<Snapshot>,
    /// counts every change of the document (for the page, to know when to draw again)
    revision: u64,
}

impl Session {
    /// A session for a document, as saved.
    pub fn new(doc: Document) -> Session {
        Session {
            doc,
            history: History::new(),
            language: LabelLanguage::English,
            laid: None,
            pending: None,
            revision: 0,
        }
    }

    /// A session for the text of a file.
    pub fn open(text: &str, kind: Kind) -> Result<Session, ParseError> {
        Ok(Session::new(Document::parse(text, kind)?))
    }

    pub fn doc(&self) -> &Document {
        &self.doc
    }

    /// The document, to be changed: the layout is made again when it's next needed.  Use it
    /// between [`Session::begin`] and [`Session::end`], so that the change can be undone.
    pub fn doc_mut(&mut self) -> &mut Document {
        self.laid = None;
        self.revision += 1;
        &mut self.doc
    }

    /// Counts the changes of the document.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The file text.
    pub fn text(&self) -> String {
        self.doc.to_text()
    }

    /// Starts an edit: the state to go back to (nothing happens if one is in progress).
    pub fn begin(&mut self, selection: Vec<SelKey>) {
        if self.pending.is_none() {
            self.pending = Some(Snapshot { text: self.doc.to_text(), selection });
        }
    }

    /// Ends the edit as one step called `label`; returns whether anything changed.
    pub fn end(&mut self, label: &str, selection: Vec<SelKey>) -> bool {
        let Some(before) = self.pending.take() else { return false };
        let after = Snapshot { text: self.doc.to_text(), selection };
        if after.text == before.text {
            return false;
        }
        self.history.push(label, before, after);
        true
    }

    /// Takes the edit in progress back; returns the selection from before it.
    pub fn cancel(&mut self) -> Option<Vec<SelKey>> {
        let before = self.pending.take()?;
        self.restore(&before);
        Some(before.selection)
    }

    /// Whether an edit is in progress.
    pub fn editing(&self) -> bool {
        self.pending.is_some()
    }

    /// Changes the document as one undoable edit called `label`.  Returns whether anything
    /// changed (nothing is recorded otherwise).
    pub fn edit(&mut self, label: &str, change: impl FnOnce(&mut Document)) -> bool {
        self.begin(vec![]);
        change(self.doc_mut());
        self.end(label, vec![])
    }

    fn restore(&mut self, s: &Snapshot) {
        // the text was written from this document, so it reads back; keep the old one if not
        if let Ok(doc) = Document::parse(&s.text, self.doc.kind) {
            self.doc = doc;
            self.laid = None;
            self.revision += 1;
        }
    }

    /// Goes back one edit; returns what was selected then.
    pub fn undo(&mut self) -> Option<Vec<SelKey>> {
        let s = self.history.undo().cloned()?;
        self.restore(&s);
        Some(s.selection)
    }

    /// Goes forward one edit; returns what was selected then.
    pub fn redo(&mut self) -> Option<Vec<SelKey>> {
        let s = self.history.redo().cloned()?;
        self.restore(&s);
        Some(s.selection)
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.history.undo_label()
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.history.redo_label()
    }

    /// Whether the document is as saved.
    pub fn is_clean(&self) -> bool {
        self.history.is_clean()
    }

    /// The document was saved: unchanged elements are written back verbatim from now on.
    pub fn mark_saved(&mut self) {
        self.doc.mark_saved();
        self.history.set_clean();
    }

    /// Another document in this session (reloaded from disk): the history starts anew.
    pub fn replace(&mut self, doc: Document) {
        self.doc = doc;
        self.history.clear();
        self.pending = None;
        self.laid = None;
        self.revision += 1;
    }

    /// The language of keywords in the drawing (IS_A or IST_EIN, ...).
    pub fn language(&self) -> LabelLanguage {
        self.language
    }

    pub fn set_language(&mut self, language: LabelLanguage) {
        if language != self.language {
            self.language = language;
            self.laid = None;
            self.revision += 1;
        }
    }

    /// The layout, made again if the document changed.  An error in the steps after the
    /// elements (as in the Python app) leaves the drawing empty.
    pub fn laid(&mut self) -> &Laid {
        let language = self.language;
        let doc = &self.doc;
        self.laid.get_or_insert_with(|| {
            let engine = FontEngine { metrics: Metrics::DesignSizes };
            match Layout::new(doc, &engine, language).run() {
                Ok(l) => Laid { gfx: l.gfx, shapes: l.shapes },
                Err(_) => Laid::default(),
            }
        })
    }

    /// The document and its layout together.
    pub fn parts(&mut self) -> (&Document, &Laid) {
        self.laid();
        let Session { doc, laid, .. } = self;
        static EMPTY: std::sync::OnceLock<Laid> = std::sync::OnceLock::new();
        (doc, laid.as_ref().unwrap_or_else(|| EMPTY.get_or_init(Laid::default)))
    }

    /// The layout's elements.
    pub fn gfx(&mut self) -> &[Gfx] {
        &self.laid().gfx
    }

    /// Runs `f` with a layout that knows the shapes, for evaluating coordinates (`(a.north)`).
    pub fn with_layout<T>(&mut self, f: impl FnOnce(&Document, &Layout<'_>) -> T) -> T {
        let shapes = self.laid().shapes.clone();
        let engine = FontEngine { metrics: Metrics::DesignSizes };
        let mut layout = Layout::new(&self.doc, &engine, self.language);
        layout.shapes = shapes;
        f(&self.doc, &layout)
    }

    /// The extent of the drawing (x0, y0, x1, y1), if there is anything.
    pub fn bounds(&mut self) -> Option<(f64, f64, f64, f64)> {
        self.gfx()
            .iter()
            .filter_map(crate::svg::bbox)
            .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
    }

    /// The warnings of the layout: (element, message), e.g. "cannot draw: ...".
    pub fn layout_warnings(&mut self) -> Vec<(crate::model::ElementKey, String)> {
        self.gfx()
            .iter()
            .filter_map(|g| g.key.map(|k| (k, g)))
            .flat_map(|(k, g)| g.warnings.iter().map(move |w| (k, w.clone())))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_undo_redo_and_clean() {
        let text = "\\entity{a}(0,0){A} % mine\n\\attributes{a}{*k : integer}\n";
        let mut s = Session::open(text, Kind::Er).unwrap();
        assert!(s.is_clean() && s.undo().is_none());
        assert!(!s.edit("nothing", |_| {}));
        assert!(s.edit("Show data types", |d| d.set_types_shown(true)));
        assert!(!s.is_clean());
        assert!(s.text().contains("\\setup{show types}"));
        assert_eq!(s.undo_label(), Some("Show data types"));
        assert!(s.undo().is_some());
        assert_eq!(s.text(), text);
        assert!(s.is_clean());
        assert!(s.redo().is_some());
        s.mark_saved();
        assert!(s.is_clean());
        // an edit over several events, taken back
        s.begin(vec![SelKey::Id("a".into())]);
        s.doc_mut().set_types_shown(false);
        assert_eq!(s.cancel(), Some(vec![SelKey::Id("a".into())]));
        assert!(s.text().contains("show types"));
        assert!(!s.gfx().is_empty());
    }
}
