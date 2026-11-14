//! The Problems panel: what the rules find and what the layout couldn't draw, with their fixes;
//! and the badges on the canvas.  From `mainwindow.py` (`collect_problems`, `on_changed`,
//! `describe`) and `canvas.py` (`paint_badge`).

use std::fmt::Write;

use crate::editor::Editor;
use crate::model::{Body, Element, ElementKey};
use crate::rules::{self, Problem, Severity};

/// The colours of the three severities (as the Python app's canvas).
pub fn severity_color(s: Severity) -> &'static str {
    match s {
        Severity::Error => "rgb(215,35,35)",
        Severity::Warning => "rgb(232,135,0)",
        Severity::Hint => "rgb(45,125,220)",
    }
}

/// The rule check, and everything the layout could not draw.
pub fn collect(ed: &mut Editor) -> Vec<Problem> {
    let revision = ed.session().revision();
    if let Some((r, probs)) = &ed.problem_cache
        && *r == revision
    {
        return probs.clone();
    }
    let mut probs = rules::check(ed.doc());
    for (element, w) in ed.session_mut().layout_warnings() {
        let severity = if w.starts_with("cannot") || w.starts_with("unknown") {
            Severity::Error
        } else {
            Severity::Hint
        };
        probs.push(Problem { element, severity, message: w, fix: None, rule: "" });
    }
    ed.problem_cache = Some((revision, probs.clone()));
    probs
}

/// A short description of an element: `entity 'student'`, `link a → b`.
pub fn describe(el: &Element) -> String {
    match &el.body {
        Body::Node(n) if n.generalisation.is_some() => format!(
            "{} '{}' → {}",
            n.command.to_uppercase(),
            n.id,
            n.generalisation.as_ref().map_or("", |g| g.supertype.as_str())
        ),
        Body::Node(n) => format!("{} '{}'", n.command, n.id),
        Body::Edge(e) => format!("{} {} → {}", e.command, e.src, e.dst),
        Body::Raw(r) if !r.command.is_empty() => format!("raw TeX \\{}", r.command),
        Body::Raw(_) => "raw text".into(),
        Body::Point(_) => "point".into(),
        Body::Attributes(_) => "attributetree".into(),
        Body::Note(_) => "note".into(),
        Body::Subdomain(_) => "subdomain".into(),
        Body::Grid(_) => "grid".into(),
        Body::Setup(_) => "setup".into(),
    }
}

/// One line of the list.
#[derive(Clone, Debug, PartialEq)]
pub struct Listed {
    pub element: ElementKey,
    pub severity: Severity,
    pub text: String,
    /// the fix's title
    pub fix: Option<String>,
    /// the rule in the rule books, empty where none is written yet
    pub rule: &'static str,
}

/// What the panel lists: the problems shown (hints only if wanted), a summary such as
/// "1 error, 0 warnings, 2 hints", and the number of errors and warnings (for its title).
pub fn listed(ed: &mut Editor, hints: bool) -> (Vec<Listed>, String, usize) {
    let probs = collect(ed);
    let doc = ed.doc();
    let list = probs
        .iter()
        .filter(|p| hints || p.severity != Severity::Hint)
        .map(|p| {
            let mut text = format!("{}: {}", doc.get(p.element).map(describe).unwrap_or_default(), p.message);
            if let Some(f) = &p.fix {
                let _ = write!(text, "   [{}]", f.title);
            }
            Listed {
                element: p.element,
                severity: p.severity,
                text,
                fix: p.fix.as_ref().map(|f| f.title.clone()),
                rule: p.rule,
            }
        })
        .collect();
    let count = |s: Severity| probs.iter().filter(|p| p.severity == s).count();
    let (e, w, h) = (count(Severity::Error), count(Severity::Warning), count(Severity::Hint));
    let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
    let summary = if e + w + h == 0 {
        "No problems found.".to_string()
    } else {
        format!("{}, {}, {}", plural(e, "error"), plural(w, "warning"), plural(h, "hint"))
    };
    (list, summary, e + w)
}

fn shown(ed: &mut Editor, hints: bool) -> Vec<Problem> {
    collect(ed).into_iter().filter(|p| hints || p.severity != Severity::Hint).collect()
}

/// Applies the fix of the n-th problem listed.
pub fn fix(ed: &mut Editor, n: usize, hints: bool) -> bool {
    let Some(p) = shown(ed, hints).into_iter().nth(n) else { return false };
    let Some(fix) = p.fix else { return false };
    ed.apply(&fix.title.clone(), |ed| fix.apply(ed.session_mut().doc_mut(), p.element))
}

/// Applies every fix listed, as one step; how many.
pub fn fix_all(ed: &mut Editor, hints: bool) -> usize {
    let fixable: Vec<Problem> = shown(ed, hints).into_iter().filter(|p| p.fix.is_some()).collect();
    if fixable.is_empty() {
        return 0;
    }
    ed.apply("Fix problems", |ed| {
        for p in &fixable {
            if let Some(f) = &p.fix
                && ed.doc().get(p.element).is_some()
            {
                f.apply(ed.session_mut().doc_mut(), p.element);
            }
        }
    });
    fixable.len()
}

/// The problems of one element, for its tooltip on the canvas.
pub fn tooltip(ed: &mut Editor, key: ElementKey, hints: bool) -> String {
    shown(ed, hints)
        .iter()
        .filter(|p| p.element == key)
        .map(|p| {
            let s = p.severity.as_str();
            let mut c = s.chars();
            let cap = c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default();
            format!("{cap}: {}", p.message)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The worst severity of each element with problems shown.
pub fn worst(ed: &mut Editor, hints: bool) -> Vec<(ElementKey, Severity)> {
    let mut out: Vec<(ElementKey, Severity)> = Vec::new();
    for p in shown(ed, hints) {
        match out.iter_mut().find(|(k, _)| *k == p.element) {
            Some((_, s)) => *s = (*s).min(p.severity),
            None => out.push((p.element, p.severity)),
        }
    }
    out
}

/// Selects the element of the n-th problem listed; its centre in pt, to scroll to.
pub fn goto(ed: &mut Editor, n: usize, hints: bool) -> Option<(f64, f64)> {
    let p = shown(ed, hints).into_iter().nth(n)?;
    ed.doc().get(p.element)?;
    ed.set_selection(vec![p.element]);
    let laid = ed.session_mut().laid();
    let (x0, y0, x1, y1) = laid.gfx_for(p.element).and_then(crate::svg::bbox)?;
    Some(((x0 + x1) / 2.0, (y0 + y1) / 2.0))
}

/// The Source tab: the text of the selected elements, each with its problems as comments.
pub fn source(ed: &mut Editor) -> String {
    let probs = collect(ed);
    let doc = ed.doc();
    let mut lines = Vec::new();
    for key in &ed.selection {
        let Some(el) = doc.get(*key) else { continue };
        lines.push(el.text());
        lines.extend(
            probs
                .iter()
                .filter(|p| p.element == *key)
                .map(|p| format!("% {}: {}", p.severity.as_str(), p.message)),
        );
    }
    lines.join("\n")
}
