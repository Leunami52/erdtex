// The window's state: tabs, each with its diagram and view, and what the menus do.  The
// behaviour follows the Python app's main window (mainwindow.py): an untouched new diagram
// gives way to a file being opened, there is always a tab, a file open in one tab is shown
// rather than opened twice, and unsaved changes are asked about.
import { tick } from "svelte";
import { Diagram, input_line } from "./core";
import * as be from "./backend";
import { baseName, dirName, EXTENSIONS, type Kind, type Opened } from "./backend";
import { Docks } from "./docks.svelte";
import { keepLayout, type Groups, type SavedLayout } from "./groups";
import type { MenuEntry } from "../components/ContextMenu.svelte";
import type { Arrangement, Graph, Options as ArrangeOptions } from "./arrange";

/** The quality figures of a diagram (see `arrange::Figures`). */
export interface Figures {
  overlaps: number;
  through: number;
  crossings: number;
  /** pairs of lines on top of one another (or touching at a bend), and how long they run so */
  close: number;
  close_length: number;
  length: number;
  gap: number;
  unevenness: number;
  width: number;
  height: number;
  moved_out: number;
  score: number;
}
import type { Bindings } from "./keys";
import { applyPanels, asMode, isDarkPalette, MODES, resolve, type Mode } from "./theme";
import { CM, centreOn, fit, PX_PER_PT, toDiagram, zoomOf, zoomTo, type Box, type View } from "./view";

export const APP_NAME = "ERDTeX";

let nextId = 1;

/** One open diagram, with its own view and undo history. */
export class Tab {
  readonly id = nextId++;
  diagram: Diagram;
  /** the file, absolute; null for a new diagram */
  path = $state<string | null>(null);
  /** a converted diagram's name until it's saved */
  suggested = $state<string | null>(null);
  /** a sample diagram (File → Open Example): unchanged, it closes without asking */
  example = $state(false);
  /** counts changes of the document, so that what depends on it is computed again */
  version = $state(0);
  /** counts changes of the selection, the tool and what's dragged (the overlay) */
  ui = $state(0);
  /** counts changes of the selection (the properties panel) */
  sel = $state(0);
  /** counts requests to put the cursor in the label field (after placing, a double click) */
  labelRequest = $state(0);
  /** the pointer over the canvas (screen pixels in it), or null when it's outside */
  pointer: [number, number] | null = null;
  view = $state<View>({ k: PX_PER_PT, tx: 0, ty: 0 });
  /** what the canvas does once it knows its size */
  pending = $state<"fit" | "origin" | null>("origin");
  /** the canvas's size in pixels */
  size = { w: 0, h: 0 };
  /** the file's change time and size when last read or written (to notice other programs) */
  stamp: [number, number] | null = null;

  constructor(diagram: Diagram) {
    this.diagram = diagram;
  }

  changed(): void {
    this.version++;
    this.ui++;
  }

  replace(diagram: Diagram): void {
    // a new document starts with the select tool, as in the Python app
    this.diagram.free();
    this.diagram = diagram;
    this.sel++;
    this.changed();
  }

  get tool(): string {
    void this.ui;
    return this.diagram.tool();
  }

  get selected(): number {
    void this.ui;
    return this.diagram.selection_count();
  }

  get kind(): Kind {
    void this.version;
    return this.diagram.kind() as Kind;
  }

  /** a converted diagram that was never saved counts as modified */
  get modified(): boolean {
    void this.version;
    return !this.diagram.is_clean() || (this.path === null && this.suggested !== null && !this.example);
  }

  get name(): string {
    if (this.path) return baseName(this.path);
    if (this.suggested) return baseName(this.suggested);
    return "untitled" + EXTENSIONS[this.kind];
  }

  /** an untouched new diagram: opening a file takes its place */
  get pristine(): boolean {
    void this.version;
    return this.path === null && this.suggested === null && this.diagram.is_clean() && this.diagram.is_blank();
  }

  bounds(): Box | null {
    const b = this.diagram.bounds();
    return b.length === 4 ? [b[0], b[1], b[2], b[3]] : null;
  }

  zoomAround(zoom: number, sx = this.size.w / 2, sy = this.size.h / 2): void {
    this.view = zoomTo(this.view, zoom, sx, sy);
  }

  fit(): void {
    this.view = fit(this.bounds(), this.size.w, this.size.h);
  }

  origin(): void {
    this.view = centreOn({ k: PX_PER_PT, tx: 0, ty: 0 }, 0, 0, this.size.w, this.size.h);
  }
}

export interface Question {
  text: string;
  buttons: string[];
  answer: (button: string) => void;
  title?: string;
}

