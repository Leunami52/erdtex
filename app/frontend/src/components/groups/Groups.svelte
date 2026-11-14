<script lang="ts">
  // The editor area: groups of tabs side by side (README, "Tabs side by side"), laid out by dockview; each tab a
  // canvas.  dockview's own dragging is off (it uses HTML drag and drop, which the window takes for
  // dropping files); our pointer dragging moves tabs through its interface (lib/groups.ts): onto a tab
  // bar (moved there, at the line shown), or onto a group's content, whose edges split it.
  import { mount, unmount, untrack } from "svelte";
  // the single-file build: only it carries dockview's styles (and injects them); the same functions
  import { createDockview } from "dockview-core/dist/dockview-core.js";
  import type { DockviewApi, DockviewGroupPanel } from "dockview-core";
  import type { App, Tab } from "../../lib/app.svelte";
  import { Groups, restoreLayout, type Side } from "../../lib/groups";
  import Canvas from "../Canvas.svelte";
  import GroupTab from "./GroupTab.svelte";

  let { app }: { app: App } = $props();
  let host: HTMLDivElement;
  let api: DockviewApi | undefined = $state();

  const tabOf = (id: string) => app.tabs.find((t) => String(t.id) === id);

  /** A Svelte component inside an element dockview gives us. */
  function renderer(make: (target: HTMLElement, tab: Tab) => ReturnType<typeof mount>, css: string) {
    const element = document.createElement("div");
    element.style.cssText = css;
    let mounted: ReturnType<typeof mount> | null = null;
    // the canvas measures itself again when dockview lays it out or shows it
    const measure = () => requestAnimationFrame(() => (mounted as { measure?: () => void } | null)?.measure?.());
    return {
      element,
      init(params: { api: { id: string } }) {
        const tab = tabOf(params.api.id);
        if (tab) mounted = make(element, tab);
      },
      layout: measure,
      onShow: measure,
      dispose() {
        if (mounted) void unmount(mounted);
      },
    };
  }

  $effect(() => {
    const dv = createDockview(host, {
      disableDnd: true,
      theme: { name: "erdtex", className: "dockview-theme-erdtex" },
      createComponent: () =>
        renderer((target, tab) => {
          // a press in a group's canvas makes its tab the current one first, so that the editor
          // (which edits the current tab) gets it
          target.addEventListener("pointerdown", () => app.select(tab), { capture: true });
          return mount(Canvas, { target, props: { app, tab } });
        }, "position: relative; width: 100%; height: 100%"),
      createTabComponent: () => renderer((target, tab) => mount(GroupTab, { target, props: { app, tab } }), "height: 100%"),
    });
    // laid out at once, before the first canvas: a canvas fits its diagram to the size it first gets
    dv.layout(host.clientWidth, host.clientHeight);
    const ro = new ResizeObserver(() => dv.layout(host.clientWidth, host.clientHeight));
    ro.observe(host);
    // only the user's changes (a click on a tab or a group): the app's own (adding, moving, closing a
    // tab) already decided the current tab, and dockview reports some of them late
    dv.onDidActivePanelChange(({ panel, origin }) => {
      if (origin !== "user") return;
      const tab = panel && tabOf(panel.id);
      if (tab && tab.id !== app.currentId) app.select(tab);
    });
    dv.onDidLayoutChange(() => app.groupsVersion++);
    const groups = new Groups(dv);
    // the layout of the last session, for the tabs open again (files gone are left out); the other
    // tabs then go into the active group.  Untracked: dockview is made once, not again for each tab
    // opened or setting changed.
    untrack(() => {
      const kept = app.keptLayout();
      const byFile = new Map(app.tabs.filter((t) => t.path).map((t) => [t.path as string, String(t.id)]));
      const panel = (id: string) => ({ id, contentComponent: "canvas", tabComponent: "tab", title: tabOf(id)?.name ?? "" });
      const layout = kept && restoreLayout(kept, (file) => byFile.get(file) ?? null, panel);
      if (layout && !groups.restore(layout, host.clientWidth, host.clientHeight)) dv.clear();
    });
    app.groups = groups;
    api = dv;
    return () => {
      ro.disconnect();
      app.groups = null;
      dv.dispose();
    };
  });

  // the tabs: a panel each, new ones in the active group; closed ones go; the current one is shown
  $effect(() => {
    if (!api) return;
    const ids = new Set(app.tabs.map((t) => String(t.id)));
    for (const p of [...api.panels]) if (!ids.has(p.id)) api.removePanel(p);
    for (const t of app.tabs) {
      if (!api.getPanel(String(t.id))) api.addPanel({ id: String(t.id), component: "canvas", tabComponent: "tab", title: t.name });
    }
    const cur = api.getPanel(String(app.currentId));
    if (cur && api.activePanel !== cur) cur.api.setActive();
  });

  // -- dragging a tab -------------------------------------------------------------------
  type Target = { group: DockviewGroupPanel; side: Side; index?: number; rect: DOMRect };
  let press: { id: number; x: number; y: number; moved: boolean } | null = null;
  let target = $state<Target | null>(null);

  function targetAt(x: number, y: number): Target | null {
    const group = api?.groups.find((g) => {
      const r = g.element.getBoundingClientRect();
      return x >= r.left && x < r.right && y >= r.top && y < r.bottom;
    });
    if (!group) return null;
    const bar = group.element.querySelector(".dv-tabs-and-actions-container")?.getBoundingClientRect();
    if (bar && y < bar.bottom) {
      // onto the tab bar: before the tab under the pointer (its left half), or after the last
      const tabs = [...group.element.querySelectorAll<HTMLElement>(".dv-tab")].map((el) => el.getBoundingClientRect());
      let index = tabs.findIndex((r) => x < r.left + r.width / 2);
      if (index < 0) index = tabs.length;
      const lineX = index < tabs.length ? tabs[index].left : (tabs[tabs.length - 1]?.right ?? bar.left);
      return { group, side: "center", index, rect: new DOMRect(lineX - 1, bar.top, 3, bar.height) };
    }
    const r = group.element.getBoundingClientRect();
    const fx = (x - r.left) / r.width;
    const fy = (y - r.top) / r.height;
    const edge = Math.min(fx, 1 - fx, fy, 1 - fy);
    if (edge > 0.25) return { group, side: "center", rect: r };
    const side: Side = edge === fx ? "left" : edge === 1 - fx ? "right" : edge === fy ? "top" : "bottom";
    const half = new DOMRect(
      side === "right" ? r.left + r.width / 2 : r.left,
      side === "bottom" ? r.top + r.height / 2 : r.top,
      side === "left" || side === "right" ? r.width / 2 : r.width,
      side === "top" || side === "bottom" ? r.height / 2 : r.height,
    );
    return { group, side, rect: half };
  }

  // Mouse events, not pointer events (as the canvas): after a right click WebKit can keep the right
  // button as pressed, and then sends no more pointerdowns (seen through WebDriver, see ARCHITECTURE.md).
  function down(e: MouseEvent) {
    const el = (e.target as Element).closest<HTMLElement>(".gtab");
    if (!el || e.button !== 0 || (e.target as Element).closest(".close")) return;
    const tab = tabOf(el.dataset.tab ?? "");
    if (!tab) return;
    app.select(tab);
    press = { id: tab.id, x: e.clientX, y: e.clientY, moved: false };
    // dockview's tab takes the press for itself; the movement is followed on the window
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
  }

  function move(e: MouseEvent) {
    if (!press) return;
    if (!press.moved && Math.hypot(e.clientX - press.x, e.clientY - press.y) < 5) return;
    press.moved = true;
    target = targetAt(e.clientX, e.clientY);
  }

  function up() {
    window.removeEventListener("mousemove", move);
    window.removeEventListener("mouseup", up);
    const p = press;
    const t = target;
    press = null;
    target = null;
    if (p?.moved && t) app.groups?.move(p.id, t.group, t.side, t.index);
  }
