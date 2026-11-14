//! Editing: the Python app's editor tests (tests/test_editor.py and the editing parts of
//! test_tabs.py), driven with the same clicks and drags, in diagram coordinates.

use diagram_core::document::{Document, Kind};
use diagram_core::editor::{Editor, Event, KIND_MARK, Mods};
use diagram_core::geometry::CM;
use diagram_core::layout::Handle;
use diagram_core::model::{Body, Coord, ElementKey};
use diagram_core::session::Session;

fn editor(src: &str, kind: Kind) -> Editor {
    let mut ed = Editor::new(Session::new(Document::parse(src, kind).unwrap()));
    ed.set_scale(72.27 / 96.0); // 100 %: pt per pixel
    ed
}

fn none() -> Mods {
    Mods::default()
}

fn click(ed: &mut Editor, x: f64, y: f64) {
    ed.press(x * CM, y * CM, none());
    ed.release(x * CM, y * CM, none());
}

fn drag(ed: &mut Editor, a: (f64, f64), b: (f64, f64), mods: Mods) {
    ed.press(a.0 * CM, a.1 * CM, mods);
    for k in 1..=5 {
        let t = f64::from(k) / 5.0;
        ed.move_to((a.0 + t * (b.0 - a.0)) * CM, (a.1 + t * (b.1 - a.1)) * CM, mods);
    }
    ed.release(b.0 * CM, b.1 * CM, mods);
}

fn text(ed: &Editor) -> String {
    ed.session().text()
}

fn nodes(ed: &Editor) -> Vec<(String, String, Option<Coord>)> {
    ed.doc().nodes().map(|n| (n.command.clone(), n.id.clone(), n.pos.clone())).collect()
}

fn edges(ed: &Editor) -> Vec<(String, String, bool)> {
    ed.doc().edges().map(|e| (e.src.clone(), e.dst.clone(), e.opts.contains("double"))).collect()
}