/** What a change in the properties panel did (see the core's `panel::Outcome`). */
export type Outcome =
  | { kind: "done" }
  | { kind: "row"; row: number; edit: boolean }
  | { kind: "ask"; title: string; text: string }
  | { kind: "refused"; text: string }
  | { kind: "inform"; title: string; text: string }
  | { kind: "invalid"; text: string };

export class App {
  tabs = $state<Tab[]>([]);
  currentId = $state(0);
  settings = $state<Record<string, string>>({});
  status = $state("");
  question = $state<Question | null>(null);
  systemDark = $state(false);
  /** the desktop's palette (KDE colour scheme), used in "System" mode */
  desktopPalette = $state<Record<string, string> | null>(null);
  spaceDown = $state(false);
  cursor = $state("");
  bindings = $state<Bindings | null>(null);
  /** a dialog of the window is open (Preferences, …): it keeps the keys */
  dialog = $state<string | null>(null);
  /** a short text page (Help → Privacy, Imprint) */
  textPage = $state<{ title: string; markdown: string } | null>(null);
  /** the Relational Schema / SQL window is open (it doesn't block the window) */
  relational = $state(false);
  /** for tests: the path the next save dialog answers with */
  nextSavePath: string | null = null;
  /** Open Recent: diagrams only, the newest first */
  recent = $state<string[]>([]);
  /** puts the keyboard back on the canvas (set by the current tab's canvas) */
  focusCanvas: () => void = () => {};
  /** the groups of tabs side by side (set by the page's Groups) */
  groups: Groups | null = null;
  /** bumped when the groups change, for what depends on them (the menus) */
  groupsVersion = $state(0);
  /** the context menu shown (canvas, tab, Problems): drawn by the window, outside dockview, whose
   * tab bars clip what they contain (overflow and a transform) */
  menu = $state<{ x: number; y: number; items: MenuEntry[] } | null>(null);
  private statusTimer: ReturnType<typeof setTimeout> | undefined;

  get current(): Tab {
    return this.tabs.find((t) => t.id === this.currentId) ?? this.tabs[0];
  }

  get mode(): Mode {
    return asMode(this.settings["display mode"]);
  }

  /** "System" mode follows the desktop's palette where there is one */
  private get systemIsDark(): boolean {
    return this.desktopPalette ? isDarkPalette(this.desktopPalette) : this.systemDark;
  }

  get panelsDark(): boolean {
    return resolve(this.mode, this.systemIsDark)[0];
  }

  get canvasDark(): boolean {
    return resolve(this.mode, this.systemIsDark)[1];
  }

  get showGrid(): boolean {
    return this.settings["show grid"] !== "false";
  }

  get language(): string {
    return this.settings["label language"] ?? "english";
  }

  /** the panes: Tools, Element, LaTeX Preview, Problems */
  docks = new Docks((text) => void this.setSetting("dock layout", text));

  /** shown with its contents (it compiles only then); hidden at first, as in the Python app */
  get showPreview(): boolean {
    return this.docks.shown("preview");
  }

  /** the LaTeX preview's state, for tests (set by the preview) */
  previewSnapshot: (() => unknown) | null = null;

  get preamble(): string {
    return this.settings["preview preamble"] ?? "\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}";
  }

  get showHints(): boolean {
    return this.settings["show hints"] !== "false";
  }

  /** grid snapping in cm, for all tabs */
  get snap(): number {
    const v = Number(this.settings["snap"] ?? 0.25);
    return Number.isFinite(v) && v >= 0 ? v : 0.25;
  }

  // -- start and end ----------------------------------------------------------------

  async start(): Promise<void> {
    this.settings = await be.getSettings();
    this.docks.load(this.settings["dock layout"]);
    if (be.IS_WEB && this.docks.pane("preview").visible) this.docks.set("preview", { visible: false }); // no pdflatex
    this.systemDark = await be.watchTheme((dark) => (this.systemDark = dark));
    this.desktopPalette = await be.desktopPalette();
    $effect.root(() => {
      $effect(() => applyPanels(this.panelsDark, this.mode === "system" ? this.desktopPalette : null));
      $effect(() => {
        const t = this.current;
        if (t) be.setTitle(`${t.modified ? "*" : ""}${t.name} — ${APP_NAME}`);
      });
    });
    this.select(this.addTab(this.fresh("er")));
    if (be.IS_WEB) {
      await this.restoreWebTabs();
      this.keepWebTabs();
    } else {
      await this.loadRecent();
      await this.restoreTabs();
      for (const p of await be.startupPaths()) await this.openPath(p);
    }
    await be.onClose(
      () => this.closeAll(),
      () => this.tabs.some((t) => t.modified),
    );
    await be.onDrop(async (paths) => {
      for (const p of paths) await this.openPath(p);
    });
    if (!be.IS_WEB) this.watchTimer = setInterval(() => void this.watch(), 1500);
    await be.startOffline(
      async () => (await this.ask("A new version of ERDTeX is available. Reload now? Unsaved diagrams are kept.", ["Reload", "Later"])) === "Reload",
      () => this.message("ERDTeX is now kept in this browser: it also works offline.", 6000),
    );
    if (be.smallScreen()) {
      void this.alert("ERDTeX is made for a mouse, a keyboard and a larger screen: on this device some parts won't fit or work as they should.");
    }
  }

