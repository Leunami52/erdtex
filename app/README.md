# ERDTeX

A graphical editor for the diagram files (`.erd`, `.gsmd`) of the `erdiagram` and `gsmdiagram` LaTeX
packages. It knows the modelling rules, shows a LaTeX preview, turns ER diagrams into a relational
schema with SQL for PostgreSQL, Oracle and SQLite, converts GSM diagrams to ER, and exports PNG, PDF
and SVG.

It runs on the desktop (Linux and Windows) and in the browser. How it is built:
[`ARCHITECTURE.md`](ARCHITECTURE.md).

## Installing

| System | File | Needs |
|---|---|---|
| Debian, Ubuntu (22.04 and newer) | `ERDTeX-4.0.0-amd64.deb`: `sudo apt install ./ERDTeX-4.0.0-amd64.deb` | WebKitGTK 4.1 (installed with it) |
| Fedora | `ERDTeX-4.0.0-x86_64.rpm`: `sudo dnf install ./ERDTeX-4.0.0-x86_64.rpm` | WebKitGTK 4.1 (installed with it) |
| Windows 10 and 11 (64-bit) | `ERDTeX-4.0.0-setup.exe` (for the current user, no administrator rights) | WebView2 (part of Windows 11; the installer fetches it where it is missing) |

The packages add a menu entry and register `.erd` and `.gsmd` files, so they open with ERDTeX
by double click. On Linux, Open, Save As and Export show the desktop's own file dialog (KDE's on Plasma,
GNOME's on GNOME), through the desktop portal (`xdg-desktop-portal`, part of both desktops); where no
portal runs, Zenity's dialog is used if it is installed. On Windows, the Start-menu entry is in a folder "ERDTeX", and the app is
uninstalled in *Settings → Apps → Installed apps*. The installer isn't code-signed, so Windows
SmartScreen may warn at the first start ("Windows protected your PC" → *More info* → *Run anyway*).

The LaTeX preview and the export of the LaTeX result need `pdflatex` (TeX Live, or MiKTeX on Windows, on
the PATH); everything else works without it. `pdftocairo` (poppler) is used for the LaTeX result as SVG
or PNG where it is installed.

**A newer version** is installed the same way, over the old one: `sudo dnf install ./ERDTeX-4.0.1-x86_64.rpm`
(or `sudo apt install ./….deb`) replaces it; there is no need to remove it first. dnf and apt only
replace it with a higher version number; the same version again needs `sudo dnf reinstall ./….rpm`
(`sudo apt install --reinstall ./….deb`). The settings stay. On Windows, run the newer installer.

`erdtex --self-test` checks an installation: the bundled packages and fonts, drawing, every
export, the relational schema and SQL, GSM → ER, the shortcuts, then the window
(`--no-window`: without opening it).

## In the browser

The same program runs as a website (its address comes with the first release), in the current Chrome,
Edge, Firefox and Safari. Everything works there as described in this guide, except:

* **No LaTeX preview and no LaTeX result** to export: they need pdflatex, which a browser doesn't have.
  *View → LaTeX Preview* and the export dialog's *LaTeX result* are greyed out and say so; the editor
  rendering exports as PNG, PDF and SVG as on the desktop.
