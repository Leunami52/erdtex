<script lang="ts">
  // A tab of a group, drawn by us inside dockview's tab bar: its name, × to close it (also the middle
  // button), and its menu.  Pressing and dragging it is Groups.svelte's.
  import type { App, Tab } from "../../lib/app.svelte";
  import type { MenuEntry } from "../ContextMenu.svelte";

  let { app, tab }: { app: App; tab: Tab } = $props();

  /** Closes the tabs one after the other; stops where one is kept (Cancel on unsaved changes). */
  async function closeAll(ids: number[]) {
    for (const id of ids) {
      const t = app.tabs.find((x) => x.id === id);
      if (t && !(await app.closeTab(t))) return;
    }
  }

  function openMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    const g = app.groups;
    const siblings = g?.siblings(tab.id) ?? [tab.id];
    const i = siblings.indexOf(tab.id);
    const items: MenuEntry[] = [
      { label: "Close Tab", action: () => app.closeTab(tab) },
      { label: "Close Other Tabs", action: () => closeAll(siblings.filter((id) => id !== tab.id)) },
      { label: "Close Tabs to the Right", action: () => closeAll(siblings.slice(i + 1)) },
    ];
    if (g?.canSplit(tab.id)) {
      items.push("-", { label: "Split Right", action: () => app.splitTab("right", tab) }, { label: "Split Down", action: () => app.splitTab("bottom", tab) });
    }
    if (g?.canMoveToOther(tab.id)) {
      if (!g.canSplit(tab.id)) items.push("-");
      items.push({ label: "Move to Other Group", action: () => app.moveTabToOtherGroup(tab) });
    }
    app.menu = { x: e.clientX, y: e.clientY, items };
  }
</script>

<div
  class="gtab"
  data-tab={tab.id}
  title={`${tab.kind.toUpperCase()} diagram${tab.path ? ": " + tab.path : ""}`}
  role="tab"
  tabindex="-1"
  aria-selected={tab.id === app.currentId}
  onauxclick={(e) => e.button === 1 && app.closeTab(tab)}
  oncontextmenu={openMenu}
>
  <span>{tab.modified ? "*" : ""}{tab.name}</span>
  <button
    class="close"
    title="Close tab"
    onclick={(e) => {
      e.stopPropagation();
      app.closeTab(tab);
    }}>×</button
  >
</div>

<style>
  .gtab {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 100%;
    padding: 0 6px 0 12px;
    color: var(--text);
    white-space: nowrap;
    user-select: none;
    cursor: default;
  }
  .close {
    font: inherit;
    color: inherit;
    background: none;
    border: none;
    border-radius: 3px;
    padding: 0 4px;
    opacity: 0.6;
  }
  .close:hover {
    opacity: 1;
    background: var(--button);
  }
</style>
