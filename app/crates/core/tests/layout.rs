//! The layout engine: the Python app's layout tests, ported.  Reference values come from
//! LaTeX (`\pgfpointanchor` of the package's nodes, and the oval centres the package prints for
//! latex_packages/tests/chen_lists.tex).

use std::path::Path;

use diagram_core::document::{Document, Kind, kind_for, normalize_newlines};
use diagram_core::fonts::Metrics;
use diagram_core::geometry::{CM, ShapeKind};
use diagram_core::layout::{DummyEngine, FontEngine, Handle, Layout, TextEngine, choose_side};
use diagram_core::style::LabelLanguage;

fn layout<'a>(doc: &'a Document, engine: &'a dyn TextEngine) -> Layout<'a> {
    Layout::new(doc, engine, LabelLanguage::English).run().unwrap()
}

fn er(src: &str) -> Document {
    Document::parse(src, Kind::Er).unwrap()
}

const FONTS: FontEngine = FontEngine { metrics: Metrics::DesignSizes };

fn anchor(src: &str, kind: Kind, name: &str) -> (f64, f64) {
    let doc = Document::parse(src, kind).unwrap();
    let lay = layout(&doc, &DummyEngine);
    let s = lay.gfx[0].shape.clone().unwrap();
    let (x, y) = s.anchor(name);
    (x - s.cx, y - s.cy)
}

fn close(got: (f64, f64), want: (f64, f64), tol: f64) {
    assert!((got.0 - want.0).abs() <= tol && (got.1 - want.1).abs() <= tol, "{got:?} != {want:?}");
}

#[test]
fn shapes_match_tikz() {
    close(anchor("\\entity{a}(0,0){A}", Kind::Er, "north"), (0.0, 14.52637), 0.02);
    close(anchor("\\entity{a}(0,0){A}", Kind::Er, "east"), (40.1339, 0.0), 0.02);
    close(anchor("\\entity{a}(0,0){A}", Kind::Er, "30"), (25.16046, 14.52637), 0.02);
    close(anchor("\\relationship{r}(0,0){A}", Kind::Er, "north"), (0.0, 20.21684), 0.02);
    close(anchor("\\relationship{r}(0,0){A}", Kind::Er, "east"), (28.75275, 0.0), 0.02);
    close(anchor("\\subtype{s}(0,0){A}", Kind::Gsm, "north"), (0.0, 20.21684), 0.02);
    close(anchor("\\representable{p}(0,0){A}", Kind::Gsm, "east"), (34.44322, 0.0), 0.02);
    close(anchor("\\aggregation{c}(0,0){}", Kind::Gsm, "north"), (0.0, 11.68114), 0.02);
}

#[test]
fn the_triangle_with_real_fonts() {
    // TikZ: apex 23.537, corners (+-37.713, -14.178) for a one-letter label
    let doc = Document::parse("\\abstract{t}(0,0){A}", Kind::Gsm).unwrap();
    let lay = layout(&doc, &FONTS);
    let Some(ShapeKind::Triangle { apex, left, right }) = lay.gfx[0].shape.as_ref().map(|s| s.kind.clone())
    else {
        panic!("not a triangle")
    };
    assert!((right.0 - (37.713 - 0.72)).abs() < 0.5, "{right:?}");
    assert!((apex.1 - left.1 - 37.0).abs() < 0.6, "{apex:?} {left:?}");
}

#[test]
fn positions_and_the_positioning_library() {
    let d = er("\\entity{a}(2,-1){A}");
    let s = layout(&d, &DummyEngine).gfx[0].shape.clone().unwrap();
    close((s.cx, s.cy), (2.0 * CM, -CM), 1e-9);
    let d = er("\\entity{a}(0,0){A}\n\\entity[right=1cm of a]{b}{B}");
    let lay = layout(&d, &DummyEngine);
    let (a, b) = (&lay.shapes["a"], &lay.shapes["b"]);
    assert!((b.anchor("west").0 - a.anchor("east").0 - CM).abs() < 0.01);
    assert!((b.cy - a.cy).abs() < 1e-9);
}