* **Files:** *Open* uses the browser's own file picker (Chrome, Edge) or an upload (Firefox, Safari);
  diagrams can also be dropped onto the page. *Save* writes back into the same file where the browser
  allows it (Chrome, Edge) and is a download elsewhere. There is no *Open Recent*, no *Reload from Disk*
  (a page can't watch files) and no *Quit*. *Help → Save LaTeX Packages…* gives one zip.
* **Kept in the browser:** your settings, and the diagrams open when you leave the page: after a reload
  they come back as unsaved tabs. Nothing is sent anywhere (*Help → Privacy*). On the very first visit
  the two sample diagrams open.
* **Offline:** after the first visit the program is kept in the browser, so it starts without a network
  and loads almost nothing again. When a new version is out, it asks before it reloads.
* **Keys** a browser keeps for itself have other defaults (changeable in Preferences as any key):

  | Command | Desktop | Browser |
  |---|---|---|
  | New ER diagram / New GSM diagram | `Ctrl+N` / `Ctrl+Shift+N` | `Alt+N` / `Alt+Shift+N` |
  | Close tab | `Ctrl+W` | `Alt+W` |
  | Next tab / Previous tab | `Ctrl+Tab` / `Ctrl+Shift+Tab` | `Alt+PgDown` / `Alt+PgUp` |
  | Copy \input line | `Ctrl+Shift+C` | `Alt+Shift+C` |

* **Phones and tablets** aren't supported: the editor needs a mouse, a keyboard and room. On a small
  screen the page says so, and opens anyway.

*Help → Download the Desktop Version* leads to the releases; the desktop app's *Help → Open the Website*
leads here. Both have *Help → Source Code on GitHub*.

## Settings

ERDTeX keeps its settings (Preferences, shortcuts, recent files, the defaults of new diagrams, the open
tabs) in `settings.toml` in `~/.config/local.erdtex/` on Linux and `%APPDATA%\local.erdtex\` on
Windows; the website keeps them in the browser. The file types go to the program installed (or chosen)
last.

## Known limitations

- **Panning can be choppy on Linux with NVIDIA graphics,** because of how WebKitGTK draws there.
- **No portable ZIP for Windows,** and the installer is for the current user only.

## The window

* **Tabs:** several diagrams stay open in one window, ER and GSM side by side, each with its own undo
  history and view. Tabs are dragged into another order and closed with × or the middle button; `Ctrl+W`
  closes one, `Ctrl+Tab` and `Ctrl+Shift+Tab` switch between the tabs of a group. A tab's menu (right
  button) closes it, the others or those to its right. Those with a saved file reopen at the next start.
  A file that's already open is just shown, and an unsaved new diagram is never replaced.
* **Tabs side by side:** the editor area holds **groups** of tabs, each with its own tab bar and canvas.
  Drag a tab to the left, right, top or bottom edge of a group to split that side off with the tab in it
  (the part it will take is shaded), onto another group's tab bar to move it there (a line shows where),
  or within its tab bar to reorder it. The same without the mouse: *Split Right* (`Ctrl+Alt+Right`), *Split
  Down* (`Ctrl+Alt+Down`) and *Move Tab to Other Group* in the *View* menu and the tab's menu, and *Next
  Group* / *Previous Group* (`Ctrl+Alt+PgDown` / `Ctrl+Alt+PgUp`). The splitters between groups change
  their sizes; a group closes with its last tab. A diagram is in one tab only, so a group's only tab
  can't split off. **The group last clicked in** decides what the panes, the toolbar, the status bar,
  the preview, the Relational Schema window and the keys work on; New and Open put the diagram there.
  The groups come back at the next start as they were, each showing the tab it showed; a group whose
  files are gone by then is left out, and the others take its space.
* **Panes:** *Tools* (left), *Element* (right), *Problems* (bottom) and *LaTeX Preview* (right, hidden at
  first). Drag a pane by its title to another side, or away from the sides to let it float; the buttons
  in its title fold it, float or dock it, and close it. The *View* menu shows and hides each one
  (*Problems* with the number of problems). Splitters between the panes and next to the canvas change their size. The
  tools reflow to the pane's width. The layout is kept for the next start.
* **Toolbar:** new ER and GSM diagram, open, save, undo, redo, zoom, fit, grid, and *Snap*.
* **The canvas:** the wheel zooms around the pointer, Shift+wheel scrolls, the middle button (or
  Space + drag) pans; *View → Fit Diagram* and *Actual Size*. Rulers show centimetres, the status bar
  the kind of diagram, the pointer's position and the zoom.

## Editing

* **Tools** (the *Tools* pane): elements are placed with a click, connections are drawn by dragging from
  one element to another. Illegal connections are refused (the target turns red and the status bar says
  why), e.g. an entity–entity link in ER or an IS-A edge that doesn't start at a subtype, or would create
  a cycle, in GSM. IS_A, CAN_BE and attribute lists are placed by clicking their entity.
* **Weak relationships**: **Kind** in the panel chooses between *existence dependency (E)*, where the weak
  entity's own key is unique everywhere, and *identification dependency (Id)*, which includes existence
  dependency, where its key is only unique together with its owner's. The panel explains the choice with
  the real names, and it decides how the key is underlined and what the relational schema does.
* **Two weak entities**: if a weak relationship joins two weak entities, one owns the other, and the
  double line leads to the dependent one. Drawing the second link leaves both single and selects the
  relationship, where **Dependent** in the panel sets the direction. It decides the schema, the key's
  underline and the explanation.
* **An entity type depending on itself** (chapters in chapters, with the roles Oberkapitel and Unterkapitel):
  only as an existence dependency (**E**), since an identification dependency would put the key of the
  one above into each key, without end; `Id` gets a warning with a fix that makes it a normal
  relationship. With E, a hint points out that the topmost ones have none above them. In SQL, deleting
  one deletes those below it (`ON DELETE CASCADE`).
* **Double lines**: a link between a weak relationship (weak, E or Id) and a weak entity is double-lined
  automatically when it is created; older single ones get a warning with a one-click fix, double lines
  anywhere else are an error (also with a fix).
* **Subdomains**: drag the squares to move corners, the circles in the middle of each edge to add a
  corner (rectangles become polygons); double-click a corner to remove it.
* **Select tool**: click, Shift+click or a rubber band; drag to move (snapping to the grid, Alt: free);
  the arrow keys nudge (Shift: 1 cm, Alt: 0.05 cm). Selected edges show handles: the round one bends the
  edge, squares move way points or reconnect the ends; double-click an edge to add a way point, a way
  point to remove it. Attribute lists are dragged around their node to change the side. A right click
  opens the context menu.
* **Label fields** (also a note's text) grow with the text, up to twelve lines. The handle below a field
  sizes it by hand; a double click on it fits the field to the text again.
* **The Element pane**: label, id, position, colours and every option of the selected element; with
  nothing selected, the diagram's settings (stored as `\setup{...}`). Its title names the element in
  full ("Entity type", "Representable type", "Single-valued attribute"…).
* **All attributes of an entity type** (ER), or of an abstract type or subtype (GSM), are listed at the
  bottom of its Element pane: its own first, then the ones it inherits from its supertypes (through IS_A
  and CAN_BE, or generalization and specialization), nearest first, in italics with where they come from
  (↑ Person). Keys are underlined as in the drawing (a weak entity's dashed), multi-valued ones in braces,
  derived ones with a slash; in GSM the type each attribute line leads to is shown too. Hovering over an
  inherited attribute shows every chain it is inherited along (`Pilot → Employee (IS_A) → Person
  (IS_A)`). A double click (or Enter) opens the attribute where it is defined: its attribute list with
  the cursor in its name, the Chen attribute, or the GSM attribute line. The *Source* tab shows the
  element's LaTeX. *Add attribute list* (also `Ctrl+L`, and in the *Edit* menu) gives an entity, a
  relationship or a PART_OF a list, with the cursor in its first name.
* Undo and redo, cut, copy and paste (as diagram source, also between diagrams), duplicate, delete with
  dependent elements.
* **While typing in a panel**: after placing an element, the cursor is in its label field, so you can
  name it right away. There, `Ctrl+Z` first undoes your typing and then goes on in the diagram, and `Esc`
  keeps what was typed and returns to the select tool, as it does everywhere.
* Nodes that use relative positions (`right=of x`) get absolute coordinates the first time they are
  moved; everything else in the file (comments, formatting, unknown TikZ) is kept.

**Copy and paste:** copied elements paste into any diagram of the same kind, also in another tab or in
another ERDTeX window. They land where the mouse is, or in the middle of the view if it's
outside. An ER diagram's elements can't go into a GSM diagram, or the other way round, and the app says
so. Connections pasted without one of their ends are left out, and the status bar says how many.

## Arranging

**Edit → Arrange Diagram…** tidies a diagram up for you. First, what is arranged:

* **The whole diagram.**
* **Only the selected elements** (also *Edit → Arrange Selection…*): nothing else moves, and the rest
  of the file stays exactly as it is. A selected attribute list stands for its element, a selected
  subdomain box for everything inside it.
* **Everything but the selected elements:** they stay where they are (pinned), and the rest is arranged
  around them.

Then how:

* **Tidy up** (the default) keeps your layout recognisable. Elements that overlap are moved apart, as
  little as possible. Each attribute list may be turned to another side of its element, away from the
  lines. A relationship may move between its entities, and elements may swap places or move to a free
  spot nearby, within their box, where that removes crossings.
* **Lay out anew** also tries several fresh layouts (supertypes above their IS_A and subtypes), and takes
  one of them if it is better.

Either way, a line that would run through an element is led around it with way points, each its own
way: two lines are never laid on top of one another, or side by side so closely that they can't be told
apart, and no line bends on another. Your own way points and bends stay where they still fit.
Generalisations keep running top to bottom where they did.

Subdomain boxes keep their members: an element never leaves its box or ends up in another, and the
boxes become rectangles around their members, the title at the top left. A relationship (or IS_A,
PART_OF, CAN_BE) that joins elements of different boxes, or of a box and none, belongs to no box: it is
placed between its elements, outside the boxes, so that its lines don't cut through another box's
contents. Boxes don't overlap one another afterwards, and a box without elements moves out of the way
like an element. A box holding pinned elements stays where it is and doesn't shrink.

Every attempt is measured on the drawing itself: overlapping elements (an attribute list counts as an
element) and boxes, lines on top of one another, lines through elements, crossing lines, the length of
the lines and the size. The best attempt is taken only if it is better than the diagram as it is;
otherwise the status bar says so and nothing changes. When it is taken, a bar above the status bar shows
the figures before and after, with **Keep** and **Undo**. Arranging is one step: `Ctrl+Z` takes all of
it back. It takes well under a second for most diagrams and a few seconds for large ones (the window
stays responsive meanwhile).

The **gap between elements** (0.5 cm by default) is set in the dialog and in *Preferences > Editor*. The
commands have no keys by default; they can be given in *Preferences > Keyboard shortcuts*.

## Files

* *File → Open* starts in the folder of the diagram shown, or where the last diagram was opened or saved;
  it takes several files at once, each in a tab of its own. *Save As* offers only the diagram's own kind.
* *File → Open*, *Open Recent*, dropping files on the window and the command line take `.erd` and `.gsmd`
  files, and other text files with diagram commands inside. Anything else is refused, because it would
  look like an empty diagram and Save would write to it. *Open Recent* lists diagrams only.
* *File → Open Example* opens a sample diagram, ER or GSM (a music festival), that shows most of what the
  program and the packages can draw. It opens in a tab of its own and isn't saved anywhere; unchanged, it
  closes without asking, and *Save As* keeps your changes.
* A file is reloaded automatically when another program changes it (after asking, if there are unsaved
  changes); *File → Reload from Disk* (F5) does so on request.
* *File → Copy \input Line* (Ctrl+Shift+C) copies the line that includes the diagram in a LaTeX
  document, such as `\inputerdiagram[max width=\linewidth]{festival.erd}`.
* A GSM diagram is never saved over an ER file, or the other way round.

## LaTeX preview and export

* **LaTeX Preview** (F6, or the *View* menu): the current state of the diagram, including unsaved
  changes, is compiled with `pdflatex` in the background a moment after each change and shown as PDF.
  LaTeX errors are listed with their line in the diagram file; the last good result stays visible. The
  pane has *Auto-update*, the package's *Grid*, *Update*, and zoom; in the page, the wheel zooms, dragging
  moves, a double click fits. The app uses the packages built into it, so the TeX installation's own
  versions don't matter. **Preferences > Output** sets the preamble (e.g. `\usepackage{kpfonts}` to match
  a document's font). Switching tabs compiles the diagram of the new tab.
* **Export** (*File → Export…*, Ctrl+E): PNG, PDF or SVG, either of the **LaTeX result** (exactly as in
  the document) or of the **editor rendering** (as on the canvas, without selection or problem marks).
  The coordinate grid can be included (off by default); PNG exports have a resolution (36 to 2400 dpi)
  and an optional transparent background. PDF and SVG exports are vector graphics. The dialog starts
  where this diagram's last export went, the first time next to the diagram (the same for saving the
  relational schema).

## Settings

**Settings > Preferences** (Ctrl+,) holds all settings of the app:

* **Appearance:** the display mode (below), and the labels: `IST_EIN*E`, `KANN_SEIN`, `TEIL_VON` are a
  setting of the app, used on the canvas, in the preview and for export. The file doesn't change. In
  your own LaTeX document, its language decides.
* **Keyboard shortcuts:** see below.
* **New ER diagrams, New GSM diagrams:** what new diagrams start with: the main choices (font, IS_A
  shape, notation, ...) and every style key of the packages (colours, fonts, sizes). Existing
  diagrams keep what they have; **Use for the open diagram** copies the values into the open one, which
  can be undone.
* **Editor:** snapping, grid, hints, the gap between elements when arranging. **Output:** the LaTeX preamble, and what Export and Relational
  Schema start with.

Each diagram keeps its own settings in its file, so they also apply in LaTeX. The Element pane, with
nothing selected, changes them for the open diagram.

### Display modes

| Mode | Menus, panels, dialogs | Canvas |
|---|---|---|
| System (default) | as the desktop | as the desktop |
| Light | light | light |
| Dark | dark | dark |
| Intermediate | dark | light |

In System mode, a KDE Plasma colour scheme is taken over (also when it changes); elsewhere the app's own
light or dark palette follows the system's light or dark setting. On a dark canvas, diagrams are recoloured on
screen only: black becomes light, white and light greys become dark, colours you chose stay. Files,
LaTeX and exports don't change. The LaTeX preview remains a white page, since it shows the real output.

### Keyboard shortcuts

All tools have one, with a separate set for ER and GSM; the set follows the diagram being edited.
Hovering shows the shortcut, and **Help > Mouse and Keyboard** (F1) lists the current ones. All can be
changed under Preferences, and only combinations that aren't in use are accepted. General commands
such as Save and Undo have one shortcut for both kinds. A shortcut also works while a panel field has
the keyboard focus (except the keys a text field needs itself, such as `Ctrl+A`, `C`, `X`, `V`, and
`Ctrl+Backspace` / `Ctrl+Delete` to delete a word, `Ctrl+Left` / `Ctrl+Right` to move by one). Where
the keyboard layout puts another character on Shift+digit (on a German one, Shift+7 is `/`),
`Ctrl+Shift+digit` goes by the physical key.

| Keys | ER | GSM |
|---|---|---|
| Ctrl+1 | Select | Select |
| Ctrl+2 | Line | Attribute line |
| Ctrl+3 | Entity | Abstract type |
| Ctrl+4 | Relationship | Representable type |
| Ctrl+5 | Attribute list | Subtype |
| Ctrl+6 | IS_A | Aggregation |
| Ctrl+7 | Weak entity | Grouping |
| Ctrl+8 | Id dependency | Generalization |
| Ctrl+9 | E dependency | Specialization |
| Ctrl+Shift+1 | Weak relationship | Multi-valued attribute line |
| Ctrl+Shift+2 | CAN_BE | Component line |
| Ctrl+Shift+3 | PART_OF | Plain line |
| Ctrl+Shift+4 | Chen attribute | Note |
| Ctrl+Shift+5 | Note | Subdomain |
| Ctrl+Shift+6 | Subdomain | Junction point |
| Ctrl+Shift+7 | Junction point | |

General ones, for both kinds, include `Ctrl+N` / `Ctrl+Shift+N` (new ER / GSM diagram), `Ctrl+O`, `Ctrl+S`,
`Ctrl+W`, `Ctrl+Tab`, `Ctrl+Alt+Right` / `Ctrl+Alt+Down` (split right / down), `Ctrl+Alt+PgDown` /
`Ctrl+Alt+PgUp` (next / previous group), `F5` (reload), `F6` (preview), `Ctrl+E` (export), `F1` (mouse and
keyboard help), `Shift+F1` (rule books); **Help > Mouse and Keyboard** lists all.

**Help > Save LaTeX Packages…** saves the LaTeX packages of this version, `erdiagram.sty` and
`gsmdiagram.sty`, with their manuals (PDF) into a folder you choose, for example next to a document;
files already there are replaced only if you agree. They are built into the program, so they always match
the diagrams it writes. **Help > About** shows the program's version and the built-in packages' versions;
its **Licences…** shows the licences: the program's (MIT), the LaTeX packages' (LPPL 1.3c), the fonts' (GUST
Font License), and those of every library built into the program.

## Attributes (ER diagrams)

* The panel of an attribute list edits its rows. **Shift+Enter** while typing a name goes on to a new
  attribute below, so a list can be typed without the mouse.
* **Part of the above** makes an attribute a part of the one above it, **Not a part** undoes that.
  **Kind** makes the selected one multi-valued (`{email}`, stored in its own relation) or derived
  (`/age`, not stored). A derived attribute with a **formula** is computed by a view in the SQL script.
  The formula is SQL as you write it, for all databases alike: with "quote all names" it has to quote
  names itself.
* **Uniq.** marks a further candidate key (`+email` in the file). It isn't drawn, and becomes `UNIQUE` in
  SQL, where NULL stays allowed. A composite attribute counts as one key, with all its parts.
* The key of a weak entity type is dashed by itself (it is unique only together with its owner's key),
  unless it depends on E alone.
* **Attributes: ovals (Chen)**, in the diagram's settings, draws all lists as ovals, placed
  automatically. Dragging one stores its position, and those of the others in its list so that they
  stay put; **Place the ovals automatically** starts over.
* **View > Show Data Types** and the diagram's settings show or hide the data types.

## Generalisations (ER diagrams)

Selecting an IS_A or CAN_BE shows two choices, **Coverage** (complete or incomplete, also called
total or partial) and below it **Overlap** (disjoint or overlapping), in the order of the letters the
file keeps (`cd`, `co`, `id`, `io`). Each choice says what it means with the real names, the overlap
going on from the coverage ("so each Person is exactly one of: Student, Employee" for complete and
disjoint), and the relational schema uses them. If either answer is missing, the Problems pane warns.
A supertype above a complete IS_A needs no key of its own when every subtype has one (each of its
entities is in one of them): its own, one inherited from another supertype, or again through a complete
IS_A below it; otherwise it gets the hint that it has no key attribute. Free text such as `vollständig, disjunkt` is
understood, and replaced by the letters once you choose.

The diagram's settings choose the **shape** (hexagon or diamond; PART_OF stays a diamond) and whether
the type is shown as **letters** inside the shape or **words** beside it.

**Several supertypes:** a subtype can belong to several generalisations, and its relation then refers to
all its supertypes. It takes its key from the first one. A second supertype above the same root has the
same key, so that column gets a second foreign key. With different roots, the other key becomes a
further column that is `UNIQUE` and `NOT NULL`, since one tutor is one employee. The options that merge
relations (2), (3) and (4) aren't offered for such a generalisation, because the subtype couldn't be
merged into two relations. A diagram that asks for one gets a warning, and the default.

## Cardinalities (ER diagrams)

Each diagram has a notation, chosen in its settings: **n-notation** (`1` exactly one, `c` at most one,
`n` one or more, `cn` any number; new diagrams) or **min-max** (`(0,*)`, `(1,1)`, ...). The two count
from opposite ends (rule book NOT-1).

* Selecting a link shows a **picker** with the choices of the notation in use, and one sentence saying
  what the value means with the real names ("Each Hund has exactly one Halter"). Min-max has a min field
  (`·` for unknown) and a max field; `max ≥ min` is enforced.
* **Switching a diagram's notation** converts all binary relationships exactly. Values without an
  equivalent (`(3,*)`) and n-ary relationships, where the notations say different things, are listed and
  stay as they are.
* One relationship can **deviate** from the diagram's notation (discouraged, and marked with a hint with
  a fix). This applies to all its edges.
* **Crow's foot** symbols can replace the labels in n-notation, in the app and in LaTeX: `1` is two bars,
  `c` a circle and a bar, `n` a bar and a fork, `cn` a circle and a fork. Only the labels are replaced, so
  diamonds and attributes stay and n-ary relationships keep working. The edge meets the entity's side at
  a right angle, ending in a short straight piece that carries the symbol.
* **Older labels** (`m`, `mc`, `[a,b]`) keep working. A hint offers to rewrite them; "Fix all" does so for
  the whole diagram. Invalid labels typed into the Source tab are reported.

## Subdomain boxes

A box groups what's inside it: an element belongs to it if its centre is inside, and to the innermost
box when they are nested. Moving a box moves its contents, including boxes inside it; moving an element
out ends its membership. The panel lists what a box contains.

The relational schema is grouped under the boxes (`Outer / Inner:` when nested, then `Outside of any
box:`), in plain text, LaTeX and Markdown. SQL marks each table with a comment (`-- box: Outer`), because the order
of the `CREATE TABLE` statements has to stay safe for foreign keys. No real database schemas are created.

## Relational schema and SQL (ER diagrams)

*File → Relational Schema / SQL* (Ctrl+Shift+E) transforms the ER diagram into relations (entity types,
1:n, n:n, 1:c, 1:1, c:c, recursive, n-ary, weak entities with E or Id dependency, aggregation, composite,
multi-valued and derived attributes, generalisation options (1) to (4)). The window stays open next to
the diagram, doesn't block editing, and follows every change.

* **Defaults**: each decision starts with a default and offers all its options; a choice is stored on
  its element. Generalisations start categorised (a categorisation attribute for cd and id, a relation for
  co and io, none if the type isn't given), recursive 1:n relationships as an own relation, n-ary
  relationships with several `1` sides keyed by a candidate key. Versions up to 3.5 offered a second rule
  set; a file with `\setup{rm rules=db1}` from then keeps its results (generalisations not categorised,
  recursive relationships as a foreign key), though nothing offers it any more (rule book REL-22).

* **Cardinalities**: `(min,max)` counts at the entity it is written at; everything else
  (`1`, `c`, `n`, `cn`, `[a,b]`, `1..*`) counts from the opposite end. Key attributes on a relationship
  type join its key, and force an own relation. A role on an edge names the foreign key column. An
  unknown minimum (`·`) is treated as 0, with a note. Bounds that foreign keys can't enforce are listed
  in the Problems pane and at the end of the SQL.
* **Decisions**: where the rules allow alternatives (generalisations, 1:1 and c:c relationships,
  recursive relationships, n-ary relationships with several single sides; collapsed: an own relation for
  1:n and 1:c) every option is listed with an explanation and *the relations it produces*, so the options
  can be compared side by side. Options that the constraint (cd, id, co, io) rules out are greyed with
  the reason. A choice is stored in the diagram (`rm=...`, see the erdiagram manual, "Relational model
  options"); the default writes nothing.
* **Data types**: stored in the diagram as `name : type` in attribute lists
  (`\attributes{p}{*PID : integer, born : date}`) or `type=` on ellipses. Missing types are guessed from
  the name (shown in italics; *Store guessed types* writes them). The Element pane has a type column for
  attribute lists and a data type field for ellipses. *Show data types in the diagram* prints them
  (`\setup{show types}`).
* **Output**: plain text (`Person : {(_PID_ : integer, name : string)}`, since a relation is a set of
  tuples; `_key_`, `/foreign key/`, `_/both/_`, or `(PK)`/`(FK)` tags) with foreign keys, alternate keys
  and allowed values listed below; LaTeX in the `relationalschema` environment of `erdiagram.sty`, which
  needs no other package and no macros in the preamble (the window says so, with a link to *Save LaTeX
  Packages…*, since the package isn't part of TeX Live or MiKTeX); **Markdown** with the same parts as
  headings and lists, where primary keys are **bold** (Markdown can't underline: a legend at the top and a
  note in the window say so), foreign keys *italic*, both ***bold italic***, and Markdown's special
  characters escaped (`floor\_count`). All three keep names as drawn, without spaces (`Raum-Id`,
  `dateOfBirth`). SQL uses sanitised names (`raumId`), for **PostgreSQL**, **Oracle** and
  **SQLite** (primary, foreign, unique and check constraints, `ON DELETE CASCADE` for existence-dependent
  tables, `DEFERRABLE` foreign keys for tables that refer to each other, reserved words quoted, optional
  `DROP TABLE` statements). Relationship relations are named `Entity_rel_Entity` or after the
  relationship; foreign keys keep the key name (qualified on clashes, e.g. `departureAPID` /
  `arrivalAPID`) or are always qualified. An entity's relation name can be set in the Element pane.

The generated SQL was tested by executing it in SQLite, PostgreSQL and Oracle Free, for many examples with
all their options.

## GSM → ER conversion

*File → Convert to ER Diagram…* (GSM diagrams) builds an ER diagram from the GSM diagram, starting with
the values of **Preferences > New ER diagrams** like any new diagram (in min-max, if that's your
default). **The GSM diagram is only read**: the result opens as a new, unsaved diagram in a new tab
(suggested name `<name>_er.erd`).

Mapping: abstract types and subtypes → entities; single-valued attributes to representable types →
attributes; references between object types → relationships (target side 1, source side cn);
multi-valued attributes and groupings → entity + n:n relationship (or a multi-valued attribute);
aggregations of value types → entities (Address) or composite attributes, of object types →
relationships (Job); attributes of a junction of several objects → attributes of their relationship;
generalization → IS_A cd, specialization → IS_A io; derived attributes → derived attributes (`/age`).
GSM has no keys and no cardinalities: identifier-like attributes (has Id, Person Id, orderNo) become
keys, otherwise `<Name> ID` is added; minimum cardinalities are 0.

The dialog lists the choices where several mappings are sensible (inverse attribute pairs such as
*lives in* / *home of*, multi-valued value types, aggregations) with a live preview of the resulting ER
diagram, plus every assumption made. The rules of the conversion are in the rule books (GSM → ER).

## Rule books

**Help → Rule Books…** (`Shift+F1`) describes every rule built into the program, one page per rule, in six books:

| Book | Ids | What it describes |
|---|---|---|
| Modelling ER | `ER-1` … `ER-34` | what makes an ER diagram correct: the errors, warnings and hints of the *Problems* pane, with the fixes offered |
| Modelling GSM | `GSM-1` … `GSM-12` | the same for GSM diagrams |
| Drawing | `DRAW-1` … `DRAW-11` | the lines the editor refuses to draw |
| GSM → ER | `G2E-1` … `G2E-13` | how *Convert GSM → ER* turns each element into ER, the questions it asks and their defaults |
| ER → Relational | `REL-1` … `REL-22` | how the Relational Schema window turns an ER diagram into relations, its decisions and defaults |
| Notations | `NOT-1` … `NOT-6` | cardinality notations, crow's foot symbols, generalisation letters, the symbols of ER and GSM diagrams, and the terms the books use |

Each page has the same parts: the rule in one sentence (**must**: an error, **should**: a warning or a
hint, **may**: allowed, **must not**: refused while drawing, **becomes**: a conversion), what it applies
to (with the palette's symbols), what the program does or the conditions and choices of a conversion,
why, and examples drawn as on the canvas (✓ follows, ✗ breaks the rule; for a conversion, what is given
and what it becomes). The books in the list open with a click on their title; a search finds rules by
id, title or text; back and forward go through the pages seen. The window doesn't block the main one.

The program links to the rules wherever it applies one:

* every problem in the *Problems* pane names its rule (`ER-1`): a click on it, or its context menu,
  opens the rule's page;
* a line refused while drawing names its rule in the status bar (*"Not allowed: two entities need a
  relationship in between (DRAW-1)"*), as a link;
* each question of *Convert GSM → ER* and each decision of the Relational Schema window shows its
  rule's id beside its title, as a link.

## Rule checks

Every change is checked against the modelling rules. Problems appear in the *Problems* pane (click to
jump to the element), as coloured marks on the canvas and as tooltips:

* **errors** (red): rule violations: entities or relationships linked directly, generalisations with
  non-entity members or supertypes, double lines anywhere but between a weak relationship and a weak
  entity, unknown or duplicate ids; in GSM diagrams IS-A edges that don't start at a subtype or end at a
  subtype or abstract type, IS-A cycles, IS-A paths with different roots, and components that don't start
  at an aggregation or grouping;
* **warnings** (orange): incomplete or doubtful parts: relationships with fewer than two links,
  generalisations without subtypes or without saying their type, weak entities without a weak (E/Id)
  relationship or with more than one owner, a missing double line, two weak entities without a single
  double line saying which one depends on the other, recursive relationships without roles that tell the
  edges apart, attributes belonging to an attribute that isn't composite, derived or multi-valued
  attributes marked as candidate keys, invalid cardinalities, relationships mixing the notations;
  groupings without exactly one component, aggregations with fewer than two, subtypes without IS-A edge,
  cardinalities on edges without an entity; an entity type identification dependent on itself, or depending
  on itself where no topmost one is possible;
* **hints** (blue, can be hidden): entities without a key attribute (a supertype above a complete IS_A
  whose subtypes all have keys doesn't need one), entity–relationship edges without
  cardinality, older labels, a relationship deviating from the diagram's notation, TikZ code the editor
  can't show, the topmost ones of an entity type that depends on itself (they depend on none).

Where there is an obvious fix, the **Fix** button applies it; **Fix all** applies every one. The checks
include everything the LaTeX packages warn about.

## File handling guarantees

* Loading and saving an unmodified file reproduces it byte for byte (comments, spacing, line endings).
* Only modified elements are rewritten, in canonical one-line form.
* Unknown lines (plain TikZ etc.) are kept verbatim and are read-only.
* Deleting an element removes its line (and its end-of-line comment) but keeps comment lines above it.

## Building and checking

```sh
npm ci                                  # once, in this folder
npx tauri dev                           # start it from source
../tools/check_rust.sh                  # every check: formatting, lints, tests, the comparison, the window
../tools/build_rust_release.sh          # the deb, rpm and Windows installer, into ../dist/
../tools/test_rust_release.sh           # each installed in a clean system, then its self-test
```

Needs Rust (rustup), Node.js 22, wasm-pack and WebKitGTK's development packages; the release builds run
in a container (podman). See [`ARCHITECTURE.md`](ARCHITECTURE.md) for how it is built and how to extend
it, and the root `README.md` for the release checklist.