  // -- the website: the open diagrams kept in the browser ---------------------------------

  private static readonly WEB_TABS = "erdtex/tabs";
  private static readonly WEB_VISITED = "erdtex/visited";

  /** The diagrams that were open when the page was last left, as unsaved tabs (a browser can't
   * reopen the files themselves); on the very first visit, the two samples. */
  private async restoreWebTabs(): Promise<void> {
    let kept: { tabs: { name: string; kind: Kind; text: string; example: boolean }[]; active: number } | null = null;
    try {
      kept = JSON.parse(localStorage.getItem(App.WEB_TABS) ?? "null");
    } catch {
      kept = null;
    }
    if (kept && Array.isArray(kept.tabs) && kept.tabs.length) {
      for (const k of kept.tabs) {
        try {
          const d = new Diagram(k.text, k.kind);
          this.prepare(d);
          const tab = this.target();
          tab.replace(d);
          tab.path = null;
          tab.suggested = k.name;
          tab.example = k.example;
          tab.pending = "fit";
        } catch {
          // a diagram that can't be read any more is left out
        }
      }
      const t = this.tabs[kept.active];
      if (t) this.select(t);
      return;
    }
    let visited = false;
    try {
      visited = localStorage.getItem(App.WEB_VISITED) !== null;
      localStorage.setItem(App.WEB_VISITED, "1");
    } catch {
      visited = true;
    }
    if (!visited) {
      await this.openExample("er");
      await this.openExample("gsm");
      this.select(this.tabs.find((t) => t.kind === "er" && t.example) ?? this.current);
    }
  }

  /** Every change of the open diagrams is kept in the browser (after a short pause). */
  private keepWebTabs(): void {
    let timer: ReturnType<typeof setTimeout> | undefined;
    $effect.root(() => {
      $effect(() => {
        const tabs = this.tabs.filter((t) => !t.pristine).map((t) => {
          void t.version;
          return { name: t.name, kind: t.kind, text: t.diagram.text(), example: t.example && !t.modified };
        });
        const active = Math.max(0, this.tabs.filter((t) => !t.pristine).indexOf(this.current));
        clearTimeout(timer);
        timer = setTimeout(() => {
          try {
            localStorage.setItem(App.WEB_TABS, JSON.stringify({ tabs, active }));
          } catch {
            // full or not allowed: kept until the page is closed
          }
        }, 500);
      });
    });
  }

  // -- recent files and files changed by other programs ---------------------------------

  private async loadRecent(): Promise<void> {
    let paths: string[] = [];
    try {
      paths = JSON.parse(this.settings["recent"] ?? "[]");
    } catch {
      paths = [];
    }
    // what isn't a diagram (any more) goes; a missing diagram stays until it is opened
    this.recent = await be.diagramPaths(paths.filter((p) => typeof p === "string"));
  }

  async addRecent(path: string): Promise<void> {
    if (be.IS_WEB || (await be.diagramPaths([path])).length === 0) return;
    this.recent = [path, ...this.recent.filter((p) => p !== path)].slice(0, 10);
    await this.setSetting("recent", JSON.stringify(this.recent));
  }

  private watchTimer: ReturnType<typeof setInterval> | undefined;
  private watching = false;

  /** A file changed on disk (another program, e.g. a text editor): its tab reloads it; with
   * unsaved changes, after asking. */
  async watch(): Promise<void> {
    if (this.watching || this.question) return;
    this.watching = true;
    try {
      // the desktop's colour scheme changed (KDE's settings)
      const pal = await be.desktopPalette();
      if (JSON.stringify(pal) !== JSON.stringify(this.desktopPalette)) this.desktopPalette = pal;
      for (const tab of [...this.tabs]) {
        if (!tab.path || !this.tabs.includes(tab)) continue;
        const stamp = await be.fileStamp(tab.path);
        if (!stamp) continue; // gone for now (an editor saving by renaming): look again later
        if (tab.stamp && stamp[0] === tab.stamp[0] && stamp[1] === tab.stamp[1]) continue;
        const before = tab.stamp;
        tab.stamp = stamp;
        if (!before) continue;
        await this.fileChanged(tab);
      }
    } finally {
      this.watching = false;
    }
  }