#[test]
fn edges_end_on_borders_and_bend_as_tikz() {
    let d = er("\\entity{a}(0,0){A}\n\\relationship{r}(5,0){R}\n\\link{a}{r}{1}");
    let lay = layout(&d, &DummyEngine);
    let seg = lay.gfx[2].segments[0];
    assert!((seg.p0.0 - lay.shapes["a"].anchor("east").0).abs() < 0.01);
    assert!((seg.p3.0 - lay.shapes["r"].anchor("west").0).abs() < 0.01);
    let d = er("\\entity{a}(0,0){A}\n\\entity{b}(6,0){B}\n\\link[bend=30]{a}{b}");
    let lay = layout(&d, &DummyEngine);
    let (tx, ty) = lay.gfx[2].segments[0].tangent(0.0);
    assert!((ty.atan2(tx).to_degrees() - 30.0).abs() < 0.5);
}

#[test]
fn sides_of_labels() {
    assert!(choose_side("auto", 10.0, 1.0)); // flat, rightwards: above = left
    assert!(!choose_side("auto", -10.0, 1.0));
    assert!(choose_side("auto", 0.0, -10.0)); // down: the right side is left of the direction
    assert!(!choose_side("below", 10.0, 0.0));
}

#[test]
fn every_repository_file_lays_out_without_errors() {
    fn files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() && ![".git", "build", "dist", "target", "node_modules"].contains(&name.as_str()) {
                files(&p, out);
            } else if name.ends_with(".erd") || name.ends_with(".gsmd") {
                out.push(p);
            }
        }
    }
    let mut all = Vec::new();
    files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."), &mut all);
    assert!(all.len() > 10);
    for f in all {
        let text = normalize_newlines(&std::fs::read_to_string(&f).unwrap());
        let ext = f.extension().map(|e| format!(".{}", e.to_string_lossy()));
        let doc = Document::parse(&text, kind_for(ext.as_deref(), &text)).unwrap();
        let lay = layout(&doc, &FONTS);
        let errors: Vec<&String> =
            lay.gfx.iter().flat_map(|g| &g.warnings).filter(|w| w.starts_with("cannot")).collect();
        assert!(errors.is_empty(), "{}: {errors:?}", f.display());
    }
}

const CROWS: &str = "\\setup{card symbols=crowsfoot}\\entity{a}(0,0){A}\\relationship{r}(4,2.5){R}\
                     \\entity{b}(9,1){B}\\link{a}{r}{1}\\link{r}{b}{cn}\\relationship{s}(1,-3){S}\\link[bend=30]{s}{a}{n}";

