//! What a subdomain box contains.
//!
//! An element belongs to a box if its centre is inside, and to the innermost box when they are
//! nested.  This follows from the geometry alone, so nothing needs storing: moving an element
//! out of a box is all it takes.

use std::collections::HashMap;

use crate::document::Document;
use crate::geometry::Pt;
use crate::latex_text::plain_text;
use crate::layout::Layout;
use crate::model::{Body, Element, ElementKey};
use crate::text::is_space;

/// A subdomain's outline.
#[derive(Clone, Debug, PartialEq)]
pub struct SubdomainBox {
    pub element: ElementKey,
    pub label: String,
    pub corners: Vec<Pt>,
    pub area: f64,
}

impl SubdomainBox {
    /// Whether a point is inside the outline (even-odd rule).
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let mut inside = false;
        let pts = &self.corners;
        for (k, &(x1, y1)) in pts.iter().enumerate() {
            let Some(&(x2, y2)) = pts.get((k + 1) % pts.len()) else { continue };
            if (y1 > y) != (y2 > y) && x < x1 + (y - y1) * (x2 - x1) / (y2 - y1) {
                inside = !inside;
            }
        }
        inside
    }

    /// Whether this box lies within a larger one.
    pub fn within(&self, other: &SubdomainBox) -> bool {
        other.element != self.element
            && other.area > self.area
            && self.corners.iter().all(|&(x, y)| other.contains(x, y))
    }

    /// The box's name on one line.
    pub fn title(&self) -> String {
        let t = plain_text(&self.label);
        let words: Vec<&str> = t.split(is_space).filter(|w| !w.is_empty()).collect();
        if words.is_empty() { "(no name)".into() } else { words.join(" ") }
    }
}

/// The boxes of a document whose outline can be evaluated.
pub fn boxes_of(doc: &Document, layout: &Layout<'_>) -> Vec<SubdomainBox> {
    let mut out = Vec::new();
    for el in &doc.elements {
        let Body::Subdomain(s) = &el.body else { continue };
        let Some(mut pts) = s.points.iter().map(|c| layout.coord(Some(c))).collect::<Option<Vec<Pt>>>()
        else {
            continue;
        };
        if pts.len() < 2 {
            continue;
        }
        if let [(x1, y1), (x2, y2)] = pts.as_slice() {
            pts = vec![(*x1, *y1), (*x2, *y1), (*x2, *y2), (*x1, *y2)];
        }
        let n = pts.len();
        let twice = crate::text::py_sum(
            (0..n)
                .filter_map(|k| Some((pts.get(k)?, pts.get((k + 1) % n)?)))
                .map(|(a, b)| a.0 * b.1 - b.0 * a.1),
        );
        out.push(SubdomainBox {
            element: el.key,
            label: s.label.clone(),
            corners: pts,
            area: twice.abs() / 2.0,
        });
    }
    out
}

/// The centre that decides which box an element is in.
pub fn centre_of(e: &Element, layout: &Layout<'_>) -> Option<Pt> {
    match &e.body {
        Body::Node(n) => layout.shapes.get(&n.id).map(|s| (s.cx, s.cy)),
        Body::Point(p) => layout.coord(p.pos.as_ref()),
        Body::Note(n) => layout.coord(n.pos.as_ref()),
        _ => None,
    }
}

/// All that moves with the box: what lies inside, with boxes inside of it and their contents.
pub fn members(
    b: &SubdomainBox,
    doc: &Document,
    layout: &Layout<'_>,
    boxes: &[SubdomainBox],
) -> Vec<ElementKey> {
    let mut out: Vec<ElementKey> = doc
        .elements
        .iter()
        .filter(|e| centre_of(e, layout).is_some_and(|(x, y)| b.contains(x, y)))
        .map(|e| e.key)
        .collect();
    out.extend(boxes.iter().filter(|o| o.within(b)).map(|o| o.element));
    out
}

/// The smallest box around a point.
pub fn innermost(boxes: &[SubdomainBox], x: f64, y: f64) -> Option<&SubdomainBox> {
    boxes.iter().filter(|b| b.contains(x, y)).min_by(|a, b| a.area.total_cmp(&b.area))
}

/// `Outer / Inner` for a box within another.
pub fn path_title(b: &SubdomainBox, boxes: &[SubdomainBox]) -> String {
    let mut around: Vec<&SubdomainBox> = boxes.iter().filter(|o| b.within(o)).collect();
    around.sort_by(|p, q| q.area.total_cmp(&p.area));
    around.push(b);
    around.iter().map(|o| o.title()).collect::<Vec<_>>().join(" / ")
}

/// Node id -> the title of its box, for what lies in one.
pub fn titles_by_element(doc: &Document, layout: &Layout<'_>) -> HashMap<String, String> {
    let boxes = boxes_of(doc, layout);
    let mut out = HashMap::new();
    for e in &doc.elements {
        if let Body::Node(n) = &e.body
            && let Some((x, y)) = centre_of(e, layout)
            && let Some(b) = innermost(&boxes, x, y)
        {
            out.insert(n.id.clone(), path_title(b, &boxes));
        }
    }
    out
}
