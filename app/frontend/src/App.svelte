<script lang="ts">
  // The main window: menus, the toolbar, the palette, tabs, the canvas, the Element and
  // Problems docks, and the status bar.  The LaTeX preview and exports follow in Milestone 7.
  import { onMount } from "svelte";
  import MenuBar, { type Item, type Menu } from "./components/MenuBar.svelte";
  import Palette from "./components/Palette.svelte";
  import Toolbar from "./components/Toolbar.svelte";
  import Question from "./components/Question.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import Groups from "./components/groups/Groups.svelte";
  import ContextMenu from "./components/ContextMenu.svelte";
  import Inspector from "./components/panel/Inspector.svelte";
  import Problems from "./components/panel/Problems.svelte";
  import Preferences from "./components/prefs/Preferences.svelte";
  import ConvertDialog from "./components/dialogs/ConvertDialog.svelte";
  import ExportDialog from "./components/dialogs/ExportDialog.svelte";
  import ArrangeDialog from "./components/dialogs/ArrangeDialog.svelte";
  import ArrangeResult from "./components/dialogs/ArrangeResult.svelte";
  import Preview from "./components/panel/Preview.svelte";
  import DockArea from "./components/dock/DockArea.svelte";
  import DockPanel from "./components/dock/DockPanel.svelte";
  import type { Area, PaneId } from "./lib/docks.svelte";
  import RelationalWindow from "./components/dialogs/RelationalWindow.svelte";
  import { dirty, revert } from "./lib/panel";
  import { App, APP_NAME } from "./lib/app.svelte";
  import * as be from "./lib/backend";
  import { LATEX_IN_BROWSER } from "./lib/backend";
  import { IMPRINT, PRIVACY, RELEASES, REPOSITORY, WEBSITE } from "./lib/links";
  import { Diagram, startCore } from "./lib/core";
  import { Bindings, commands, display, type Command } from "./lib/keys";
  import { MODES, type Mode } from "./lib/theme";

  const app = new App();
  let started = $state(false);
  let table = $state<Command[]>([]);
  let bindings = $state<Bindings | null>(null);
  const marks: string[] = [];
  const mark = (what: string) => marks.push(`${what} ${performance.now().toFixed(0)}`);
  mark("script");

  const keysOf = (id: string) => display(bindings?.keys.get(id) ?? "");

  // what a command does; the others come with later milestones
  const actions: Record<string, () => unknown> = {
    a_new_er: () => app.newDiagram("er"),
    a_new_gsm: () => app.newDiagram("gsm"),
    a_open: () => app.open(),
    a_save: () => app.save(),
    a_save_as: () => app.saveAs(),
    a_reload: () => app.reload(),
    a_close_tab: () => app.closeTab(),
    a_next_tab: () => app.nextTab(1),
    a_prev_tab: () => app.nextTab(-1),
    a_split_right: () => app.splitTab("right"),
    a_split_down: () => app.splitTab("bottom"),
    a_next_group: () => app.nextGroup(1),
    a_prev_group: () => app.nextGroup(-1),
    a_move_tab_group: () => app.moveTabToOtherGroup(),
    a_add_attributes: () => app.addAttributeList(),
    a_quit: () => app.quit(),
    a_undo: () => app.undo(),
    a_redo: () => app.redo(),
    a_zoom_in: () => app.zoomBy(1.25),
    a_zoom_out: () => app.zoomBy(0.8),
    a_zoom_100: () => app.current.zoomAround(1),
    a_fit: () => app.current.fit(),
    a_grid: () => app.toggleGrid(),
    a_types: () => app.toggleTypes(),
    a_cut: () => app.cut(),
    a_copy: () => app.copy(),
    a_paste: () => app.paste(),
    a_dup: () => app.duplicate(),
    a_all: () => app.selectAll(),
    a_keys: () => showKeys(),
    a_rulebook: () => (app.ruleBook = { open: true, rule: app.ruleBook.rule }),
    a_preferences: () => (app.dialog = "preferences"),
    a_export: () => (app.dialog = "export"),
    a_arrange: () => {
      app.arrangeScope = "all";
      app.dialog = "arrange";
    },
    a_arrange_selection: () => {
      if (!app.current.selected) return app.message("Select the elements to arrange first.");
      app.arrangeScope = "selection";
      app.dialog = "arrange";
    },
    a_copy_input: () => app.copyInputLine(),
    a_preview: () => (be.IS_WEB ? app.message(LATEX_IN_BROWSER, 8000) : app.togglePreview()),
    a_relational: () => app.current.kind === "er" && (app.relational = true),
    a_convert: () => app.current.kind === "gsm" && (app.dialog = "convert"),
    a_about: () => void about(),
    a_example_er: () => void app.openExample("er"),
    a_example_gsm: () => void app.openExample("gsm"),
    a_save_packages: () => void import("./lib/latexPackages").then((m) => m.savePackages(app)),
  };

  const item = (label: string, id: string, extra: Partial<Item> = {}) => ({
    label,
    action: actions[id],
    keys: keysOf(id),
    icon: id,
    ...extra,
  });

  const menus: Menu[] = $derived.by(() => {
    const t = app.current;
    if (!started || !t) return [];
    void t.ui;
    const undo = t.diagram.undo_label();
    const redo = t.diagram.redo_label();
    void app.groupsVersion; // the groups' entries follow the groups
    void app.tabs.length;
    const canSplit = !!app.groups?.canSplit(t.id);
    const canMove = !!app.groups?.canMoveToOther(t.id);
    const groupCount = app.groups?.list().length ?? 1;
    return [
      {
        label: "File",
        items: [
          item("New ER Diagram", "a_new_er"),
          item("New GSM Diagram", "a_new_gsm"),
          item("Open…", "a_open"),
          // the website: a browser can't reopen files by their paths, nor reload them or quit
          ...(be.IS_WEB
            ? []
            : [
                {
                  label: "Open Recent",
                  icon: "recent",
                  disabled: !app.recent.length,
                  items: app.recent.map((p) => ({ label: p, action: () => app.openPath(p) })),
                },
              ]),
          {
            label: "Open Example",
            icon: "example",
            items: [item("ER Diagram: a Music Festival", "a_example_er"), item("GSM Diagram: a Music Festival", "a_example_gsm")],
          },
          "-",
          item("Save", "a_save"),
          item("Save As…", "a_save_as"),
          ...(be.IS_WEB ? [] : [item("Reload from Disk", "a_reload", { disabled: !t.path })]),
          item("Close Tab", "a_close_tab"),
          "-",
          item("Export…", "a_export"),
          item("Relational Schema / SQL…", "a_relational", { disabled: t.kind !== "er" }),
          item("Convert to ER Diagram…", "a_convert", { disabled: t.kind !== "gsm" }),
          item("Copy \\input Line", "a_copy_input"),
          ...(be.IS_WEB ? [] : ["-" as const, item("Quit", "a_quit")]),
        ],
      },
      {
        label: "Edit",
        items: [
          item(undo ? `Undo ${undo}` : "Undo", "a_undo", { disabled: !undo }),
          item(redo ? `Redo ${redo}` : "Redo", "a_redo", { disabled: !redo }),
          "-",
          item("Cut", "a_cut", { disabled: !t.selected }),
          item("Copy", "a_copy", { disabled: !t.selected }),
          item("Paste", "a_paste"),
          item("Duplicate", "a_dup", { disabled: !t.selected }),
          item("Add Attribute List", "a_add_attributes", { disabled: !app.canAddAttributeList() }),
          { label: "Delete", action: () => app.deleteSelection(), keys: keysOf("delete"), disabled: !t.selected, icon: "delete" },
          "-",
          item("Select All", "a_all"),
          "-",
          item("Arrange Diagram…", "a_arrange"),
          item("Arrange Selection…", "a_arrange_selection", { disabled: !t.selected }),
        ],
      },
      {
        label: "View",
        items: [
          item("Zoom In", "a_zoom_in"),
          item("Zoom Out", "a_zoom_out"),
          item("Actual Size", "a_zoom_100"),
          item("Fit Diagram", "a_fit"),
          "-",
          item("Show Grid", "a_grid", { checked: app.showGrid }),
          paneItem("tools"),
          paneItem("element"),
          paneItem("preview", "a_preview"),
          paneItem("problems"),
          "-",
          item("Show Data Types", "a_types", { checked: t.kind === "er" && t.diagram.types_shown(), disabled: t.kind !== "er" }),
          {
            label: "Display Mode",
            icon: "mode",
            items: (Object.keys(MODES) as Mode[]).map((m) => ({
              label: MODES[m],
              checked: app.mode === m,
              action: () => app.setMode(m),
            })),
          },
          "-",
          item("Next Tab", "a_next_tab"),
          item("Previous Tab", "a_prev_tab"),
          "-",
          item("Split Right", "a_split_right", { disabled: !canSplit }),
          item("Split Down", "a_split_down", { disabled: !canSplit }),
          item("Move Tab to Other Group", "a_move_tab_group", { disabled: !canMove }),
          item("Next Group", "a_next_group", { disabled: groupCount < 2 }),
          item("Previous Group", "a_prev_group", { disabled: groupCount < 2 }),
        ],
      },
      { label: "Settings", items: [item("Preferences…", "a_preferences")] },
      {
        label: "Help",
        items: [
          item("Mouse and Keyboard", "a_keys"),
          item("Rule Books…", "a_rulebook"),
          item("Save LaTeX Packages…", "a_save_packages"),
          "-",
          be.IS_WEB
            ? { label: "Download the Desktop Version", icon: "download", action: () => void be.openUrl(RELEASES) }
            : { label: "Open the Website", icon: "website", action: () => void be.openUrl(WEBSITE) },
          { label: "Source Code on GitHub", icon: "source", action: () => void be.openUrl(REPOSITORY) },
          ...(be.IS_WEB ? [{ label: "Privacy", icon: "privacy", action: () => (app.textPage = { title: "Privacy", markdown: PRIVACY }) }] : []),
          ...(IMPRINT ? [{ label: "Imprint", icon: "imprint", action: () => (app.textPage = { title: "Imprint", markdown: IMPRINT }) }] : []),
          "-",
          item("About", "a_about"),
        ],
      },
    ];
  });

  function showKeys() {
    const kindTools = app.current.kind === "gsm" ? "GSM tools" : "ER tools";
    const groups = ["On the canvas and in lists", kindTools, "General"];
    const lines = groups.map((g) => {
      const rows = table
        .filter((c) => c.group === g && (actions[c.id] || g !== "General") && bindings?.keys.get(c.id))
        .map((c) => `${keysOf(c.id)}: ${c.title}`);
      return `${g}: ${rows.join(" · ")}`;
    });
    void app.alert(
      "Mouse\nWheel: zoom · Shift+wheel: scroll · Middle button or Space+drag: pan\n" +
        "Click: select · Shift/Ctrl+click: add to the selection · drag on empty space: select an area\n" +
        "Drag elements to move them (Alt: without snapping)\n" +
        "Edges: drag the round handle to curve them, the squares to move way points or to reconnect the ends; " +
        "double-click an edge to add a way point, a way point to remove it\n" +
        "Attribute lists: drag them around their node to change the side; ovals: drag one to fix its place\n" +
        "Subdomain boxes: moving one moves what's inside; drag the squares to move corners, the circles to add " +
        "a corner, double-click a corner to remove it\n" +
        "Connection tools: press on the first element and drag to the second; red means the connection is not allowed\n\n" +
        "Keyboard\nSpace (held): pan with the mouse\n" +
        lines.join("\n"),
    );
  }

  async function about() {
    const [version, { PACKAGES }] = await Promise.all([be.appVersion(), import("./lib/latexPackages")]);
    const packages = PACKAGES.map((p) => `${p.name} ${p.version}`).join(", ");
    const answer = await app.ask(
      `${APP_NAME} ${version}\nER and GSM diagrams for the erdiagram and gsmdiagram LaTeX packages.\n` +
        "Copyright (c) 2026 Leunami52, under the MIT licence.\n\n" +
        `Built-in LaTeX packages: ${packages} (Help → Save LaTeX Packages…).\n` +
        "The LaTeX preview and the export of the LaTeX result need pdflatex (TeX Live or MiKTeX).",
      ["OK", "Licences…"],
      "About",
    );
    if (answer === "Licences…") app.dialog = "licenses";
  }

  /** Keys for the canvas: Esc, Delete, the arrows (with Shift or Alt for the step). */
  const CANVAS = new Set(["cancel", "delete", "nudge left", "nudge right", "nudge up", "nudge down"]);

  function inField(e: KeyboardEvent): boolean {
    const el = e.target as HTMLElement | null;
    return !!el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT" || el.isContentEditable);
  }

  function onKeyDown(e: KeyboardEvent) {
    if (app.question || app.dialog || !bindings || e.defaultPrevented) return; // a dialog's or a field's own
    const id = bindings.commandFor(e, app.current.kind);
    const field = inField(e);
    // in a field, a plain character types (even if it is a shortcut, such as a letter for Esc)
    if (field && id && !e.ctrlKey && !e.metaKey && !e.altKey && [...e.key].length === 1) return;
    if (id && CANVAS.has(id)) {
      if (field && id !== "cancel") return;
      e.preventDefault();
      if (field) {
        // what was typed is kept (leaving the field commits it), then back to the canvas
        (e.target as HTMLElement).blur();
        app.focusCanvas();
      }
      app.edit((d) => d.key(id, e.shiftKey, e.altKey));
      return;
    }
    // Ctrl+Z in a field takes back its own typing first, then edits of the diagram
    if (id === "a_undo" && field && dirty(e.target as Element)) {
      e.preventDefault();
      revert(e.target as HTMLInputElement);
      return;
    }
    // in a field, the text's own: select all, copy, cut, paste (as in Qt)
    if (field && id && ["a_all", "a_copy", "a_cut", "a_paste"].includes(id)) return;
    if (id && /^(er|gsm)\./.test(id)) {
      e.preventDefault();
      app.setTool(id.split(".").slice(1).join("."));
      return;
    }
    if (id === "pan") {
      e.preventDefault();
      if (!e.repeat) app.spaceDown = true;
      return;
    }
    const action = id ? actions[id] : undefined;
    if (action) {
      e.preventDefault();
      if (!e.repeat || id?.startsWith("a_zoom")) void action();
    } else if ((e.ctrlKey || e.metaKey) && !(field && FIELD_KEYS.has(e.key))) {
      e.preventDefault(); // nothing the WebView would do on its own (reload, find, print …)
    }
  }

  /** Keys a text field handles with Ctrl too, when no command has them: Ctrl+Backspace and Ctrl+Delete
   * delete a word, Ctrl+arrows move by word, with Shift they select. */
  const FIELD_KEYS = new Set(["Backspace", "Delete", "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"]);

  /** WebKit's own menu for web pages (Back, Forward, Stop, Reload — Reload would even start the page
   * anew, unsaved work lost) appears nowhere; text fields keep theirs (Cut, Copy, Paste). */
  function noPageMenu(e: MouseEvent) {
    const t = e.target as HTMLElement | null;
    if (!t?.closest?.("input, textarea, [contenteditable]")) e.preventDefault();
  }

  function onKeyUp(e: KeyboardEvent) {
    if (bindings?.commandFor(e, app.current.kind) === "pan" || e.key === " ") app.spaceDown = false;
  }

  function closeDialog() {
    app.dialog = null;
    app.focusCanvas();
  }

  /** After Preferences: shortcuts as they are now in the settings. */
  function reloadBindings() {
    bindings = new Bindings(table, app.settings);
    app.bindings = bindings;
  }

  /** The window's part of `--self-test` (the rest runs before the window, in Rust): what the
   * page does with this build's WebView. */
  async function selfTest() {
    let ok = true;
    const check = (name: string, cond: boolean, detail = "") => {
      ok &&= cond;
      void be.selfTestLine(`${cond ? "ok  " : "FAIL"}  ${name}${detail ? "  " + detail : ""}`);
    };
    void be.selfTestLine("info  the window: the page has started");
    const frames = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    try {
      check("the core (WebAssembly) and its fonts", document.fonts.size >= 37, `${document.fonts.size} fonts`);
      testHook.load("\\entity{p}(0,0){Person}\\attributes{p}{*PID}\\relationship{w}(3,0){works for}\\link{p}{w}{n}\n", "er");
      await frames();
      const drawn = document.querySelector("[data-testid=drawing]")?.innerHTML.length ?? 0;
      check("an ER diagram on the canvas", drawn > 2000, `${drawn} characters of SVG`);
      testHook.select(["p"]);
      await frames();
      check("the properties panel", testHook.panel().title === "Entity type");
      app.newDiagram("gsm");
      await frames();
      const tools = [...document.querySelectorAll("[data-tool]")].map((b) => (b as HTMLElement).dataset.tool);
      check("the GSM tools", tools.includes("abstract type") && !tools.includes("entity"));
      const keys = table.filter((c) => /^(er|gsm)\./.test(c.id)).every((c) => (bindings?.keys.get(c.id) ?? "") !== "");
      check("keyboard shortcuts for every tool", keys);
      for (const [mode, panels, canvas] of [["light", "#efefef", "rgb(255,255,255)"], ["dark", "#2b2b2b", "rgb(28,28,28)"], ["intermediate", "#2b2b2b", "rgb(255,255,255)"]] as const) {
        await app.setMode(mode);
        await frames();
        const win = getComputedStyle(document.body).getPropertyValue("--window").trim();
        const bg = (document.querySelector("[data-testid=canvas]") as HTMLElement | null)?.dataset.background ?? "";
        check(`display mode ${mode}`, win === panels && bg.replace(/ /g, "") === canvas, `${win}, ${bg}`);
      }
      await app.setMode("system");
      app.dialog = "preferences";
      await frames();
      check("preferences", document.querySelectorAll("[data-testid=preferences] [data-page]").length === 6);
      app.dialog = null;
      app.togglePreview();
      await frames();
      check("the LaTeX preview dock", !!document.querySelector("[data-testid=preview]"));
    } catch (e) {
      check("the window", false, String(e));
    }
    await be.selfTestReport(ok);
  }

  // the docks' sizes: the splitter between the editor and a side
  function resize(e: PointerEvent, area: Area) {
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const start = { x: e.clientX, y: e.clientY, size: app.docks.layout.size[area] };
    const move = (m: PointerEvent) => {
      const size =
        area === "left"
          ? Math.max(60, Math.min(start.size + (m.clientX - start.x), window.innerWidth - 300))
          : area === "right"
            ? Math.max(160, Math.min(start.size - (m.clientX - start.x), window.innerWidth - 300))
            : Math.max(60, Math.min(start.size - (m.clientY - start.y), window.innerHeight - 200));
      app.docks.setSize(area, size);
    };
    const up = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
  }

  const problemCount = $derived.by(() => {
    const t = app.current;
    if (!started || !t) return 0;
    void t.version;
    void app.showHints;
    return (JSON.parse(t.diagram.problems()) as { count: number }).count;
  });
  const paneTitle = (id: PaneId) =>
    ({ tools: "Tools", element: "Element", preview: "LaTeX Preview", problems: `Problems (${problemCount})` })[id];
  const paneItem = (id: PaneId, cmd?: string) => ({
    label: paneTitle(id),
    checked: app.docks.pane(id).visible && !(be.IS_WEB && id === "preview"),
    action: () => app.docks.toggle(id),
    keys: cmd ? keysOf(cmd) : undefined,
    icon: id,
    // the website has no pdflatex: the entry stays, greyed, and says why
    ...(be.IS_WEB && id === "preview" ? { disabled: true, tip: LATEX_IN_BROWSER } : {}),
  });

  // for tests (WebDriver): what a user does with the menus and dialogs
  const testHook = {
    open: (path: string) => app.openPath(path),
    save: () => app.save(),
    saveAs: (path: string) => app.saveTo(app.current, path),
    text: () => app.current.diagram.text(),
    tabTexts: () => app.tabs.map((t) => t.diagram.text()),
    state: () => ({
      tabs: app.tabs.map((t) => ({ name: t.name, path: t.path, modified: t.modified, kind: t.kind })),
      current: app.tabs.indexOf(app.current),
      zoom: app.current.view.k,
      view: { ...app.current.view },
      tool: app.current.tool,
      selected: app.current.selected,
      snap: app.snap,
      status: app.status,
      question: app.question?.text ?? null,
    }),
    answer: (button: string) => app.question?.answer(button),
    run: (id: string) => actions[id]?.(),
    tool: (name: string) => app.setTool(name),
    handles: () => {
      const h = app.current.diagram.handles();
      const out: Record<string, [number, number]> = {};
      for (let i = 0; i + 2 < h.length; i += 3) out[h[i]] = [Number(h[i + 1]), Number(h[i + 2])];
      return out;
    },
    paste: (text: string, x: number, y: number) => app.edit((d) => d.paste(text, x, y)),
    fresh: (kind: "er" | "gsm") => app.newDiagram(kind),
    load: (text: string, kind: "er" | "gsm") => {
      const d = new Diagram(text, kind);
      app.prepare(d);
      app.current.replace(d);
    },
    setMode: (m: Mode) => app.setMode(m),
    panel: () => JSON.parse(app.current.diagram.panel()),
    focusCanvas: () => app.focusCanvas(),
    settings: () => ({ ...app.settings }),
    preview: () => app.previewSnapshot?.() ?? null,
    docks: () => JSON.parse(JSON.stringify(app.docks.layout)),
    savePathOnce: (path: string) => (app.nextSavePath = path),
    /** the next dialogs' answers (a path, several for Open, or null for Cancel); `dialogsAsked` gives
     * what each was asked with */
    answerDialogs: (answers: (string | string[] | null)[]) => {
      be.dialogTest.answers = [...answers];
      be.dialogTest.asked = [];
    },
    dialogsAsked: () => JSON.parse(JSON.stringify(be.dialogTest.asked)),
    arrangeResult: () => JSON.parse(JSON.stringify(app.arrangeResult)),
    figures: () => JSON.parse(app.current.diagram.arrange_figures()),
    clearStatus: () => app.message(""),
    /** the website: a file "on disk" for the next Open (its made-up path), and what a path got written */
    webFile: (name: string, text: string) => (be as unknown as { addFile?: (n: string, t: string) => string }).addFile?.(name, text) ?? null,
    written: (path: string) => (be as unknown as { writtenText?: (p: string) => string | null }).writtenText?.(path) ?? null,
    recent: () => [...app.recent],
    watch: () => app.watch(),
    select: (ids: string[]) => app.edit((d) => d.select_ids(ids)),
    focused: () => {
      const el = document.activeElement as HTMLElement | null;
      if (!el || el === document.body) return "none";
      if (el.closest("[data-testid=canvas]")) return "canvas";
      const row = el.closest("[data-row]") as HTMLElement | null;
      const f = el.closest("[data-field]") as HTMLElement | null;
      return `${el.tagName.toLowerCase()}:${row?.dataset.row ?? f?.dataset.field ?? ""}`;
    },
  };

  onMount(async () => {
    marks.unshift(`page started ${((await be.sinceStart()) - performance.now()).toFixed(0)} ms after Rust;`);
    await startCore();
    mark("core and fonts");
    table = commands();
    bindings = new Bindings(table, await be.getSettings());
    app.bindings = bindings;
    await app.start();
    started = true;
    (window as unknown as { __erdtex: typeof testHook }).__erdtex = testHook;
    if (await be.selfTestMode()) void selfTest();
    // two frames: the diagram has been painted, not only put into the page
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        mark("painted");
        void be.ready(marks.join(", "));
      }),
    );
  });