  private async fileChanged(tab: Tab): Promise<void> {
    if (!tab.path) return;
    let opened: Opened;
    try {
      opened = await be.readDiagram(tab.path);
    } catch (e) {
      this.message(`Not reloaded: ${String(e).replace(/\n/g, " ")}`, 8000);
      return;
    }
    if (opened.text === tab.diagram.text()) return; // our own save
    if (tab.modified) {
      this.select(tab);
      const r = await this.ask(`${tab.name} was changed on disk. Discard your changes and reload?`, ["Yes", "No"]);
      if (r !== "Yes") return;
    }
    let d: Diagram;
    try {
      d = new Diagram(opened.text, opened.kind);
    } catch (e) {
      this.message(`Not reloaded, the file has an error: ${e}`, 8000);
      return;
    }
    this.prepare(d);
    tab.replace(d);
    this.message(`Reloaded ${tab.name}`, 3000);
  }

  /** The line that includes the diagram in a LaTeX document, to the clipboard. */
  async copyInputLine(): Promise<void> {
    const t = this.current;
    const line = input_line(t.kind, t.path ? baseName(t.path) : undefined);
    await be.copyText(line);
    this.message(`Copied: ${line}`, 5000);
  }

  togglePreview(): void {
    this.docks.toggle("preview");
  }

  private async restoreTabs(): Promise<void> {
    let paths: string[] = [];
    try {
      paths = JSON.parse(this.settings["open tabs"] ?? "[]");
    } catch {
      paths = [];
    }
    const missing: string[] = [];
    for (const p of paths) {
      if (await be.fileExists(p)) await this.openPath(p, true);
      else missing.push(baseName(p));
    }
    const k = Number(this.settings["active tab"] ?? 0);
    if (k >= 0 && k < this.tabs.length) this.select(this.tabs[k]);
    if (missing.length) this.message("No longer there: " + missing.join(", "), 8000);
  }

  /** When the window closes: every tab may be saved; the open files are kept for next time. */
  async closeAll(): Promise<boolean> {
    for (const t of [...this.tabs]) if (!(await this.maybeSave(t))) return false;
    // the tabs in the order they appear, group by group; and how the groups are laid out
    const order = this.groups?.list().flatMap((g) => g.tabs) ?? [];
    const inOrder = [...this.tabs].sort((a, b) => order.indexOf(a.id) - order.indexOf(b.id));
    const withPath = inOrder.filter((t) => t.path);
    await this.setSetting("open tabs", JSON.stringify(withPath.map((t) => t.path)));
    await this.setSetting("active tab", String(Math.max(0, withPath.indexOf(this.current))));
    const kept = this.groups ? keepLayout(this.groups.layout(), (id) => this.byId(Number(id))?.path ?? null) : null;
    await this.setSetting("tab layout", kept ? JSON.stringify(kept) : null);
    return true;
  }

  /** The layout of the groups kept at the last end (closeAll), or null. */
  keptLayout(): SavedLayout | null {
    try {
      const l = JSON.parse(this.settings["tab layout"] ?? "null") as SavedLayout | null;
      return l && typeof l === "object" && l.grid && typeof l.grid === "object" ? l : null;
    } catch {
      return null;
    }
  }

  async quit(): Promise<void> {
    await be.quit();
  }

  // -- helpers --------------------------------------------------------------------------

  async setSetting(key: string, value: string | null): Promise<void> {
    if (value === null) delete this.settings[key];
    else this.settings[key] = value;
    await be.setSetting(key, value);
  }

  message(text: string, ms = 4000): void {
    this.status = text;
    clearTimeout(this.statusTimer);
    this.statusTimer = setTimeout(() => (this.status = ""), ms);
  }

  ask(text: string, buttons: string[], title?: string): Promise<string> {
    return new Promise((resolve) => {
      this.question = {
        text,
        buttons,
        title,
        answer: (b) => {
          this.question = null;
          resolve(b);
        },
      };
    });
  }

  async alert(text: string): Promise<void> {
    await this.ask(text, ["OK"]);
  }

  /** A new diagram with the defaults kept in the settings. */
  fresh(kind: Kind): Diagram {
    const names = Object.keys(this.settings).filter((k) => k.startsWith("new diagram/"));
    const d = Diagram.fresh(kind, names, names.map((k) => this.settings[k]));
    this.prepare(d);
    return d;
  }

  /** What every diagram takes from the settings. */
  prepare(d: Diagram): void {
    d.set_language(this.language);
    d.set_snap(this.snap);
    d.set_show_hints(this.showHints);
  }

  // -- tabs ---------------------------------------------------------------------------

