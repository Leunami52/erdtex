//! The core's interface for the web page.  Plain values only (strings, numbers, booleans and
//! arrays of them), so that the page needs no glue beyond what wasm-bindgen makes; the panels'
//! structured data is JSON (see `json`).

#![forbid(unsafe_code)]

mod json;

use diagram_core::document::{Document, Kind, kind_for, normalize_newlines};
use diagram_core::editor::{self, Editor, Event, Mods};
use diagram_core::fonts::{self, Family, Metrics, Style};
use diagram_core::gsm2er::{AttrNames, ConversionSettings};
use diagram_core::model::Body;
use diagram_core::newdefaults;
use diagram_core::session::Session;
use diagram_core::shortcuts;
use diagram_core::style::LabelLanguage;
use diagram_core::svg::{Colors, Options, TextMode};
use diagram_core::{panel, problems, rmwindow};

/// The relational window's options, as the page keeps them.
#[allow(clippy::too_many_arguments)]
fn rm_options(
    format: &str,
    rel: &str,
    fk: &str,
    marks: &str,
    text_types: bool,
    quote_all: bool,
    drop: bool,
    source: &str,
) -> rmwindow::Options {
    rmwindow::Options {
        format: format.into(),
        rel_naming: rel.into(),
        fk_naming: fk.into(),
        marks: marks.into(),
        text_types,
        quote_all,
        drop,
        source: source.into(),
    }
}
use wasm_bindgen::prelude::*;

fn style(family: &str, bold: bool, italic: bool) -> Style {
    let family = match family {
        "sans" => Family::Sans,
        "mono" => Family::Mono,
        _ => Family::Roman,
    };
    Style::new(family, bold, italic)
}

fn kind(name: &str) -> Kind {
    Kind::from_name(name).unwrap_or(Kind::Er)
}

/// Width in pt, in the design size LaTeX would use.
#[wasm_bindgen]
pub fn text_width(family: &str, bold: bool, italic: bool, size: f64, text: &str) -> f64 {
    fonts::text_width(style(family, bold, italic), size, text)
}

/// The font file (and @font-face family) LaTeX would use.
#[wasm_bindgen]
pub fn design_font(family: &str, bold: bool, italic: bool, size: f64) -> String {
    fonts::design_font(style(family, bold, italic), size).to_string()
}

/// The names of the bundled fonts, for registering them with the page (FontFace).
#[wasm_bindgen]
pub fn font_names() -> Vec<String> {
    fonts::files().map(|(name, _)| name.to_string()).collect()
}

/// A bundled font file, so that the page needn't ship and fetch a second copy.
#[wasm_bindgen]
pub fn font_data(name: &str) -> Option<Vec<u8>> {
    fonts::files().find(|(n, _)| *n == name).map(|(_, data)| data.to_vec())
}

/// The diagram as the editor draws it, for exports: a self-contained SVG, text as outlines, the size
/// in PostScript points (the website makes PNG and PDF from it; the desktop app does so in Rust).
#[wasm_bindgen]
pub fn export_svg(text: &str, kind_name: &str, language: &str, grid: bool) -> Result<String, JsError> {
    let doc = Document::parse(text, kind(kind_name)).map_err(|e| JsError::new(&e.to_string()))?;
    let lang = if language == "german" { LabelLanguage::German } else { LabelLanguage::English };
    Ok(diagram_core::export::editor_svg(&doc, lang, grid))
}

/// The kind of diagram a file holds: by its extension (`.erd`), else by its commands.
#[wasm_bindgen]
pub fn kind_of_file(extension: Option<String>, text: &str) -> String {
    kind_for(extension.as_deref(), &normalize_newlines(text)).name().to_string()
}

/// The commands with shortcuts: `id`, `title`, `group` and the default keys, one command per
/// four entries.
#[wasm_bindgen]
pub fn shortcut_table() -> Vec<String> {
    shortcuts::COMMANDS
        .iter()
        .flat_map(|c| {
            [c.id.to_string(), c.title.to_string(), c.group.title().to_string(), c.default.to_string()]
        })
        .collect()
}