#[test]
fn crows_foot_meets_the_side_at_a_right_angle() {
    let d = er(CROWS);
    let lay = layout(&d, &DummyEngine);
    let edges: Vec<_> = d.edges().collect();
    let drawn: Vec<_> =
        lay.gfx.iter().filter(|g| g.key.and_then(|k| d.get(k)).and_then(|e| e.as_edge()).is_some()).collect();
    assert_eq!(edges.len(), drawn.len());
    for (e, g) in edges.iter().zip(drawn) {
        assert_eq!(g.stub.len(), 1);
        let s = g.stub[0];
        let (dx, dy) = (s.p3.0 - s.p0.0, s.p3.1 - s.p0.1);
        assert!(dx.abs() < 1e-6 || dy.abs() < 1e-6);
        let entity = if lay.types.get(e.dst_id()).map(String::as_str) == Some("entity") {
            e.dst_id()
        } else {
            e.src_id()
        };
        let shape = &lay.shapes[entity];
        let Some(Handle::Point(hit)) = g.handles.get(if entity == e.dst_id() { "end" } else { "start" })
        else {
            panic!()
        };
        let hit = *hit;
        let outer = if s.p0 == hit { s.p3 } else { s.p0 };
        let (hw, hh) = (shape.anchor("east").0 - shape.cx, shape.anchor("north").1 - shape.cy);
        if dy.abs() < 1e-6 {
            assert!(((hit.0 - shape.cx).abs() - hw).abs() < 1e-4);
            assert!((outer.0 - hit.0) * (hit.0 - shape.cx) > 0.0);
        } else {
            assert!(((hit.1 - shape.cy).abs() - hh).abs() < 1e-4);
            assert!((outer.1 - hit.1) * (hit.1 - shape.cy) > 0.0);
        }
        // the rest of the edge continues from the stub
        let ends = [g.segments[0].p0, g.segments[g.segments.len() - 1].p3];
        assert!(
            ends.iter()
                .any(|p| [s.p0, s.p3].iter().any(|q| (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6))
        );
    }
    let plain = er(&CROWS.replace("\\setup{card symbols=crowsfoot}", ""));
    assert!(layout(&plain, &DummyEngine).gfx.iter().all(|g| g.stub.is_empty()));
    let other =
        er("\\setup{card symbols=crowsfoot}\\entity{a}(0,0){A}\\relationship{r}(4,2){R}\\link{a}{r}{(3,*)}");
    assert!(layout(&other, &DummyEngine).gfx.iter().all(|g| g.stub.is_empty()));
}

const PARTIAL: &str = "\\entity{o}(0,0){Owner}\\attributes{o}{*id}\n\\entity[weak]{w}(5,0){Weak}\\attributes{w}{*nr, a{*b, c}, d}\n\
\\relationship%s{r}(2.5,0){has}\n\\link{o}{r}{1}\\link{r}{w}{n}\n\\attribute[key, of=w]{k}(5,2){chen}\n\
\\attribute[composite, of=w]{m}(7,2){comp}\\attribute[key, of=m]{n}(9,2){part}\n\\attribute[key]{l}(5,-2){linked}\\link{w}{l}\n\
\\attribute[key, of=o]{s}(0,2){strong}\n";

#[test]
fn partial_keys_are_dashed_unless_they_depend_on_e_alone() {
    for (rel, partial) in [("[dependency=Id]", true), ("[weak]", true), ("[dependency=E]", false)] {
        let d = er(&PARTIAL.replace("%s", rel));
        let lay = layout(&d, &DummyEngine);
        assert_eq!(lay.key_label("x", "w").contains("dashuline"), partial, "{rel}");
        for ident in ["k", "n", "l"] {
            assert_eq!(
                lay.key_label("x", &lay.owner_of_attribute(ident)).contains("dashuline"),
                partial,
                "{rel} {ident}"
            );
        }
        assert!(!lay.key_label("x", "o").contains("dashuline"));
        assert!(!lay.key_label("x", &lay.owner_of_attribute("s")).contains("dashuline"));
    }
    let alone = er("\\entity[weak]{w}(0,0){W}\\attributes{w}{*nr}");
    assert!(layout(&alone, &DummyEngine).key_label("x", "w").contains("dashuline"));
    let mixed = er("\\entity{o}(0,0){O}\\entity[weak]{w}(5,0){W}\\relationship[dependency=E]{e}(2,1){A}\
                    \\relationship[dependency=Id]{i}(2,-1){B}\\link{o}{e}\\link{e}{w}\\link{o}{i}\\link{i}{w}");
    assert!(layout(&mixed, &DummyEngine).key_label("x", "w").contains("dashuline"));
    let rel = er("\\relationship[weak]{r}(0,0){R}\\attributes{r}{*a}");
    assert!(!layout(&rel, &DummyEngine).key_label("x", "r").contains("dashuline"));
}

const CHEN: &str = "\\setup{attribute style=chen}\n\\entity{p}(0,0){Person}\n\
\\attributes{p}{*PID, name, address{street, town}, \\{email\\}, /age}\n\\relationship{r}(5,0){owns}\\link{p}{r}{1}\n\
\\attribute[of=p]{old}(-4,0){old}\n";

fn ovals(src: &str) -> Vec<(String, diagram_core::geometry::Shape, (f64, f64))> {
    let d = er(src);
    let lay = layout(&d, &FONTS);
    lay.gfx
        .iter()
        .filter_map(|g| match g.handles.get("ovals") {
            Some(Handle::Ovals(o)) => Some(o.clone()),
            _ => None,
        })
        .flatten()
        .map(|o| (o.item.text.clone(), o.shape, o.origin))
        .collect()
}

#[test]
fn ovals_get_their_shapes_and_keep_away_from_edges() {
    let got = ovals(CHEN);
    let names: Vec<&str> = got.iter().map(|(n, _, _)| n.as_str()).collect();
    assert_eq!(names, ["PID", "name", "address", "street", "town", "email", "age"]);
    let shape = |n: &str| got.iter().find(|(m, _, _)| m == n).map(|(_, s, _)| s.clone()).unwrap();
    assert!(matches!(shape("PID").kind, ShapeKind::Ellipse { .. }));
    assert!(matches!(shape("address").kind, ShapeKind::Rect { .. }));
    let pid = shape("PID");
    assert!(pid.cx.abs() < 0.01 && pid.cy > 0.0); // the first attribute is on top
    let d = er(CHEN);
    let lay = layout(&d, &FONTS);
    let used: Vec<f64> = lay.angles["p"].iter().map(|(_, a)| *a).collect();
    for (name, s, (ox, oy)) in &got {
        if name == "street" || name == "town" {
            continue;
        }
        let ang = (s.cy - oy).atan2(s.cx - ox).to_degrees().rem_euclid(360.0);
        for a in &used {
            let d = (ang - a).rem_euclid(360.0).min((a - ang).rem_euclid(360.0));
            assert!(d > 14.0, "{name}");
        }
    }
    assert!(lay.gfx.iter().any(|g| g.handles.contains_key("ovals")));
}

#[test]
fn many_ovals_never_overlap() {
    let names: Vec<String> = (0..14).map(|k| format!("attribute{k}")).collect();
    let got = ovals(&format!(
        "\\setup{{attribute style=chen}}\\entity{{m}}(0,0){{Many}}\\attributes{{m}}{{{}}}",
        names.join(", ")
    ));
    for (i, (_, a, _)) in got.iter().enumerate() {
        for (_, b, _) in got.iter().skip(i + 1) {
            let (ShapeKind::Ellipse { a: aa, b: ab }, ShapeKind::Ellipse { a: ba, b: bb }) =
                (&a.kind, &b.kind)
            else {
                continue;
            };
            assert!(!((a.cx - b.cx).abs() < aa + ba && (a.cy - b.cy).abs() < ab + bb));
        }
    }
}

#[test]
fn ovals_where_the_package_puts_them() {
    // oval centres in pt, printed by the package for latex_packages/tests/chen_lists.tex
    let reference = [
        ("PID", (0.00, 50.09)),
        ("name", (41.36, 41.36)),
        ("age", (-82.81, 0.00)),
        ("email", (0.00, -50.09)),
        ("address", (42.59, -42.59)),
        ("street", (46.57, -88.01)),
        ("town", (85.30, -85.30)),
        ("geo", (102.25, -47.81)),
        ("lat", (155.86, -72.81)),
        ("lon", (159.57, -32.45)),
        ("since", (142.26, 55.78)),
        ("breed", (368.74, 0.00)),
        ("one", (256.07, -120.62)),
        ("ten", (214.84, -129.48)),
        ("seven", (256.07, -220.81)),
        ("free", (-13.16, -219.82)),
        ("x", (34.14, -97.86)),
    ];
    let src = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../latex_packages/tests/chen_lists.tex"),
    )
    .unwrap();
    let body = src.split("\\begin{erdiagram}").nth(1).unwrap().split("\\end{erdiagram}").next().unwrap();
    let body = body.split_once(']').unwrap().1;
    let got = ovals(&format!("\\setup{{attribute style=chen}}{body}"));
    let mut worst = 0.0f64;
    for (name, want) in reference {
        let (_, s, _) = got.iter().find(|(n, _, _)| n == name).unwrap_or_else(|| panic!("{name}"));
        worst = worst.max((s.cx - want.0).abs()).max((s.cy - want.1).abs());
        close((s.cx, s.cy), want, 0.6);
    }
    eprintln!("ovals: at most {worst:.3} pt from where LaTeX puts them");
}

#[test]
fn a_fixed_position_is_relative_to_its_owner() {
    let got = ovals("\\setup{attribute style=chen}\\entity{p}(1,1){P}\\attributes{p}{a@2/0{b@0/1}}");
    let c = |n: &str| got.iter().find(|(m, _, _)| m == n).map(|(_, s, _)| (s.cx, s.cy)).unwrap();
    assert!((c("a").0 - 3.0 * CM).abs() < 0.01);
    assert!((c("b").1 - 2.0 * CM).abs() < 0.01);
}