  addTab(diagram: Diagram): Tab {
    const tab = new Tab(diagram);
    if (this.tabs.length) tab.view = { ...this.current.view };
    this.tabs.push(tab);
    return tab;
  }

  select(tab: Tab): void {
    this.currentId = tab.id;
  }

  tabWithPath(path: string): Tab | undefined {
    return this.tabs.find((t) => t.path === path);
  }

  /** The next or previous tab of the current tab's group. */
  nextTab(step: number): void {
    const ids = this.groups?.siblings(this.currentId) ?? this.tabs.map((t) => t.id);
    const i = ids.indexOf(this.currentId);
    const tab = this.tabs.find((t) => t.id === ids[(i + step + ids.length) % ids.length]);
    if (tab) this.select(tab);
  }

  private byId(id: number | null): Tab | undefined {
    return id === null ? undefined : this.tabs.find((t) => t.id === id);
  }

  /** The tab into a new group at the right of (or below) its group. */
  splitTab(side: "right" | "bottom", tab: Tab = this.current): void {
    if (!this.groups?.split(tab.id, side)) this.message("A group's only tab can't split off: it is already on its own.");
  }

  /** The tab into the next group (with one group: into a new one at the right). */
  moveTabToOtherGroup(tab: Tab = this.current): void {
    if (!this.groups?.moveToOther(tab.id)) this.message("There is no other group, and a group's only tab can't split off.");
  }

  /** The rule books' window: whether it is open, and the rule it shows (null: a book's introduction) */
  ruleBook = $state<{ open: boolean; rule: string | null }>({ open: false, rule: null });

  /** Opens a rule in the rule books' window. */
  openRule(id: string): void {
    this.ruleBook = { open: true, rule: id };
  }

  /** The row of an attribute list to show once it is selected (opened from an entity's list of all its
   * attributes) */
  attributeRow = $state<number | null>(null);

  /** Opens an attribute where it is defined (a row of an entity's list of all attributes): selects the
   * element, shows it, and in an attribute list puts the cursor in the attribute's name. */
  gotoAttribute(target: string): void {
    const tab = this.current;
    const at = this.edit((d) => d.goto_element(target));
    if (at.length !== 2) return this.message("That attribute is no longer there.");
    tab.view = centreOn(tab.view, at[0], at[1], tab.size.w, tab.size.h);
    const row = target.split(":")[1];
    this.attributeRow = row === undefined ? null : Number(row);
  }

  /** What the Arrange dialog starts with: the whole diagram, or the selection (Arrange Selection…) */
  arrangeScope = $state<"all" | "selection" | "pin">("all");
  /** The last arrangement, while it can still be kept or undone (the result bar) */
  arrangeResult = $state<{ name: string; before: Figures; after: Figures; tab: number; version: number } | null>(null);

  /** The candidates of lib/arrange.ts, computed in a worker. */
  private arrangeCandidates(graph: Graph, options: ArrangeOptions): Promise<{ name: string; arrangement: Arrangement }[]> {
    return new Promise((resolve, reject) => {
      const worker = new Worker(new URL("./arrangeWorker.ts", import.meta.url), { type: "module" });
      worker.onmessage = (e: MessageEvent<{ ok: boolean; candidates?: { name: string; arrangement: Arrangement }[]; error?: string }>) => {
        worker.terminate();
        if (e.data.ok) resolve(e.data.candidates ?? []);
        else reject(new Error(e.data.error));
      };
      worker.onerror = (e) => {
        worker.terminate();
        reject(new Error(e.message));
      };
      worker.postMessage({ graph, options });
    });
  }

  /** Arranges the current diagram (README, "Arranging"): the candidates measured by the core, the best
   * one written in as one step that can be undone, unless none is better than the diagram as it is. */
  async arrange(options: ArrangeOptions): Promise<void> {
    const tab = this.current;
    const graph = JSON.parse(tab.diagram.arrange_graph()) as Graph;
    if (!graph.nodes.length) return this.message("There is nothing to arrange.");
    if (options.scope && options.scope !== "all" && !graph.selected?.length)
      return this.message("The selection holds no elements to arrange or keep in place (lines alone don't count).");
    const before = JSON.parse(tab.diagram.arrange_figures()) as Figures;
    this.message("Arranging…", 60000);
    let cands: { name: string; arrangement: Arrangement }[];
    try {
      cands = await this.arrangeCandidates(graph, options);
    } catch (e) {
      return this.message(`Arranging failed: ${e}`);
    }
    if (tab !== this.current || !this.tabs.includes(tab)) return this.message("");
    let best: { name: string; text: string; figures: Figures } | null = null;
    for (const c of cands) {
      const text = JSON.stringify(c.arrangement);
      const figures = JSON.parse(tab.diagram.arrange_try(text)) as Figures;
      if (!best || figures.score < best.figures.score) best = { name: c.name, text, figures };
    }
    if (!best || best.figures.score >= before.score - 1e-6) return this.message("The diagram is as tidy as arranging can make it.");
    this.edit((d) => d.arrange_apply(best.text));
    this.arrangeResult = { name: best.name, before, after: best.figures, tab: tab.id, version: tab.version };
    this.message("");
  }

