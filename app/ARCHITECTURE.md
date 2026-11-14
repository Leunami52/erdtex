# ERDTeX: how it's built

How to use it is in [`README.md`](README.md). This file explains the structure, the rules the code follows,
how to add each kind of thing, and how it is tested. ERDTeX was ported from an earlier
version in Python and Qt; while it was ported, every result was compared with that version's, case by case.

## The parts

```
crates/core/        the core: everything that doesn't need a window
crates/core-wasm/   the core's interface for the page (WebAssembly)
crates/pdf-wasm/    the website's PDF export (svg2pdf as WebAssembly, loaded when needed)
src-tauri/          the Rust side of the desktop app: files, settings, pdflatex, exports, self-test
frontend/           the page: TypeScript and Svelte, built with Vite
e2e/                the window driven by WebDriver (tauri-driver)
e2e-web/            the website in Chromium, Firefox and WebKit (Playwright)
texts/              the website's privacy note, and the imprint (empty: Help shows no entry)
fonts/              the Latin Modern fonts, every design size LaTeX uses
rulebook/           the rule books (Help → Rule Books), one Markdown page per rule
samples/            the sample diagrams (File → Open Example)
test_diagrams/      diagrams the tests use
licenses/           the licences of the Rust crates built in (tools/licenses.sh)
spike/              the start-up measurement and the arranging harness
```

The core runs inside the page as WebAssembly, also in the desktop app, so that the browser version later
gets the same code. The Rust side of Tauri only does what a page can't.

## The core's modules

In the direction of their dependencies; a module only uses those above it:

| Module | What it does |
|---|---|
| `text` | String helpers that behave like the Python functions the file format began with (`strip`, number formatting) |
| `texscan` | The scanner for the file syntax: finds arguments exactly as TeX and xparse do |
| `model` | Elements, coordinates, options, attribute items; canonical output |
| `document` | A diagram file: reading, lossless writing, editing helpers, and the table of commands |
| `latex_text` | Approximate typesetting of labels (fonts, sizes, maths, special characters) |
| `colors`, `style` | xcolor expressions; the packages' style keys and their defaults |
| `fonts` | Text metrics from the bundled fonts: as LaTeX sets them (design sizes), or as the Python app measures (for comparing) |
| `geometry` | Node shapes, border points, anchors and edge segments as TikZ computes them |
| `cardinality`, `generalisation` | What labels and generalisation types mean; the two notations, converting between them |
| `rules` | The modelling rules: connections the editor refuses (`can_connect`, `refusal` with its rule), and `check` with its problems and fixes; each problem names its rule in the rule books (`rulebook/`, see its README) |
| `layout` | The layout engine: a document as drawing primitives, per element (see below) |
| `svg` | A layout as SVG, text as glyph outlines or as `<text>`: a `Writer` decides what is drawn (colours, double lines, arrows, glyphs, underlines) and paints through a `Painter`, one for each form |
| `subdomains` | Which box an element lies in (from the layout) |
| `lineage` | Every attribute of an entity or object type, its own and the inherited ones (IS_A, CAN_BE; GSM generalization, specialization), each with where it is defined and every chain it is inherited along (the Element pane's list) |
| `relational` | ER -> relational schema with its decisions, each naming its rule (`Decision::rule`, see below); `names` (identifiers, type guesses), `format` (text, LaTeX, Markdown, SQL for PostgreSQL, Oracle and SQLite) |
| `gsm2er` | GSM -> a new ER diagram, with the questions it asks (each naming its rule, `ConversionChoice::rule`) and a report |
| `newdefaults` | What new diagrams start with; checking style values |
| `shortcuts` | The table of commands and their default keys, as Qt writes them; conflicts |
| `history`, `session` | One open diagram: the document, undo and redo (snapshots of the text and the selection), the last layout; an edit may span several events (`begin`, `end`, `cancel`) |
| `editor` | Editing, the port of `editor.py`: tools, hit testing, selection, moving with snapping, handles, connecting with the rules, the clipboard, the overlay (see below) |
| `panel` | The properties panel as data: the form for the selection and what its fields change (see below) |
| `problems` | The Problems panel: rule check and layout warnings, fixes, the badges on the canvas |
| `rmwindow` | The Relational Schema / SQL window: output, decisions with each option's relations, data types |
| `export` | The LaTeX job's wrapper and reading pdflatex's errors, the `\input` line, the editor rendering as SVG |
| `arrange` | Arranging: the diagram as a graph (exact sizes, each attribute list's box on all eight sides, subdomain boxes, generalisation order), an arrangement written back (positions on a 0.05 cm grid, list sides, way points, boxes as rectangles, the helper grid fitted), and the quality figures measured on the layout; JSON for the page behind the `json` feature |

### The layout engine

`layout/` is a port of what the packages compute in TeX, by way of the Python app's `layout.py`. One module
per kind of thing: `prims` (paths, text, handles), `text` (text boxes; the `TextEngine` that measures),
`position` (coordinates, anchors, `right=1cm of a`), `nodes`, `edges` (arrow heads, cardinalities, roles,
crow's foot, GSM labels), `attributes` (compact lists and ovals), `other` (notes, subdomains, the grid).

- An error while laying out one element becomes a warning on that element, and what was drawn before
  it stays. Functions return `Fallible`, and stop with `?` at the same points as the Python code did.
- Python's arithmetic is kept where it differs from Rust's: `py_mod` for `%`, `round_half_even` for
  `round()`, `py_sum` for `sum()` of floats (compensated since Python 3.12; it decides ties between
  boxes of the same area), `py_float` / `py_int` with Python's error messages.
- Steps that need all edges first (ovals, cardinalities beside edges, labels beside shapes) are
  collected while laying out the elements and done afterwards, in that order.

### The relational schema

`relational/transform.rs` follows the Python `Transformer` phase by phase: attributes, generalisations,
weak entities, entity relations, keys (supertypes and owners first), categorisation, relationships,
multi-valued attributes, views, unique names, subdomain boxes.

- Relations live in an arena (`Schema::tables`) and are referred to by `TableId`, because the Python code
  shares and moves them between lists; `Schema::relations` is the schema in order. A relation merged into
  another stays in the arena and leaves the list.
- Columns carry a `uid`, so that a column can be found again after it was renamed or moved, as the
  Python code holds on to the object.
- A decision is a list of `Choice`s, each with the element options it sets (`rm=...`); `Overrides` try
  other choices without changing the document. The default choice writes nothing.
- Python's dict order is kept where it shows (`Ordered` in `transform.rs`), and Python's integers in
  labels: a `Count` has any number of digits.
### The editor

`editor.rs` takes pointer and key events in diagram coordinates (TeX points, y up) and changes the document
through its `Session`; the page only reports the events and draws what comes back. So editing works the
same in the desktop app and a browser, and is tested in Rust (`tests/editor.rs`, the Python app's editor
tests) as well as with real clicks (`e2e/test_editing.py`).

- What the window should show comes back as `Event`s (a message, a refusal to read, the label to edit,
  the selection or the tool changed); `overlay()` draws the selection, handles, the rubber band and the
  connection being drawn, in red or green.
- A drag is one undoable step: `begin` on the press, `end` on the release, `cancel` with Esc. A drag that
  is interrupted (a tool chosen, another edit, undo, a press without a release) is ended first, so nothing
  escapes the history; the property tests check that undoing everything gives back the file.
- Elements are referred to by `ElementKey`; across undo, the selection is found again by id, or as the
  n-th element of its kind (as the Python app does).

### The panels and dialogs

The properties panel is described by the core, not built by the page: `panel::form` gives the rows for
the selection (a label, an id and a field: text, tick, drop-down, line, label, number, position, colour,
data type, cardinality picker, attribute table, problem with its fix), and `panel::set(id, value)`
applies what a field changed, as one undoable step. Ids say what they change: `opt:<key>` an option of
the element, `setup:<key>` the diagram's `\setup`, `attr.<what>.<row>` the attribute table, `fix.<n>`
a problem's fix. A change that needs the user's agreement (a change of notation) comes back as
`Outcome::Ask` and is sent again once agreed; a refusal or an invalid value comes back too. So the
panel is tested in Rust (`tests/panel.rs`: the Python app's panel tests), works in a browser, and a
new field is a row in `panel.rs` — the page draws every kind of field already.

The Relational Schema / SQL window (`rmwindow`) and GSM → ER (`gsm2er` with `newdefaults`) work the same
way; `core-wasm` hands their data to the page as JSON (`json.rs`, written by hand in Milestone 6 to keep
the WebAssembly small; if it grows, `serde_json` is the tried and trusted replacement).

## The window

`frontend/src/`:

| File | What it does |
|---|---|
| `App.svelte` | The window: menus, the keyboard, the test hook (`window.__erdtex`) |
| `lib/app.svelte.ts` | Tabs and what the menus do, as the Python main window: files, saving with its checks, unsaved changes, reload, the tabs restored at the next start |
| `lib/view.ts` | The view arithmetic: zoom around a point, fit, the grid's steps (pure, tested with Vitest) |
| `lib/keys.ts` | Key presses in Qt's form (`Ctrl+Shift+Z`), with the physical key as a fallback, matched against the core's table of shortcuts |
| `lib/theme.ts` | The four display modes and their colours |
| `lib/backend.ts` | What the page asks of the Rust side (a browser version replaces this module) |
| `lib/core.ts` | Starts the WebAssembly core and registers its fonts |
| `lib/icons.ts` | The tools' icons, after the Python app's palette |
| `lib/docks.svelte.ts` | The panes (Tools, Element, LaTeX Preview, Problems): docked left, right or bottom, floating, folded or closed; kept as "dock layout" |
| `lib/actionIcons.ts` | The icons of the menus and the toolbar (Lucide) |
| `lib/panel.ts` | The panel's data types, and what text fields share (commit only when changed, a field's own Ctrl+Z) |
| `lib/arrange.ts`, `lib/arrangeWorker.ts` | Arranging's candidates, computed with WebCola in a worker: list sides, a local improvement against crossings and lines close together, overlaps removed level by level with WebCola's VPSC solver (inside each subdomain box, then the boxes), elements on crossing lines moved to free spots (`relocate`), lines routed around elements and apart from one another (our own router: one or two bends at elements' corners at several distances, the cheapest by the core's weights); the core measures each candidate and the best is applied (`App.arrange`) |
| `components/dialogs/LicensesDialog.svelte` | Help → About → Licences…: `LICENSE`, `latex_packages/LICENSE`, the fonts' licence, `licenses/rust-desktop.txt` (cargo-about, `tools/licenses.sh`, checked by `tools/check_rust.sh`) and `licenses-javascript.txt` (the page's and its workers' libraries, listed by rollup-plugin-license when the page is built, `vite.config.ts`); loaded when opened |
| `lib/examples.ts` | The sample diagrams (`samples/festival.erd`, `festival.gsmd`) for *File → Open Example*, loaded on demand |
| `lib/backend.ts`, `lib/platform/` | What the page asks of its platform: `desktop.ts` (Tauri: files, dialogs, settings, pdflatex, the window), `web.ts` (the website: files without paths, settings and open diagrams in local storage, exports in the page and `crates/pdf-wasm`, the service worker), `common.ts` (what they share), `same.ts` (a type check that both offer the same functions). The build chooses one through the alias `#platform` (`vite.config.ts`: `vite build` for the desktop, `vite build --mode web` for the website, into `dist-web/`, with relative paths) |
| `lib/links.ts` | The repository's, the releases' and the website's addresses, the privacy note and the imprint (`texts/`) |
| `lib/latexPackages.ts` | The LaTeX packages (`../latex_packages/`) and their manuals built in, loaded on demand: *Help → Save LaTeX Packages…*, their versions for About |
| `lib/rulebook.ts` | The rule books (`rulebook/`): the pages loaded on first opening, their fields (TOML, `smol-toml`) and text (Markdown, `marked`), examples as figures, rule ids as links, the search |
| `lib/pdf.ts` | Draws a PDF page with pdf.js (the preview; PNG of the LaTeX result without pdftocairo), loaded when first needed |
| `components/` | `Canvas` (mouse events to the editor, its overlay, the context menu), `Palette`, `Toolbar` (with Snap), `MenuBar`, `ContextMenu`, `StatusBar`, `Question` |
| `components/groups/` | `Groups` (the editor area: groups of tabs side by side, laid out by dockview; a canvas per tab; dragging tabs), `GroupTab` (a tab: its name, ×, its menu) |
| `lib/groups.ts` | What splitting, moving and the keys do with the groups (over dockview's interface), without a window |
| `components/panel/` | The Element dock (`Inspector`: `Properties`, Source) and its fields (`LabelField`, `ColorField`, `CardField`, `AttributeTable`); `Problems`; `Preview` (the LaTeX preview) |
| `components/dock/` | `DockArea` (the panes of one side, with splitters; a strip when all are folded), `DockPanel` (title bar: fold, float/dock, close; drag to detach and dock) |
| `components/prefs/` | Preferences and its six pages |
| `components/dialogs/` | `ConvertDialog` (GSM → ER, with its preview), `RelationalWindow` (doesn't block the window), `ExportDialog`, `ArrangeDialog` and `ArrangeResult` (the bar with the figures, Keep and Undo), `RuleBook` (Help → Rule Books…, loaded when first opened; examples drawn by the core) |

- Each tab holds a `Diagram` (the core's `Session`, through WebAssembly). Every change goes through it, so
  it can be undone; the page bumps the tab's `version`, and what depends on the document is drawn again.
- Panning is the compositor's work only, nothing is painted again for it: the grid is drawn once on a
  canvas a little larger than the view and moved, shifted by its own spacing (it repeats); the axes are
  thin lines that are moved; the diagram and the overlay are on a layer that is moved (CSS transforms on
  device pixels); only the ruler numbers are drawn again, on two thin strips. Pointer movements are
  applied once per frame. Zooming draws the diagram again, because the
  thinnest line is one device pixel.
- "System" mode takes the desktop's palette where there is one (a KDE colour scheme, `desktop.rs`), as
  the Python app does. The dark canvas is a
  drawing option of the core (`svg::Colors`), so files, LaTeX and exports never see it.
- The menus are HTML, not the system's: the browser version needs them anyway, and shortcuts can be
  changed: a changed one is kept in the settings as `shortcuts/<id>`. Tools have a set per kind of diagram.
- The canvas and the tab dragging listen to mouse events, not pointer events: after a right click, WebKit
  (at least through WebDriver) keeps the right button as pressed, and then sends a `pointermove` instead
  of a `pointerdown` for every later press (the right button's release arrives as a `mouseup` of button 0
  with `buttons` 2, and another right click doesn't clear it). What starts on `pointerdown` stops
  responding then in the tests: dockview's splitters, and the panes' dragging and splitters. **With a real
  mouse all of these keep working** (checked by hand, 2026-11-08): it is the WebDriver's simulated right
  click. So a window test that needs those after a right click has to do them first, or start anew.
- **Tabs side by side** (the decisions: the root README, "Tabs side by side"): dockview (`dockview-core`, MIT, pinned at 8.9.0) lays out the
  groups; each tab is a dockview panel, by the tab's id, whose content is a `Canvas` and whose tab is a
  `GroupTab`, both Svelte components mounted into dockview's elements. `app.tabs` stays the list of open
  tabs; `Groups.svelte` keeps dockview's panels in step with it, and dockview's active panel with
  `app.currentId`, which everything else follows. dockview's own dragging is off (`disableDnd`: it is HTML
  drag and drop, which the window takes for dropping files); ours calls `lib/groups.ts`, which moves
  panels with dockview's `moveTo`. The page imports dockview's single-file build, because only it carries
  the styles (the module build has none); its look comes from our palette through dockview's CSS
  variables. A closed tab's diagram is freed in the core only after the page has updated (`tick`),
  because its canvas stays mounted until dockview removes its panel. **Context menus are drawn by the
  window** (`app.menu`, in `App.svelte`), never inside dockview: its tab bars have `overflow` and a
  `transform`, which clip anything fixed-positioned inside them. A canvas in a group is mounted before
  dockview shows it, and the ResizeObserver doesn't always report the size it gets then, so it measures
  itself again on dockview's `layout` and `onShow` and when its tab becomes current (`measure`). The app
  follows only the user's changes of dockview's active panel (`origin: "user"`): its own (adding, moving,
  closing) have already decided the current tab, and dockview reports some of them late.
- **The groups at the next start:** on quitting, `closeAll` keeps dockview's layout (`toJSON`) with each
  tab replaced by its file (`keepLayout` in `lib/groups.ts`; tabs without a file left out) as the setting
  `tab layout`, and the open tabs in the order they appear. At the start, once the tabs are open again,
  `Groups.svelte` gives the files their new tabs (`restoreLayout`: files gone are left out, an empty
  group goes, a split left with one part gives way to it, and the root turns if it has to) and loads it
  with `fromJSON`; tabs not in it (from the command line) then go into the active group. The pure part
  is tested with Vitest (`lib/groups.test.ts`).
- The clipboard is the system's (Tauri's plugin); tests use one inside the app (`ERDTEX_PRIVATE_CLIPBOARD`).
- File dialogs: Tauri's dialog plugin, on Linux through the desktop portal (its `xdg-portal` feature), so
  that the desktop's own dialog appears (KDE's on Plasma); Zenity where no portal runs. The packages
  recommend `xdg-desktop-portal`.
- Keys and fields, as in the Python app: placing an element puts the cursor in its label; Esc keeps what
  was typed and returns to the canvas; Ctrl+Z takes back a field's own typing first, then edits of the
  diagram; plain characters and Ctrl+A/C/X/V stay with the field, and so do Ctrl with Backspace,
  Delete, the arrows, Home and End when no command has them (`FIELD_KEYS`: deleting and moving by word;
  every other unassigned Ctrl key is held back from the WebView, which would reload, find or print); a
  dialog keeps its keys. A button of the Element pane that is a command too shows its key
  (`BUTTON_COMMANDS` in `Properties.svelte`), and the command uses the pane's button while it is shown
  (`app.panelButton`), so that the cursor goes where the button puts it.
- The LaTeX preview runs pdflatex on the Rust side (with the packages bundled into the program, so
  that the TeX installation's own versions don't matter), one run at a time; the page shows the PDF
  with pdf.js, which is loaded only when first needed (start-up).
- Exports: the editor rendering is the core's SVG, turned into PDF by `svg2pdf` and into PNG by `resvg`
  on the Rust side; the LaTeX result comes from pdflatex, and pdftocairo makes SVG and PNG of it (without
  pdftocairo, the page draws the PNG with pdf.js).
- Files changed by other programs: the page asks for each open file's change time every 1.5 s; a
  change reloads the tab (asked first with unsaved changes; the app's own saves are recognised).
- `src-tauri/src`: `files.rs` (reading and writing, with the Python app's checks: other files refused,
  line endings kept; what counts as a diagram file), `settings.rs` (a TOML file in the config folder, or
  in `ERDTEX_CONFIG_DIR`), `latex.rs` (pdflatex), `export.rs` (both kinds of export), `desktop.rs`
  (a KDE colour scheme for System mode), `refresh.rs` (switches off WebKit's preference for 60 frames a
  second; no effect with WebKitGTK 2.54, kept for later versions), `selftest.rs` (`--self-test` without the
  window; the window's part runs in `App.svelte`), `main.rs` (GIO's network monitor), `lib.rs` (the
  commands, the window, the self-test's start).
- `src-tauri/tauri.linux.conf.json`: the product name `erdtex` on Linux, which names the deb and rpm
  packages (Tauri takes the package name from the product name, and "ERDTeX" would give `erd-te-x`).
- `src-tauri/packaging/`: the menu entry (`erdtex.desktop`, Tauri's template for its own entry, which
  is named after the product name and so matches the window's app id, as Wayland needs
  for the icon) and the MIME types of `.erd` and `.gsmd`.
- **Arranging** (the decisions: the root README, "Arranging a diagram automatically"): the core gives the
  diagram as a graph (`arrange::graph`: each node's exact size, each attribute list's box on all eight
  sides, the subdomain boxes with their outlines, connectors between boxes, generalisation order, the
  nodes the selection stands for). `lib/arrange.ts`, in a worker (`arrangeWorker.ts`), computes some ten
  candidates: (anew: WebCola's layout first;) list sides; a local improvement; overlaps removed level by
  level with WebCola's VPSC solver; elements on crossing lines moved to free spots; lines routed by our own
  router. `App.arrange` has the core measure each (`arrange_try`: written into a copy, laid out, the
  figures taken from the drawing) and applies the best (`arrange_apply`, one undoable step) only if it
  beats the diagram as it is. Elements that stay (arranging the selection, or keeping it in place) are
  fixed in every step and not written back. `spike/arrange_check.sh` does the same outside the app for
  every repository diagram, for comparing the figures.

## Rules

- **No `unsafe` in the core:** `core` and `core-wasm` have `#![forbid(unsafe_code)]`. `src-tauri` has four
  `unsafe` places where it talks to the operating system, each with a `SAFETY` comment: environment
  variables set while there is only one thread (`main.rs`, the self-test in `lib.rs`), `AttachConsole` on
  Windows (`lib.rs`), and `dlsym` for a WebKit function (`refresh.rs`). A new one needs the same comment
  and a reason the safe API can't serve.
- **The core never panics on its input.** Clippy denies `unwrap`, `expect`, `panic!`, `unreachable!` and
  unchecked indexing (`v[i]`, `&s[a..b]`) in the core outside tests. Use `get`, `text::slice`, and
  errors as values (`ParseError`, `ItemError`).
- **Positions in strings are byte offsets at ASCII characters**, which are always char boundaries. Where
  Python worked on characters (accents, control words), the code works on `char`s.
- **A changed result is a decision.** The snapshot tests (`tests/snapshots.rs`) hold every test diagram's
  results; a change shows up there and is accepted (`cargo insta review`) only when it is meant.
- **Every public item has a doc comment.**

## Lossless files

Every element keeps the text it was read from (`Meta::source`), the white space and comments before it
(`leading`) and the rest of its line (`trailing`). An unchanged element is written back verbatim; a
changed one (`touch()`) in canonical form. So loading and saving an unchanged file reproduces it byte for
byte, which a test checks for every diagram in the repository and a property test for random input.

## How to add …

- **a command of the file format:** a row in `ER_COMMANDS`, `GSM_COMMANDS` or `COMMON_COMMANDS`
  (`document.rs`): its name, its arguments in xparse letters (`O` options, `m` mandatory, `d` optional
  coordinate, `r` coordinate, `g` optional trailing group) and the element it becomes. A new kind of element
  is a new `ElementType`, a `Body` variant and its `to_tex`.
- **a style key:** a row in `ER_DEFAULTS` / `GSM_DEFAULTS` (`style.rs`), and the same key in the package.
- **a kind of diagram:** a `Kind` variant and its command table.
- **a font size or family:** the font file in `fonts/` and its range in `fonts::design_font`, copied from
  LaTeX's font definition file.
- **a label command** (such as a new symbol): a row in the tables of `latex_text.rs`.
- **a node shape:** a `ShapeKind` variant in `geometry.rs` (outline, border point, anchors, bounding box,
  containment) and its size function, ported from PGF; then the case in `layout/nodes.rs` that chooses it.
- **an option of an element:** read it where that element is laid out (`layout/nodes.rs`, `edges.rs`, ...),
  as the package reads it.
- **a rule check:** a function in `rules.rs` called from `check_er` or `check_gsm`; a problem names its
  element, and a button that solves it is a `FixAction` (a new kind of change is a new variant and its
  case in `Fix::apply`).
- **a decision of the transformation:** where the Python code has one, `transform.rs` builds `Choice`s
  and calls `decide`; the choice is read back from the element's `rm...` options with `opt`.
- **an SQL dialect:** its name in `DIALECTS`, its types in `sql_type`, and its differences in `to_sql`;
  then run its scripts in that database by hand, and add its snapshot.
- **a command with a shortcut:** a row in `shortcuts::COMMANDS`.
- **a main choice for new diagrams:** a `Toggle` in `newdefaults.rs`.
- **a menu entry:** an action in `App.svelte`'s `actions` (by the command's id, so it gets its shortcut)
  and an `item(...)` in `menus`; what it does goes into `App` (`lib/app.svelte.ts`).
- **an edit of the document:** a method of `Editor` that changes the document inside `apply(label, ...)`
  (the label is what Undo shows), offered by `Diagram` in `core-wasm`; the page calls it through `app.edit`.
- **a field of the properties panel:** a row in the right `*_form` function of `panel.rs` and, if it
  isn't an option (`opt:<key>`), what its id does in `panel::set`; a test in `tests/panel.rs`.
- **a tool:** a row in `node_tools`, `edge_tools` or `special_tools` (`editor.rs`) and what `place` does
  with it; its icon in `lib/icons.ts`, its tooltip in `editor::tip`, its shortcut in `shortcuts::COMMANDS`.

## Tests

- unit tests next to the code (`cargo test -p diagram-core`)
- `crates/core/tests/files.rs`: every diagram of the repository round-trips
- `crates/core/tests/properties.rs`: random input never panics, and is written back unchanged; laying it out
  and writing it as SVG never panics either
- `crates/core/tests/layout.rs`: layout tests (from version 2's), with reference values from LaTeX
- `crates/core/tests/semantics.rs`: rules, schemas and GSM -> ER on worked examples (the diagrams in
  `tests/data/rm_examples.json` and `test_diagrams/`)

- `frontend/src/lib/*.test.ts`: the page's pure parts (`npm test`, Vitest): the view arithmetic, the
  layout of the groups kept for the next start, and arranging (overlaps removed, generalisation order,
  subdomain boxes, routing)
- `e2e/test_window.py`: the real window through WebDriver (`tauri-driver` with WebKitWebDriver): every
  diagram of the repository opens and is saved unchanged, undo and redo with the keys, unsaved changes,
  refused files, the display modes, the tabs at the next start; screenshots in `target/e2e/`
- `e2e/test_editing.py`: editing with real clicks, drags and keys: the Python app's editor tests through
  the palette, the canvas and the keyboard
- `e2e/test_panels.py`, `e2e/test_dialogs.py`: the panels, Preferences, GSM → ER and the Relational
  Schema / SQL window with real clicks and typing, and the focus rules (`test_focus.py`)
- `tests/panel.rs`, `tests/dialogs.rs`: the Python app's panel, Preferences, GSM → ER and relational
  window tests, on the core's data
- `e2e/test_files.py`: the LaTeX preview (errors, the last good result, newest change wins), every export,
  Copy `\input` Line, Open Recent and refused files, files changed on disk
- `src-tauri` tests: the editor rendering in all three formats (the PDF's page size), pdflatex with its
  errors, the LaTeX result through pdftocairo
- `e2e/test_docks.py`: the panes dragged, docked, folded, closed and brought back, the layout kept; the
  tab bar (dragging, the tab's menu, no page menu)
- `e2e/test_groups.py`: tabs side by side with real pointer input and keys: split off to each side, moved
  between groups and reordered, by dragging, the tab's menu and the keys; the splitter; the panels and the
  keys following the group clicked in; a group closing with its last tab; the tabs and the layout at the
  next start (also with a file gone, and a file changed by another program in a group not active)
- `e2e/test_arrange.py`: Edit → Arrange Diagram…, tidy and anew, the result bar, Keep and Undo; every
  diagram of the repository tidied in the window: no overlaps left, never worse, one Ctrl+Z gives back the file
- `spike/arrange_check.sh [anew]`: arranging outside the app (Node.js and the core's example
  `arrange_check`) for every repository diagram: the figures before and after, each result reloaded and
  compiled with LaTeX, the candidates' figures in `target/arrange/`
- `e2e-web/website.spec.ts` (`npx playwright test`, after `npm run build:web`): the website served under a
  sub-path as GitHub Pages serves it, in Chromium, Firefox and WebKit (`tools/check_rust.sh` runs all three in
  Playwright's container image where podman is installed): the samples on the first visit, opening, saving
  (as a download), keeping diagrams and settings across reloads, refusing a non-diagram, PNG, SVG and PDF
  exports, the LaTeX preview greyed out, the packages as a zip, the website's keys and Help entries, working
  offline
- `tools/check_sql.py`: every test diagram's SQL (from the snapshots) executed twice in SQLite, and in
  PostgreSQL where it is installed
- `crates/core/tests/samples.rs`: the sample diagrams have no problem of any kind, nothing overlapping, no line
  through an element or across another, and convert
- `crates/core/tests/snapshots.rs`: every test diagram (`test_diagrams/`, `tests/data/rm_examples.json`) with its
  problems, relational schema and SQL or GSM → ER result, and where each element is drawn, kept as snapshots
  (`tests/snapshots/`, insta): a changed result fails with the difference; `cargo insta review` accepts it
- `crates/core/tests/rulebook.rs`: the rule books: every page follows the template, ids are unique and
  links lead somewhere, every example gives what it says (run through the checks, the drawing rules or the
  conversions; a notation's `shows` example is only read and drawn), every rule id in the code has its page and every check's page is named by the code, and
  every example is drawn cleanly (no overlaps, no line through an element, measured as arranging does)
- `e2e/test_rulebook.py`: Help → Rule Books… and `Shift+F1`, the books collapsed at first, a page with its
  examples drawn, the search, links and Back, and a rule opened from a problem, a refused line, a decision of
  the Relational Schema window and a question of the GSM → ER dialog
- `e2e/measure_pan.py`: frame times while panning a maximised, zoomed window, and with nothing happening
  (on the real display, on purpose: the other window tests run on Xvfb)
- `--self-test` (`selftest.rs`, and the page's checks in `App.svelte`): an installed build checks itself;
  `tools/test_rust_release.sh` runs it on Ubuntu, Debian, Fedora and under Wine
- property tests of editing: random clicks, drags and keys on random and real diagrams never panic, and
  undoing everything gives back the file (`PROPTEST_CASES=4000 cargo test --release --test properties editing`
  for a thorough run)

`tools/check_rust.sh` runs all of it (the SQL in SQLite and PostgreSQL; the window where there is a display),
plus rustfmt, clippy, the WebAssembly build and the page's checks.