</script>

{#if app.menu}
  <ContextMenu x={app.menu.x} y={app.menu.y} items={app.menu.items} close={() => (app.menu = null)} />
{/if}
<svelte:window onkeydown={onKeyDown} onkeyup={onKeyUp} onblur={() => (app.spaceDown = false)} oncontextmenu={noPageMenu} />

<div class="window">
  <MenuBar {menus} />
  {#if started}
    <Toolbar {app} run={(id) => actions[id]?.()} {keysOf} />
    {#snippet content(id: PaneId)}
      {#if id === "tools"}
        <Palette {app} {keysOf} />
      {:else if id === "element"}
        <Inspector {app} />
      {:else if id === "preview"}
        <Preview {app} />
      {:else}
        <Problems {app} />
      {/if}
    {/snippet}
    <div class="body">
      {#if app.docks.docked("left").length}
        <div class="dock side" style:width={app.docks.folded("left") ? "24px" : `${app.docks.layout.size.left}px`}>
          <DockArea docks={app.docks} area="left" title={paneTitle} {content} />
        </div>
        {#if !app.docks.folded("left")}
          <div class="split v" role="separator" aria-orientation="vertical" onpointerdown={(e) => resize(e, "left")}></div>
        {/if}
      {/if}
      <div class="centre">
        <div class="editor">
          <main><Groups {app} /></main>
        </div>
        {#if app.docks.docked("bottom").length}
          {#if !app.docks.folded("bottom")}
            <div class="split h" role="separator" aria-orientation="horizontal" onpointerdown={(e) => resize(e, "bottom")}></div>
          {/if}
          <div class="dock bottom" style:height={app.docks.folded("bottom") ? "auto" : `${app.docks.layout.size.bottom}px`}>
            <DockArea docks={app.docks} area="bottom" title={paneTitle} {content} />
          </div>
        {/if}
      </div>
      {#if app.docks.docked("right").length}
        {#if !app.docks.folded("right")}
          <div class="split v" role="separator" aria-orientation="vertical" onpointerdown={(e) => resize(e, "right")}></div>
        {/if}
        <div class="dock side" style:width={app.docks.folded("right") ? "24px" : `${app.docks.layout.size.right}px`}>
          <DockArea docks={app.docks} area="right" title={paneTitle} {content} />
        </div>
      {/if}
    </div>
    {#each app.docks.floating() as id (id)}
      <DockPanel docks={app.docks} {id} title={paneTitle(id)}>{@render content(id)}</DockPanel>
    {/each}
    {#if app.docks.drag}
      <div class="dropzones" aria-hidden="true">
        {#each ["left", "right", "bottom"] as z}
          <div class="zone {z}" class:on={app.docks.drag.target === z}></div>
        {/each}
      </div>
    {/if}
    <ArrangeResult {app} />
    <StatusBar {app} />
  {:else}
    <main></main>
  {/if}
</div>
{#if app.ruleBook.open}
  <!-- loaded when first opened, with its libraries and pages -->
  {#await import("./components/dialogs/RuleBook.svelte") then m}
    <m.default {app} close={() => (app.ruleBook = { open: false, rule: app.ruleBook.rule })} />
  {/await}
{/if}
{#if app.relational}
  <RelationalWindow {app} close={() => (app.relational = false)} />
{/if}
{#if app.dialog === "arrange"}
  <ArrangeDialog {app} close={closeDialog} />
{/if}
{#if app.dialog === "export"}
  <ExportDialog {app} close={closeDialog} />
{/if}
{#if app.dialog === "convert"}
  <ConvertDialog {app} close={closeDialog} />
{/if}
{#if app.textPage}
  {#await import("./components/dialogs/TextDialog.svelte") then m}
    <m.default title={app.textPage.title} markdown={app.textPage.markdown} close={() => (app.textPage = null)} />
  {/await}
{/if}
{#if app.dialog === "licenses"}
  {#await import("./components/dialogs/LicensesDialog.svelte") then m}
    <m.default {app} close={closeDialog} />
  {/await}
{/if}
{#if app.dialog === "preferences"}
  <Preferences {app} commands={table} close={closeDialog} applied={reloadBindings} />
{/if}
{#if app.question}
  <Question q={app.question} />
{/if}

<style>
  :global(html, body) {
    margin: 0;
    height: 100%;
    overflow: hidden;
    font: 13px system-ui, sans-serif;
    background: var(--window, #efefef);
  }
  :global(#app) {
    height: 100%;
  }
  .window {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .centre {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }
  .editor {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  .dock.side {
    display: flex;
    flex-direction: column;
  }
  .dropzones {
    position: fixed;
    inset: 0;
    pointer-events: none;
    z-index: 34;
  }
  .zone {
    position: absolute;
    background: color-mix(in srgb, var(--highlight) 12%, transparent);
    border: 2px dashed color-mix(in srgb, var(--highlight) 50%, transparent);
  }
  .zone.on {
    background: color-mix(in srgb, var(--highlight) 35%, transparent);
    border-color: var(--highlight);
  }
  .zone.left {
    left: 0;
    top: 70px;
    bottom: 24px;
    width: 56px;
  }
  .zone.right {
    right: 0;
    top: 70px;
    bottom: 24px;
    width: 56px;
  }
  .zone.bottom {
    left: 56px;
    right: 56px;
    bottom: 24px;
    height: 56px;
  }
  .dock {
    flex: none;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
    background: var(--window);
  }
  .split {
    flex: none;
    background: var(--mid);
  }
  .split.v {
    width: 4px;
    cursor: ew-resize;
  }
  .split.h {
    height: 4px;
    cursor: ns-resize;
  }
  main {
    position: relative;
    flex: 1;
    min-height: 0;
  }
</style>