  /** The Element pane's "Add attribute list" for the selected element (set by the pane while it is
   * shown, so that the cursor goes into the new list's first name). */
  panelButton: ((id: string) => unknown) | null = null;

  /** Whether the selected element can get an attribute list (entities, relationships, PART_OF). */
  canAddAttributeList(): boolean {
    void this.current.sel;
    void this.current.version;
    try {
      const form = JSON.parse(this.current.diagram.panel()) as { rows: { id: string }[] };
      return form.rows.some((r) => r.id === "add-tree");
    } catch {
      return false;
    }
  }

  /** Adds an attribute list to the selected element, as the Element pane's button does. */
  async addAttributeList(): Promise<void> {
    if (!this.canAddAttributeList()) {
      this.message("Select an entity, a relationship or a PART_OF to add an attribute list to it.");
      return;
    }
    if (this.panelButton) await this.panelButton("add-tree");
    else await this.panelSet("add-tree", "");
  }

  /** The next or previous group's tab becomes the current one. */
  nextGroup(step: number): void {
    const tab = this.byId(this.groups?.nextGroup(this.currentId, step) ?? null);
    if (tab) this.select(tab);
  }

  async closeTab(tab: Tab = this.current): Promise<boolean> {
    if (!(await this.maybeSave(tab))) return false;
    if (this.tabs.length === 1) this.select(this.addTab(this.fresh(tab.kind))); // there is always a tab
    const neighbour = this.byId(this.groups?.successor(tab.id) ?? null); // in its group
    const i = this.tabs.indexOf(tab);
    this.tabs.splice(i, 1);
    if (this.currentId === tab.id) this.select(neighbour ?? this.tabs[Math.min(i, this.tabs.length - 1)]);
    // freed once the page has updated: until then the tab's canvas (in its group) is still there
    await tick();
    tab.diagram.free();
    return true;
  }

  // -- documents ----------------------------------------------------------------------

  private target(): Tab {
    return this.current.pristine ? this.current : this.addTab(this.fresh("er"));
  }

  newDiagram(kind: Kind): void {
    const tab = this.target();
    tab.replace(this.fresh(kind));
    tab.path = null;
    tab.suggested = null;
    tab.example = false;
    tab.pending = "origin";
    this.select(tab);
  }

  /** File → Open Example: the sample diagram of a kind, in a tab of its own, not yet saved anywhere. */
  async openExample(kind: Kind): Promise<void> {
    const { EXAMPLES } = await import("./examples");
    const sample = EXAMPLES[kind];
    const open = this.tabs.find((t) => t.example && t.path === null && t.suggested === sample.name && !t.modified);
    if (open) return this.select(open);
    const d = new Diagram(sample.text, kind);
    this.prepare(d);
    const tab = this.target();
    tab.replace(d);
    tab.path = null;
    tab.suggested = sample.name;
    tab.example = true;
    tab.pending = "fit";
    this.select(tab);
  }

  async open(): Promise<void> {
    const start = this.current.path ? dirName(this.current.path) : (this.settings["last dir"] ?? "");
    for (const path of await be.openDialog(start)) await this.openPath(path);
  }

  /** Where an output of the tab's diagram is saved first (exports, the relational schema): where the
   * last such output of this diagram went (`key`: a map from diagram to folder), else next to the
   * diagram, else where the last one of any diagram went (`global`). */
  outputDir(tab: Tab, key: string, global: string): string {
    const known = tab.path ? this.readMap(key)[tab.path] : undefined;
    return known ?? (tab.path ? dirName(tab.path) : (this.settings[global] ?? ""));
  }

  /** Remembers where an output of the tab's diagram went (the last 100 diagrams). */
  async rememberOutputDir(tab: Tab, key: string, global: string, dir: string): Promise<void> {
    await this.setSetting(global, dir);
    if (!tab.path) return;
    const map = this.readMap(key);
    delete map[tab.path];
    map[tab.path] = dir;
    const entries = Object.entries(map).slice(-100);
    await this.setSetting(key, JSON.stringify(Object.fromEntries(entries)));
  }

  private readMap(key: string): Record<string, string> {
    try {
      const m = JSON.parse(this.settings[key] ?? "{}") as unknown;
      return m && typeof m === "object" && !Array.isArray(m) ? (m as Record<string, string>) : {};
    } catch {
      return {};
    }
  }