</script>

<div class="groups" bind:this={host} onmousedowncapture={down} role="presentation" data-testid="groups"></div>
{#if target}
  <div class="drop" class:line={target.index !== undefined} style:left="{target.rect.left}px" style:top="{target.rect.top}px" style:width="{target.rect.width}px" style:height="{target.rect.height}px"></div>
{/if}

<style>
  .groups {
    position: absolute;
    inset: 0;
  }
  .drop {
    position: fixed;
    z-index: 40;
    pointer-events: none;
    background: color-mix(in srgb, var(--highlight) 22%, transparent);
    border: 2px solid var(--highlight);
  }
  .drop.line {
    background: var(--highlight);
    border: none;
  }
  /* our palette, also the KDE colours in System mode */
  .groups :global(.dockview-theme-erdtex) {
    --dv-group-view-background-color: var(--base);
    --dv-tabs-and-actions-container-background-color: var(--window);
    --dv-tabs-and-actions-container-height: 28px;
    --dv-tabs-and-actions-container-font-size: inherit;
    --dv-activegroup-visiblepanel-tab-background-color: var(--base);
    --dv-activegroup-visiblepanel-tab-color: var(--text);
    --dv-activegroup-hiddenpanel-tab-background-color: var(--window);
    --dv-activegroup-hiddenpanel-tab-color: var(--text);
    --dv-inactivegroup-visiblepanel-tab-background-color: var(--base);
    --dv-inactivegroup-visiblepanel-tab-color: var(--text);
    --dv-inactivegroup-hiddenpanel-tab-background-color: var(--window);
    --dv-inactivegroup-hiddenpanel-tab-color: var(--text);
    --dv-tab-divider-color: var(--mid);
    --dv-separator-border: var(--mid);
    --dv-active-sash-color: var(--highlight);
    --dv-paneview-active-outline-color: var(--highlight);
    --dv-border-radius: 0;
    --dv-tab-border-radius: 0;
    --dv-tab-margin: 0;
    --dv-tab-font-size: inherit;
    --dv-icon-hover-background-color: var(--button);
  }
  /* the current tab: underlined in the active group */
  .groups :global(.dv-groupview.dv-active-group .dv-tab.dv-active-tab) {
    box-shadow: inset 0 -2px 0 var(--highlight);
  }
  .groups :global(.dv-tab) {
    border-right: 1px solid var(--mid);
  }
</style>