/// Which other command already uses one of these keys: its title, or empty.  `ids`/`keys_now`:
/// the bindings as they are in the dialog.
#[wasm_bindgen]
pub fn shortcut_conflict(id: &str, keys: &str, ids: Vec<String>, keys_now: Vec<String>) -> String {
    let bindings: std::collections::HashMap<String, String> = ids.into_iter().zip(keys_now).collect();
    shortcuts::find_conflict(&bindings, id, keys).map(|c| c.title.to_string()).unwrap_or_default()
}

/// A key combination as Qt writes it (`Ctrl+Shift+Z`); empty if it isn't one.
#[wasm_bindgen]
pub fn canonical_keys(text: &str) -> String {
    shortcuts::canonical(text)
}

/// What a colour expression looks like, as `#rrggbb`; empty if it isn't one.
#[wasm_bindgen]
pub fn color_rgb(expr: &str, tinted: bool) -> String {
    panel::color_rgb(expr, tinted)
        .map(|(r, g, b)| {
            let ch = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            format!("#{:02x}{:02x}{:02x}", ch(r), ch(g), ch(b))
        })
        .unwrap_or_default()
}

/// A page of Preferences: the defaults of new diagrams of a kind (JSON).  `names`/`values`:
/// the settings.  Each key has its setting's name, the packages' default and the value kept
/// ("" when it is the default).
#[wasm_bindgen]
pub fn defaults_page(kind_name: &str, names: Vec<String>, values: Vec<String>) -> String {
    let k = kind(kind_name);
    let kept: std::collections::HashMap<String, String> = names.into_iter().zip(values).collect();
    let get = |name: &str| kept.get(name).cloned();
    let toggles = newdefaults::toggles(k).into_iter().map(|t| {
        let choices =
            t.choices.iter().map(|(v, x)| format!("[{},{}]", json::s(v), json::s(x))).collect::<Vec<_>>();
        format!(
            r#"{{"key":{},"title":{},"setting":{},"choices":[{}],"value":{}}}"#,
            json::s(t.key),
            json::s(t.title),
            json::s(&newdefaults::setting_name(k, t.key)),
            choices.join(","),
            json::s(&newdefaults::stored_value(&get, k, t.key))
        )
    });
    let groups = newdefaults::style_groups(k).into_iter().map(|(title, keys)| {
        let keys = keys.into_iter().map(|key| {
            let default = newdefaults::default_value(k, key).unwrap_or("");
            let value = newdefaults::stored_value(&get, k, key);
            format!(
                r#"{{"key":{},"setting":{},"default":{},"type":{},"value":{}}}"#,
                json::s(key),
                json::s(&newdefaults::setting_name(k, key)),
                json::s(default),
                json::s(newdefaults::key_type(key)),
                json::s(if value == default { "" } else { &value })
            )
        });
        format!(r#"{{"title":{},"keys":[{}]}}"#, json::s(title), keys.collect::<Vec<_>>().join(","))
    });
    format!(
        r#"{{"toggles":[{}],"groups":[{}]}}"#,
        toggles.collect::<Vec<_>>().join(","),
        groups.collect::<Vec<_>>().join(",")
    )
}

/// What is wrong with a value for a style key; empty if nothing.
#[wasm_bindgen]
pub fn check_default(key: &str, value: &str) -> String {
    newdefaults::check_value(key, value).unwrap_or_default()
}

/// The packages' default of a style key for a kind of diagram.
#[wasm_bindgen]
pub fn default_of(kind_name: &str, key: &str) -> String {
    newdefaults::default_value(kind(kind_name), key).unwrap_or("").to_string()
}

/// The line that includes a diagram in a LaTeX document (File → Copy \input Line).
#[wasm_bindgen]
pub fn input_line(kind_name: &str, file_name: Option<String>) -> String {
    diagram_core::export::input_line(kind(kind_name), file_name.as_deref())
}

/// The usual data types, offered in the type fields.
#[wasm_bindgen]
pub fn type_presets() -> Vec<String> {
    panel::type_presets().iter().map(|t| t.to_string()).collect()
}

/// The palette of a kind of diagram: `section`, `tool` pairs side by side.
#[wasm_bindgen]
pub fn palette(kind_name: &str) -> Vec<String> {
    editor::palette(kind(kind_name))
        .into_iter()
        .flat_map(|(section, tools)| {
            tools.into_iter().flat_map(move |t| [section.to_string(), t.to_string()])
        })
        .collect()
}

/// What a tool does, for its tooltip.
#[wasm_bindgen]
pub fn tool_tip(tool: &str) -> String {
    editor::tip(tool).to_string()
}

fn mods(shift: bool, ctrl: bool, alt: bool) -> Mods {
    Mods { shift, ctrl, alt }
}

/// One open diagram: the document, its undo history, the editing state and the drawing.
#[wasm_bindgen]
pub struct Diagram {
    ed: Editor,
}

#[wasm_bindgen]
impl Diagram {
    /// A diagram from the text of a file; the error says where the text can't be read.
    #[wasm_bindgen(constructor)]
    pub fn new(text: &str, kind_name: &str) -> Result<Diagram, JsError> {
        Session::open(&normalize_newlines(text), kind(kind_name))
            .map(|session| Diagram { ed: Editor::new(session) })
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// A new diagram with the defaults the app keeps (settings: names and values side by side,
    /// as `new diagram/er/isa shape` = `diamond`).
    pub fn fresh(kind_name: &str, names: Vec<String>, values: Vec<String>) -> Diagram {
        let mut doc = Document::new(kind(kind_name));
        let kept: std::collections::HashMap<String, String> = names.into_iter().zip(values).collect();
        newdefaults::apply_defaults(&mut doc, &|name: &str| kept.get(name).cloned());
        Diagram { ed: Editor::new(Session::new(doc)) }
    }

    /// `er` or `gsm`.
    pub fn kind(&self) -> String {
        self.ed.doc().kind.name().to_string()
    }

    /// The file text.
    pub fn text(&self) -> String {
        self.ed.session().text()
    }

    /// Counts the changes of the document: the page draws again when it changed.
    pub fn revision(&self) -> f64 {
        self.ed.session().revision() as f64
    }

    /// Whether only `\setup` is in it (a new diagram nobody has touched).
    pub fn is_blank(&self) -> bool {
        self.ed.doc().elements.iter().all(|e| matches!(e.body, Body::Setup(_)))
    }

    /// The drawing for the canvas: `[defs, body]`.  `px_per_pt`, in device pixels, decides the
    /// thinnest line (one device pixel).
    pub fn drawing(&mut self, dark: bool, px_per_pt: f64) -> Vec<String> {
        let (doc, laid) = self.ed.session_mut().parts();
        let points =
            doc.elements.iter().filter(|e| matches!(e.body, Body::Point(_))).map(|e| e.key).collect();
        let opts = Options {
            text: TextMode::Text,
            metrics: Metrics::DesignSizes,
            colors: if dark { Colors::DarkCanvas } else { Colors::Original },
            min_stroke: if px_per_pt > 0.0 { 1.0 / px_per_pt } else { 0.0 },
            points,
        };
        let d = diagram_core::svg::draw(&laid.gfx, &opts);
        vec![d.defs, d.body]
    }

    /// The extent of the drawing: `[x0, y0, x1, y1]`, or nothing for an empty diagram.
    pub fn bounds(&mut self) -> Vec<f64> {
        self.ed.session_mut().bounds().map_or(vec![], |(a, b, c, d)| vec![a, b, c, d])
    }

    /// Whether the diagram prints data types.
    pub fn types_shown(&self) -> bool {
        self.ed.doc().types_shown()
    }

    /// Shows or hides data types, as an undoable edit.
    pub fn set_types_shown(&mut self, on: bool) -> bool {
        self.ed.apply(if on { "Show data types" } else { "Hide data types" }, |ed| {
            ed.session_mut().doc_mut().set_types_shown(on)
        })
    }

    pub fn undo(&mut self) -> bool {
        self.ed.undo()
    }

    pub fn redo(&mut self) -> bool {
        self.ed.redo()
    }

    /// What Undo would undo (empty: nothing).
    pub fn undo_label(&self) -> String {
        self.ed.session().undo_label().unwrap_or("").to_string()
    }

    /// What Redo would redo (empty: nothing).
    pub fn redo_label(&self) -> String {
        self.ed.session().redo_label().unwrap_or("").to_string()
    }

    /// Whether it is as saved.
    pub fn is_clean(&self) -> bool {
        self.ed.session().is_clean()
    }

    /// It was saved.
    pub fn mark_saved(&mut self) {
        self.ed.session_mut().mark_saved();
    }

    /// The language of keywords in the drawing: `english` or `german`.
    pub fn set_language(&mut self, name: &str) {
        self.ed.session_mut().set_language(if name == "german" {
            LabelLanguage::German
        } else {
            LabelLanguage::English
        });
    }

    /// The layout's warnings, such as "cannot draw: ...".
    pub fn layout_warnings(&mut self) -> Vec<String> {
        self.ed.session_mut().layout_warnings().into_iter().map(|(_, w)| w).collect()
    }

    // -- editing ----------------------------------------------------------------------------

    /// TeX points per screen pixel (for handle sizes and how near a click must be).
    pub fn set_scale(&mut self, pt_per_px: f64) {
        self.ed.set_scale(pt_per_px);
    }

    pub fn set_tool(&mut self, tool: &str) {
        self.ed.set_tool(tool);
    }

    pub fn tool(&self) -> String {
        self.ed.tool.clone()
    }

    /// Grid snapping in cm (0: none).
    pub fn set_snap(&mut self, cm: f64) {
        self.ed.snap_cm = cm;
    }

    /// The left button pressed at (x, y), in TeX points with y up.
    pub fn press(&mut self, x: f64, y: f64, shift: bool, ctrl: bool, alt: bool) {
        self.ed.press(x, y, mods(shift, ctrl, alt));
    }

    pub fn move_to(&mut self, x: f64, y: f64, shift: bool, ctrl: bool, alt: bool) {
        self.ed.move_to(x, y, mods(shift, ctrl, alt));
    }

    pub fn release(&mut self, x: f64, y: f64, shift: bool, ctrl: bool, alt: bool) {
        self.ed.release(x, y, mods(shift, ctrl, alt));
    }

    pub fn double_click(&mut self, x: f64, y: f64) -> bool {
        self.ed.double_click(x, y)
    }

    /// A keyboard command for the canvas (`cancel`, `delete`, `nudge left`, ...).
    pub fn key(&mut self, command: &str, shift: bool, alt: bool) -> bool {
        self.ed.key(command, mods(shift, false, alt))
    }

    /// The pointer's look at (x, y): a CSS cursor.
    pub fn cursor(&mut self, x: f64, y: f64) -> String {
        self.ed.cursor(x, y).to_string()
    }

    /// The selection, handles and previews, as SVG in the diagram's coordinates.
    pub fn overlay(&mut self) -> String {
        self.ed.overlay()
    }

    /// What happened: `message\ttext`, `refused\ttext`, `edit-label`, `selection`, `tool`.
    pub fn events(&mut self) -> Vec<String> {
        self.ed
            .take_events()
            .into_iter()
            .map(|e| match e {
                Event::Message(m) => format!("message\t{m}"),
                Event::Refused(m) => format!("refused\t{m}"),
                Event::EditLabel => "edit-label".into(),
                Event::SelectionChanged => "selection".into(),
                Event::ToolChanged => "tool".into(),
            })
            .collect()
    }

    // -- the panels ---------------------------------------------------------------------------

    /// The properties panel for the selection (JSON, see `panel::Form`).
    pub fn panel(&mut self) -> String {
        json::form(&panel::form(&mut self.ed))
    }

    /// A field of the panel changed (JSON, see `panel::Outcome`).
    pub fn panel_set(&mut self, id: &str, value: &str, confirmed: bool) -> String {
        json::outcome(&panel::set(&mut self.ed, id, value, confirmed))
    }

    /// The diagram as a graph for arranging it, with the nodes the selection stands for (JSON, see
    /// `arrange::json::graph`, `arrange::selected`).
    pub fn arrange_graph(&mut self) -> String {
        let language = self.ed.session().language();
        let g = diagram_core::arrange::graph(self.ed.doc(), language);
        let selected = diagram_core::arrange::selected(self.ed.doc(), &g, &self.ed.selection);
        diagram_core::arrange::json::graph_with(&g, &selected)
    }

    /// The quality figures of the diagram as it is (JSON, with the score).
    pub fn arrange_figures(&mut self) -> String {
        let language = self.ed.session().language();
        diagram_core::arrange::json::figures(&diagram_core::arrange::figures(self.ed.doc(), language, None))
    }

    /// The figures the diagram would have with an arrangement (JSON from `lib/arrange.ts`); changes nothing.
    pub fn arrange_try(&mut self, arrangement: &str) -> Result<String, JsError> {
        let a = diagram_core::arrange::json::arrangement(arrangement).map_err(|e| JsError::new(&e))?;
        let language = self.ed.session().language();
        Ok(diagram_core::arrange::json::figures(&diagram_core::arrange::try_arrangement(
            self.ed.doc(),
            &a,
            language,
        )))
    }

    /// Writes an arrangement into the diagram, as one step that can be undone.
    pub fn arrange_apply(&mut self, arrangement: &str) -> Result<bool, JsError> {
        let a = diagram_core::arrange::json::arrangement(arrangement).map_err(|e| JsError::new(&e))?;
        Ok(self.ed.arrange(&a))
    }

    /// The Problems panel (JSON: summary, count of errors and warnings, rows).
    pub fn problems(&mut self) -> String {
        let hints = self.ed.show_hints;
        let (list, summary, count) = problems::listed(&mut self.ed, hints);
        json::problems(&list, &summary, count)
    }

    pub fn set_show_hints(&mut self, on: bool) {
        self.ed.show_hints = on;
    }

    pub fn fix_problem(&mut self, n: usize) -> bool {
        let hints = self.ed.show_hints;
        problems::fix(&mut self.ed, n, hints)
    }

    pub fn fix_all(&mut self) -> usize {
        let hints = self.ed.show_hints;
        problems::fix_all(&mut self.ed, hints)
    }

    /// Selects the element of a problem; its centre (x, y in pt), or nothing.
    /// Selects the element of a row of the attributes list (`12` or `12:3`, see `panel::LineageRow`):
    /// its centre (pt) for showing it, or nothing if it is gone.
    pub fn goto_element(&mut self, target: &str) -> Vec<f64> {
        let key = target.split(':').next().and_then(|k| k.parse().ok()).map(diagram_core::model::ElementKey);
        let Some(key) = key.filter(|k| self.ed.doc().get(*k).is_some()) else { return vec![] };
        self.ed.set_selection(vec![key]);
        let laid = self.ed.session_mut().laid();
        laid.gfx_for(key)
            .and_then(diagram_core::svg::bbox)
            .map_or(vec![], |(x0, y0, x1, y1)| vec![(x0 + x1) / 2.0, (y0 + y1) / 2.0])
    }

    pub fn goto_problem(&mut self, n: usize) -> Vec<f64> {
        let hints = self.ed.show_hints;
        problems::goto(&mut self.ed, n, hints).map_or(vec![], |(x, y)| vec![x, y])
    }

    /// The problems of the element at a point (pt), for its tooltip.
    pub fn tooltip(&mut self, x: f64, y: f64) -> String {
        let hints = self.ed.show_hints;
        match self.ed.element_at(x, y) {
            Some(key) => problems::tooltip(&mut self.ed, key, hints),
            None => String::new(),
        }
    }

    /// The text of the selected elements, with their problems (the Source tab).
    pub fn source(&mut self) -> String {
        problems::source(&mut self.ed)
    }

    /// The handles of the selected element: `name, x, y` side by side (x and y in pt).
    pub fn handles(&mut self) -> Vec<String> {
        self.ed.handles().into_iter().flat_map(|(n, (x, y))| [n, x.to_string(), y.to_string()]).collect()
    }

    pub fn selection_count(&self) -> usize {
        self.ed.selection.len()
    }

    // -- the Relational Schema / SQL window ----------------------------------------------------

    /// What the window shows (JSON, see `rmwindow::View`).
    #[allow(clippy::too_many_arguments)]
    pub fn relational(
        &self,
        format: &str,
        rel: &str,
        fk: &str,
        marks: &str,
        text_types: bool,
        quote_all: bool,
        drop: bool,
        source: &str,
    ) -> String {
        json::relational(&rmwindow::view(
            self.ed.doc(),
            &rm_options(format, rel, fk, marks, text_types, quote_all, drop, source),
        ))
    }

    pub fn rm_choose(&mut self, element: &str, index: usize, rel: &str, fk: &str) -> bool {
        let o = rm_options("text", rel, fk, "underscore", false, false, false, "");
        rmwindow::choose(&mut self.ed, element, index, &o)
    }

    pub fn rm_set_type(&mut self, row: usize, value: &str) -> bool {
        rmwindow::set_type(&mut self.ed, row, value)
    }

    pub fn rm_store_guesses(&mut self) -> bool {
        rmwindow::store_guesses(&mut self.ed)
    }

    // -- GSM -> ER ---------------------------------------------------------------------------

    /// The ER diagram for this GSM diagram, with the defaults of new ER diagrams (`names`,
    /// `values`: the settings): JSON with its text, the questions and the notes; or an error.
    #[allow(clippy::too_many_arguments)]
    pub fn convert_to_er(
        &self,
        attr_names: &str,
        add_keys: bool,
        choice_keys: Vec<String>,
        choice_values: Vec<String>,
        names: Vec<String>,
        values: Vec<String>,
        source: Option<String>,
    ) -> Result<String, String> {
        let settings = ConversionSettings {
            attr_names: match attr_names {
                "type" => AttrNames::Type,
                "edge" => AttrNames::Edge,
                _ => AttrNames::Auto,
            },
            add_keys,
            choices: choice_keys.into_iter().zip(choice_values).collect(),
        };
        let mut c = diagram_core::gsm2er::convert(self.ed.doc(), &settings, source.as_deref())
            .map_err(|e| e.to_string())?;
        let kept: std::collections::HashMap<String, String> = names.into_iter().zip(values).collect();
        let notes = newdefaults::apply_defaults_to_generated(&mut c.doc, &|n: &str| kept.get(n).cloned());
        c.report.extend(notes);
        Ok(json::conversion(&c))
    }

    /// Preferences' "Use for the open diagram": these style values into the `\setup`, as one step.
    pub fn use_defaults(&mut self, keys: Vec<String>, values: Vec<String>) {
        let pairs: Vec<(String, String)> = keys.into_iter().zip(values).collect();
        self.ed.apply("Settings for this diagram", |ed| {
            newdefaults::write_values(ed.session_mut().doc_mut(), &pairs)
        });
    }

    /// Selects the elements with these ids (for tests and the Problems panel).
    pub fn select_ids(&mut self, ids: Vec<String>) {
        let keys = ids.iter().filter_map(|i| self.ed.doc().by_id(i).map(|e| e.key)).collect();
        self.ed.set_selection(keys);
    }

    pub fn select_all(&mut self) {
        self.ed.select_all();
    }

    pub fn clear_selection(&mut self) {
        self.ed.set_selection(vec![]);
    }

    /// What Copy puts on the clipboard (empty: nothing selected).
    pub fn copy(&mut self) -> String {
        self.ed.copy()
    }

    pub fn cut(&mut self) -> String {
        self.ed.cut()
    }

    /// Pastes text from the clipboard, its middle at (x, y) cm.
    pub fn paste(&mut self, text: &str, x_cm: f64, y_cm: f64) {
        self.ed.paste(text, Some((x_cm, y_cm)), None);
    }

    pub fn duplicate(&mut self) {
        self.ed.duplicate();
    }

    pub fn delete_selection(&mut self) {
        self.ed.delete_selection();
    }

    /// A right click: `edge`, `node`, `one`, `many` or `none` (what the menu offers).
    pub fn context(&mut self, x: f64, y: f64) -> String {
        self.ed.context(x, y).to_string()
    }

    pub fn straighten(&mut self) {
        self.ed.straighten();
    }

    pub fn add_attribute_list(&mut self) {
        self.ed.add_attribute_list();
    }
}