  /** In a tab of its own; a file that's already open just gets shown. */
  async openPath(path: string, quiet = false): Promise<boolean> {
    const abs = await be.absolutePath(path);
    const open = this.tabWithPath(abs);
    if (open) {
      this.select(open);
      return true;
    }
    const opened = await this.read(path, quiet);
    if (!opened) return false;
    const diagram = this.parse(opened, quiet);
    if (!diagram) return false;
    const tab = this.target();
    this.show(tab, diagram, opened.path);
    this.select(tab);
    await this.setSetting("last dir", dirName(opened.path));
    await this.addRecent(opened.path);
    this.message(`Opened ${opened.path}`);
    return true;
  }

  private async complain(text: string, quiet: boolean): Promise<void> {
    if (quiet) this.message(text.replace(/\n/g, " "), 8000);
    else await this.alert(text);
  }

  private async read(path: string, quiet: boolean): Promise<Opened | null> {
    try {
      return await be.readDiagram(path);
    } catch (e) {
      await this.complain(String(e), quiet);
      return null;
    }
  }

  private parse(opened: Opened, quiet: boolean): Diagram | null {
    try {
      const d = new Diagram(opened.text, opened.kind);
      this.prepare(d);
      return d;
    } catch (e) {
      void this.complain(`Cannot read ${baseName(opened.path)}:\n${e}`, quiet);
      return null;
    }
  }

  private show(tab: Tab, diagram: Diagram, path: string): void {
    tab.replace(diagram);
    tab.path = path;
    tab.example = false;
    tab.stamp = null;
    void be.fileStamp(path).then((s) => (tab.stamp = s));
    tab.suggested = null;
    tab.pending = "fit";
  }

  async maybeSave(tab: Tab = this.current): Promise<boolean> {
    if (!tab.modified) return true;
    this.select(tab); // so that it's clear which one, and Save saves it
    const r = await this.ask(`${tab.name} has unsaved changes. Save them?`, ["Save", "Discard", "Cancel"]);
    if (r === "Save") return await this.save(tab);
    return r === "Discard";
  }

  async save(tab: Tab = this.current): Promise<boolean> {
    return tab.path ? await this.saveTo(tab, tab.path) : await this.saveAs(tab);
  }

  async saveAs(tab: Tab = this.current): Promise<boolean> {
    const ext = EXTENSIONS[tab.kind];
    const lastDir = this.settings["last dir"] ?? "";
    const start = tab.path ?? tab.suggested ?? (lastDir ? `${lastDir}/diagram${ext}` : `diagram${ext}`);
    let path = await be.saveDialog(start, tab.kind);
    if (!path) return false;
    if (!be.hasSuffix(path)) path += ext;
    const saved = await this.saveTo(tab, path);
    if (saved && tab.path) await this.setSetting("last dir", dirName(tab.path)); // Open starts there next
    return saved;
  }

  /** Writes the tab's diagram to `path`, with the Python app's checks. */
  async saveTo(tab: Tab, path: string): Promise<boolean> {
    const other = await be.fileKind(path);
    if (other !== null && other !== tab.kind) {
      const what = other === "gsm" ? "a GSM" : "an ER";
      await this.alert(
        `${baseName(path)} is ${what} diagram file. This ${tab.kind.toUpperCase()} diagram is not saved over it — please choose another name.`,
      );
      return path === tab.path || tab.path === null ? await this.saveAs(tab) : false;
    }
    const abs = await be.absolutePath(path);
    const same = this.tabWithPath(abs);
    if (same && same !== tab) {
      await this.alert(`${baseName(path)} is open in another tab. Close it there, or choose another name.`);
      return false;
    }
    try {
      tab.path = await be.writeDiagram(path, tab.diagram.text());
    } catch (e) {
      await this.alert(String(e));
      return false;
    }
    tab.suggested = null;
    tab.diagram.mark_saved();
    tab.changed();
    tab.stamp = await be.fileStamp(tab.path);
    await this.addRecent(tab.path);
    this.message(`Saved ${tab.path}`);
    return true;
  }

  async reload(): Promise<void> {
    const tab = this.current;
    if (!tab.path || !(await this.maybeSave(tab))) return;
    const opened = await this.read(tab.path, false);
    const diagram = opened && this.parse(opened, false);
    if (opened && diagram) {
      this.show(tab, diagram, opened.path);
      this.message(`Reloaded ${baseName(opened.path)}`, 3000);
    }
  }

  // -- editing ----------------------------------------------------------------------------