fn messages(ed: &mut Editor) -> String {
    ed.take_events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Message(m) | Event::Refused(m) => Some(m),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn build_er() -> Editor {
    let mut ed = editor("", Kind::Er);
    ed.set_tool("entity");
    click(&mut ed, 0.0, 0.0);
    ed.set_tool("relationship");
    click(&mut ed, 4.0, 0.0);
    ed.set_tool("entity");
    click(&mut ed, 8.0, 0.0);
    ed.set_tool("link");
    drag(&mut ed, (0.0, 0.0), (4.0, 0.0), none());
    drag(&mut ed, (4.0, 0.0), (8.0, 0.0), none());
    ed.set_tool("select");
    ed
}

#[test]
fn place_and_connect() {
    let ed = build_er();
    let n = nodes(&ed);
    assert_eq!(n.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["entity", "relationship", "entity"]);
    assert_eq!(
        n.iter().map(|x| x.2.clone()).collect::<Vec<_>>(),
        [Some(Coord::numeric(0.0, 0.0)), Some(Coord::numeric(4.0, 0.0)), Some(Coord::numeric(8.0, 0.0))]
    );
    let e = edges(&ed);
    assert_eq!(
        e.iter().map(|x| (x.0.as_str(), x.1.as_str())).collect::<Vec<_>>(),
        [("entity", "relates"), ("relates", "entity2")]
    );
}

#[test]
fn entity_entity_link_is_refused() {
    let mut ed = build_er();
    ed.take_events();
    ed.set_tool("link");
    drag(&mut ed, (0.0, 0.0), (8.0, 0.0), none());
    assert_eq!(edges(&ed).len(), 2);
    assert!(messages(&mut ed).contains("relationship"));
}

#[test]
fn move_with_snapping_and_undo() {
    let mut ed = build_er();
    drag(&mut ed, (8.0, 0.0), (9.13, 1.9), none());
    assert!(text(&ed).contains("\\entity{entity2}(9.25,2){Entity}"), "{}", text(&ed)); // 0.25 cm snapping
    assert_eq!(ed.session().undo_label(), Some("Move"));
    ed.undo();
    assert!(text(&ed).contains("\\entity{entity2}(8,0){Entity}"));
    assert_eq!(ed.selection.len(), 1, "the moved entity is selected again");
    ed.redo();
    assert!(text(&ed).contains("\\entity{entity2}(9.25,2){Entity}"));
    // Alt: without snapping
    drag(&mut ed, (9.25, 2.0), (10.13, 2.0), Mods { alt: true, ..none() });
    assert!(text(&ed).contains("\\entity{entity2}(10.13,2){Entity}"), "{}", text(&ed));
}

fn handle(ed: &mut Editor, name: &str) -> (f64, f64) {
    let h = ed.handles().into_iter().find(|h| h.0 == name).unwrap().1;
    (h.0 / CM, h.1 / CM)
}

#[test]
fn bend_handle_and_waypoints() {
    let mut ed = build_er();
    let edge = ed.doc().elements.iter().find(|e| e.as_edge().is_some()).unwrap().key;
    ed.set_selection(vec![edge]);
    let (mx, my) = handle(&mut ed, "bend");
    drag(&mut ed, (mx, my), (mx, my + 0.6), none());
    let bend: i64 = ed.doc().edges().next().unwrap().opts.get("bend").unwrap().parse().unwrap();
    assert!(bend > 20, "{bend}");
    // a double click adds a way point and removes the bend
    let (px, py) = handle(&mut ed, "bend");
    assert!(ed.double_click(px * CM, py * CM));
    let e = ed.doc().edges().next().unwrap().clone();
    assert!(e.opts.get("bend").is_none());
    assert!(e.opts.get("via").is_some());
    // ... and a double click on the way point removes it
    let (wx, wy) = handle(&mut ed, "wp0");
    assert!(ed.double_click(wx * CM, wy * CM));
    assert!(ed.doc().edges().next().unwrap().opts.get("via").is_none());
}

#[test]
fn relative_position_becomes_absolute_on_drag() {
    let mut ed = editor("\\entity{a}(0,0){A}\n\\entity[right=2cm of a]{b}{B}\n", Kind::Er);
    let b = ed.session_mut().laid().shapes["b"].clone();
    drag(&mut ed, (b.cx / CM, b.cy / CM), (b.cx / CM, b.cy / CM - 1.0), none());
    let n = ed.doc().by_id("b").and_then(|e| e.as_node()).unwrap().clone();
    assert!(!n.opts.contains("right"));
    assert_eq!(n.pos, Some(Coord::numeric(4.75, -1.0))); // snapped
}

#[test]
fn delete_cascades_and_keeps_comments() {
    let mut ed = editor(
        "% people\n\\entity{a}(0,0){A}\n\\relationship{r}(3,0){R}\n\\link{a}{r}{1}\n\\attributes{a}{*x}\n",
        Kind::Er,
    );
    click(&mut ed, 0.0, 0.0);
    assert!(ed.key("delete", none()));
    assert_eq!(text(&ed), "% people\n\\relationship{r}(3,0){R}\n");
}

#[test]
fn copy_paste_and_duplicate() {
    let mut ed = build_er();
    ed.select_all();
    let clip = ed.clipboard_text();
    ed.paste(&clip, None, None);
    assert_eq!(ed.doc().nodes().count(), 6);
    assert_eq!(ed.doc().edges().count(), 4);
    let ids: Vec<String> = ed.doc().nodes().map(|n| n.id.clone()).collect();
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(ids.len(), unique.len());
    // duplicating keeps its offset (1 cm right, 1 cm down)
    let mut ed = editor("\\entity{a}(0,0){A}\n", Kind::Er);
    click(&mut ed, 0.0, 0.0);
    ed.duplicate();
    assert!(text(&ed).contains("\\entity{a2}(1,-1){A}"), "{}", text(&ed));
}

#[test]
fn paste_lands_where_asked_and_says_why_not() {
    let mut ed = editor("\\entity{a}(0,0){A}\n", Kind::Er);
    click(&mut ed, 0.0, 0.0);
    let clip = ed.copy();
    assert!(clip.starts_with(&format!("{KIND_MARK}er\n")));
    ed.paste(&clip, Some((5.1, 2.0)), None);
    assert!(text(&ed).contains("\\entity{a2}(5,2){A}"), "{}", text(&ed)); // at the target, snapped
    // twice at one spot isn't exactly on top
    ed.paste(&clip, Some((5.1, 2.0)), None);
    assert!(text(&ed).contains("\\entity{a3}(5.5,1.5){A}"), "{}", text(&ed));
    // across kinds: refused, with the reason
    let mut gsm = editor("", Kind::Gsm);
    gsm.paste(&clip, Some((0.0, 0.0)), None);
    assert!(messages(&mut gsm).contains("can't be pasted into a GSM diagram"));
    gsm.paste("\\entity{x}(0,0){X}\n", Some((0.0, 0.0)), None);
    assert!(messages(&mut gsm).contains("comes from an ER diagram"));
    // connections without their ends are left out; to something already there, kept
    let mut ed = editor("\\entity{a}(0,0){A}\n", Kind::Er);
    ed.paste("\\relationship{r}(3,0){R}\n\\link{r}{zz}{1}\n\\link{r}{a}{n}\n", Some((3.0, 0.0)), None);
    assert_eq!(edges(&ed), [("r".to_string(), "a".to_string(), false)]);
    assert!(messages(&mut ed).contains("left out 1 connection"));
}

#[test]
fn gsm_isa_rules() {
    let mut ed = editor("", Kind::Gsm);
    ed.set_tool("abstract type");
    click(&mut ed, 0.0, 0.0);
    ed.set_tool("subtype");
    click(&mut ed, 0.0, -3.0);
    click(&mut ed, 4.0, -3.0);
    ed.set_tool("generalization");
    drag(&mut ed, (0.0, 0.0), (0.0, -3.0), none()); // from the abstract type: refused
    assert!(edges(&ed).is_empty());
    drag(&mut ed, (0.0, -3.0), (0.0, 0.0), none()); // subtype -> abstract: fine
    drag(&mut ed, (4.0, -3.0), (0.0, -3.0), none()); // subtype -> subtype: fine
    assert_eq!(edges(&ed).len(), 2);
    drag(&mut ed, (0.0, -3.0), (4.0, -3.0), none()); // would make a cycle: refused
    assert_eq!(edges(&ed).len(), 2);
}

#[test]
fn a_second_weak_entity_leaves_the_direction_open() {
    let mut ed = editor("", Kind::Er);
    ed.set_tool("weak entity");
    click(&mut ed, 0.0, 0.0);
    ed.set_tool("weak relationship");
    click(&mut ed, 4.0, 0.0);
    ed.set_tool("weak entity");
    click(&mut ed, 8.0, 0.0);
    ed.set_tool("link");
    drag(&mut ed, (0.0, 0.0), (4.0, 0.0), none());
    assert!(edges(&ed)[0].2, "the only weak entity so far");
    drag(&mut ed, (4.0, 0.0), (8.0, 0.0), none());
    assert!(edges(&ed).iter().all(|e| !e.2), "only the user knows who depends on whom");
    let sel: Vec<&str> =
        ed.selection.iter().filter_map(|k| ed.doc().get(*k)?.as_node()).map(|n| n.command.as_str()).collect();
    assert_eq!(sel, ["relationship"]);
    ed.undo();
    assert_eq!(edges(&ed).len(), 1);
    assert!(edges(&ed)[0].2);
}

#[test]
fn reconnecting_keeps_the_direction() {
    let src = "\\entity{o}(0,0){O}\\entity[weak]{w2}(0,3){W2}\\entity[weak]{w3}(8,0){W3}\
               \\relationship[weak]{r}(4,0){R}\\link{r}{o}{1}\\link[double]{r}{w3}{n}";
    let mut ed = editor(src, Kind::Er);
    let owner = ed.doc().elements.iter().find(|e| e.as_edge().is_some()).unwrap().key;
    if let Some(Body::Edge(e)) = ed.session_mut().doc_mut().get_mut(owner).map(|el| &mut el.body) {
        e.dst = "w2".into();
    }
    ed.sync_double(owner);
    assert_eq!(edges(&ed).iter().map(|e| e.2).collect::<Vec<_>>(), [false, true]);
}

#[test]
fn reconnecting_by_dragging_an_end() {
    let mut ed = build_er();
    ed.set_tool("entity");
    click(&mut ed, 4.0, 4.0);
    ed.set_tool("select");
    let edge = ed.doc().elements.iter().find(|e| e.as_edge().is_some()).unwrap().key;
    ed.set_selection(vec![edge]);
    let start = handle(&mut ed, "start");
    drag(&mut ed, start, (4.0, 4.0), none());
    assert_eq!(edges(&ed)[0].0, "entity3");
    assert_eq!(ed.session().undo_label(), Some("Edit edge"));
}

#[test]
fn weak_links_are_doubled_automatically() {
    let mut ed = editor("", Kind::Er);
    ed.set_tool("entity");
    click(&mut ed, 0.0, 0.0);
    ed.set_tool("identification dependency");
    click(&mut ed, 4.0, 0.0);
    ed.set_tool("weak entity");
    click(&mut ed, 8.0, 0.0);
    ed.set_tool("link");
    drag(&mut ed, (0.0, 0.0), (4.0, 0.0), none());
    drag(&mut ed, (4.0, 0.0), (8.0, 0.0), none());
    assert_eq!(edges(&ed).iter().map(|e| e.2).collect::<Vec<_>>(), [false, true]);
}

#[test]
fn subdomain_corners() {
    let mut ed = editor("\\subdomain{S}{(0,0) (4,-3)}\n", Kind::Er);
    let sd = ed.doc().elements[0].key;
    ed.set_selection(vec![sd]);
    let (mx, my) = handle(&mut ed, "m0"); // the middle of the top side
    drag(&mut ed, (mx, my), (mx, my + 1.0), none());
    let pts = match &ed.doc().elements[0].body {
        Body::Subdomain(s) => s.points.clone(),
        _ => panic!(),
    };
    assert_eq!(
        pts,
        [(0.0, 0.0), (2.0, 1.0), (4.0, 0.0), (4.0, -3.0), (0.0, -3.0)].map(|(x, y)| Coord::numeric(x, y))
    );
    // a double click on a corner removes it
    assert!(ed.double_click(2.0 * CM, 1.0 * CM));
    match &ed.doc().elements[0].body {
        Body::Subdomain(s) => assert_eq!(s.points.len(), 4),
        _ => panic!(),
    }
    ed.make_rectangle(sd_key(&ed));
    assert!(text(&ed).contains("\\subdomain{S}{(0,0) (4,-3)}"), "{}", text(&ed));
}

fn sd_key(ed: &Editor) -> ElementKey {
    ed.doc().elements[0].key
}

#[test]
fn boxes_move_their_contents() {
    let src = "\\subdomain{Box}{(0,2) (6,-2)}\n\\entity{a}(1,0){A}\n\\entity{b}(4,0){B}\n\\point{p}(5,1)\n\
               \\link[via={(2,1) (3,1)}]{a}{b}\n\\entity{out}(10,0){Out}\n";
    let mut ed = editor(src, Kind::Er);
    // by dragging the label box
    drag(&mut ed, (0.3, 1.8), (1.3, 0.8), none());
    let t = text(&ed);
    assert!(t.contains("\\subdomain{Box}{(1,1) (7,-3)}"), "{t}");
    assert!(
        t.contains("\\entity{a}(2,-1){A}")
            && t.contains("\\entity{b}(5,-1){B}")
            && t.contains("\\point{p}(6,0)"),
        "{t}"
    );
    assert!(t.contains("via={(3,0) (4,0)}"), "way points move with both ends: {t}");
    assert!(t.contains("\\entity{out}(10,0){Out}"));
    // undo brings all of it back
    ed.undo();
    assert_eq!(text(&ed), src);
    // nudging (Shift: 1 cm)
    let sd = ed.doc().elements[0].key;
    ed.set_selection(vec![sd]);
    assert!(ed.key("nudge right", Mods { shift: true, ..none() }));
    assert!(text(&ed).contains("\\entity{a}(2,0){A}"), "{}", text(&ed));
}

#[test]
fn rubber_band_and_shift_click() {
    let mut ed = build_er();
    drag(&mut ed, (-3.0, 3.0), (5.5, -3.0), none());
    assert_eq!(ed.selection.len(), 3, "two nodes and the edge between them");
    ed.press(8.0 * CM, 0.0, Mods { shift: true, ..none() });
    ed.release(8.0 * CM, 0.0, Mods { shift: true, ..none() });
    assert_eq!(ed.selection.len(), 4);
    assert!(ed.key("cancel", none()));
    assert!(ed.selection.is_empty());
}

#[test]
fn escape_cancels_a_drag_and_leaves_a_tool() {
    let mut ed = build_er();
    ed.press(8.0 * CM, 0.0, none());
    ed.move_to(10.0 * CM, 2.0 * CM, none());
    assert!(text(&ed).contains("\\entity{entity2}(10,2)"));
    ed.key("cancel", none());
    assert!(text(&ed).contains("\\entity{entity2}(8,0)"));
    assert!(ed.session().undo_label() != Some("Move"));
    ed.set_tool("entity");
    ed.key("cancel", none());
    assert_eq!(ed.tool, "select");
}

#[test]
fn attribute_lists_and_ovals() {
    let mut ed = editor("\\entity{p}(0,0){Person}\n", Kind::Er);
    ed.set_tool("attribute tree");
    click(&mut ed, 0.0, 0.0);
    click(&mut ed, 0.0, 0.0);
    assert!(text(&ed).contains("\\attributes{p}{attribute, attribute}"), "{}", text(&ed));
    // dragging the list around its node changes its side
    ed.set_tool("select");
    let tree = ed.doc().elements[1].key;
    ed.set_selection(vec![tree]);
    let b = diagram_core::svg::bbox(ed.session_mut().laid().gfx_for(tree).unwrap()).unwrap();
    let at = ((b.0 + b.2) / 2.0 / CM, (b.1 + b.3) / 2.0 / CM);
    drag(&mut ed, at, (0.0, -3.0), none());
    assert!(text(&ed).contains("\\attributes[anchor=south]{p}"), "{}", text(&ed));
    // ovals: dragging one fixes its place, and the others stay where they are
    let mut ed =
        editor("\\setup{attribute style=chen}\n\\entity{p}(0,0){Person}\n\\attributes{p}{a, b}\n", Kind::Er);
    let tree = ed.doc().elements[2].key;
    let first = match ed.session_mut().laid().gfx_for(tree).unwrap().handles.get("ovals") {
        Some(Handle::Ovals(o)) => o[0].shape.center(),
        _ => panic!(),
    };
    drag(&mut ed, (first.0 / CM, first.1 / CM), (first.0 / CM - 1.0, first.1 / CM + 1.0), none());
    let t = text(&ed);
    assert!(t.contains("a@") && t.contains("b@"), "{t}");
}

#[test]
fn chen_attribute_and_isa_need_their_node() {
    let mut ed = editor("\\entity{p}(0,0){Person}\n", Kind::Er);
    ed.set_tool("chen attribute");
    click(&mut ed, 0.0, 0.0);
    assert!(text(&ed).contains("\\attribute[of=p]{attr}(2,1.25){attribute}"), "{}", text(&ed));
    ed.set_tool("is a");
    click(&mut ed, 10.0, 10.0);
    assert!(messages(&mut ed).contains("Click on the supertype entity"));
    click(&mut ed, 0.0, 0.0);
    assert!(text(&ed).contains("\\isa{isa}(0,-1.75){p}"), "{}", text(&ed));
}

#[test]
fn context_menu_actions() {
    let mut ed = build_er();
    let (mx, my) = {
        let edge = ed.doc().elements.iter().find(|e| e.as_edge().is_some()).unwrap().key;
        ed.set_selection(vec![edge]);
        handle(&mut ed, "bend")
    };
    drag(&mut ed, (mx, my), (mx, my + 0.6), none());
    let (bx, by) = handle(&mut ed, "bend"); // the middle of the bent edge
    ed.set_selection(vec![]);
    assert_eq!(ed.context(bx * CM, by * CM), "edge");
    ed.straighten();
    assert!(ed.doc().edges().next().unwrap().opts.get("bend").is_none());
    assert_eq!(ed.context(0.0, 0.0), "node");
    ed.add_attribute_list();
    assert!(text(&ed).contains("\\attributes{entity}{attribute}"));
    assert_eq!(ed.context(30.0 * CM, 30.0 * CM), "one", "the selection stays");
}

#[test]
fn the_overlay_shows_selection_handles_and_previews() {
    let mut ed = build_er();
    let edge = ed.doc().elements.iter().find(|e| e.as_edge().is_some()).unwrap().key;
    ed.set_selection(vec![edge]);
    let o = ed.overlay();
    assert!(o.contains("stroke-opacity") && o.contains("<circle") && o.contains("<rect"));
    ed.set_tool("link");
    ed.press(0.0, 0.0, none());
    ed.move_to(8.0 * CM, 0.0, none());
    assert!(ed.overlay().contains("#dc1e1e"), "red: not allowed");
    ed.move_to(4.0 * CM, 0.0, none());
    assert!(ed.overlay().contains("#00aa3c"), "green: allowed");
}