  /** After the editor did something: draw again what changed, and show what it says. */
  settle(tab: Tab = this.current, revision?: number): void {
    const d = tab.diagram;
    if (revision === undefined || d.revision() !== revision) tab.version++;
    tab.ui++;
    for (const ev of d.events()) {
      const [what, text] = ev.split("\t");
      if (what === "message") this.message(text, 6000);
      else if (what === "refused") void this.alert(text);
      else if (what === "selection") tab.sel++;
      else if (what === "edit-label") tab.labelRequest++;
    }
  }

  /** Runs `f` on the active tab's diagram, then settles. */
  edit<T>(f: (d: Diagram) => T): T {
    const tab = this.current;
    const before = tab.diagram.revision();
    const r = f(tab.diagram);
    this.settle(tab, before);
    return r;
  }

  undo(): void {
    this.edit((d) => d.undo());
  }

  redo(): void {
    this.edit((d) => d.redo());
  }

  toggleTypes(): void {
    if (this.current.kind === "er") this.edit((d) => d.set_types_shown(!d.types_shown()));
  }

  setTool(tool: string): void {
    this.edit((d) => d.set_tool(tool));
  }

  async copy(): Promise<void> {
    const text = this.edit((d) => d.copy());
    if (text) await be.copyText(text);
  }

  async cut(): Promise<void> {
    const text = this.edit((d) => d.cut());
    if (text) await be.copyText(text);
  }

  /** From the clipboard: where the pointer is over the canvas, else in the middle of the view;
   * or at (x, y) cm. */
  async paste(at?: [number, number]): Promise<void> {
    const text = await be.pasteText();
    const tab = this.current;
    const [sx, sy] = tab.pointer ?? [tab.size.w / 2, tab.size.h / 2];
    const [x, y] = at ?? toDiagram(tab.view, sx, sy).map((v) => v / CM);
    this.edit((d) => d.paste(text, x, y));
  }

  duplicate(): void {
    this.edit((d) => d.duplicate());
  }

  deleteSelection(): void {
    this.edit((d) => d.delete_selection());
  }

  selectAll(): void {
    this.edit((d) => d.select_all());
  }

  /** A diagram converted from the GSM diagram in `from`: in a new tab, not saved yet, with a
   * name next to the GSM file (`aspects_er.erd`). */
  openConverted(text: string, from: Tab): void {
    const d = new Diagram(text, "er");
    this.prepare(d);
    const tab = this.addTab(d);
    const folder = from.path ? dirName(from.path) : (this.settings["last dir"] ?? "");
    const stem = from.path ? baseName(from.path).replace(/\.[^.]*$/, "") : "diagram";
    tab.suggested = folder ? `${folder}/${stem}_er${EXTENSIONS.er}` : `${stem}_er${EXTENSIONS.er}`;
    tab.pending = "fit";
    this.select(tab);
    this.message("Converted from the GSM diagram — not saved yet (File → Save)", 8000);
  }

  /** A field of the properties panel changed: asks first where the core says so.  An invalid
   * value comes back for the field to show. */
  async panelSet(id: string, value: string): Promise<Outcome> {
    let out = JSON.parse(this.edit((d) => d.panel_set(id, value, false))) as Outcome;
    if (out.kind === "ask") {
      const tab = this.current;
      const answer = await this.ask(out.text, ["Yes", "No"], out.title);
      if (answer !== "Yes" || tab !== this.current) {
        tab.sel++; // the field shows the value it had
        return { kind: "done" };
      }
      out = JSON.parse(this.edit((d) => d.panel_set(id, value, true))) as Outcome;
    }
    if (out.kind === "refused") {
      this.message(out.text, 6000);
      this.current.sel++;
    } else if (out.kind === "inform") {
      await this.alert(out.text);
    }
    return out;
  }

  /** The language of the labels the packages write (IS_A, IST_EIN): how diagrams are shown,
   * never what the files say. */
  async setLanguage(language: string): Promise<void> {
    await this.setSetting("label language", language);
    for (const t of this.tabs) {
      t.diagram.set_language(language);
      t.changed();
    }
  }

  async setShowHints(on: boolean): Promise<void> {
    await this.setSetting("show hints", on ? "true" : "false");
    for (const t of this.tabs) {
      t.diagram.set_show_hints(on);
      t.changed();
    }
  }

  async setSnap(cm: number): Promise<void> {
    await this.setSetting("snap", String(cm));
    for (const t of this.tabs) t.diagram.set_snap(cm);
  }

  // -- view ---------------------------------------------------------------------------

  zoomBy(factor: number): void {
    this.current.zoomAround(zoomOf(this.current.view) * factor);
  }

  async toggleGrid(): Promise<void> {
    await this.setSetting("show grid", this.showGrid ? "false" : "true");
  }

  async setMode(mode: Mode): Promise<void> {
    if (mode in MODES) await this.setSetting("display mode", mode);
  }
}
