<script lang="ts">
  // The main toolbar, as in the Python app: files, undo, zoom, grid, and grid snapping.
  import type { App } from "../lib/app.svelte";
  import { actionIcon } from "../lib/actionIcons";
  import { SNAPS } from "../lib/view";

  let { app, run, keysOf }: { app: App; run: (id: string) => void; keysOf: (id: string) => string } = $props();

  const TITLES: Record<string, string> = {
    a_new_er: "New ER diagram", a_new_gsm: "New GSM diagram", a_open: "Open", a_save: "Save", a_undo: "Undo", a_redo: "Redo",
    a_zoom_out: "Zoom out", a_zoom_100: "Actual size", a_zoom_in: "Zoom in", a_fit: "Fit diagram", a_grid: "Show grid",
  };
  const GROUPS = [["a_new_er", "a_new_gsm", "a_open", "a_save"], ["a_undo", "a_redo"], ["a_zoom_out", "a_zoom_100", "a_zoom_in", "a_fit", "a_grid"]];

  const title = (id: string) => {
    void app.current.ui;
    if (id === "a_undo" || id === "a_redo") {
      const what = id === "a_undo" ? app.current.diagram.undo_label() : app.current.diagram.redo_label();
      return `${TITLES[id]}${what ? " " + what : ""}`;
    }
    const keys = keysOf(id);
    return keys ? `${TITLES[id]} (${keys})` : TITLES[id];
  };
  const disabled = (id: string) => {
    void app.current.ui;
    if (id === "a_undo") return !app.current.diagram.undo_label();
    if (id === "a_redo") return !app.current.diagram.redo_label();
    return false;
  };
</script>

<div class="toolbar" data-testid="toolbar">
  {#each GROUPS as group, g}
    {#if g}<span class="sep"></span>{/if}
    {#each group as id}
      <button title={title(id)} aria-label={TITLES[id]} disabled={disabled(id)} class:on={id === "a_grid" && app.showGrid} onclick={() => run(id)}>
        {@html actionIcon(id, 18)}
      </button>
    {/each}
  {/each}
  <span class="sep"></span>
  <label title="Grid snapping (hold Alt while dragging to place freely)">
    Snap:
    <select value={String(app.snap)} onchange={(e) => app.setSnap(Number((e.target as HTMLSelectElement).value))} data-testid="snap">
      {#each SNAPS as [text, v]}<option value={String(v)}>{text}</option>{/each}
    </select>
  </label>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px 6px;
    background: var(--window);
    color: var(--text);
    border-bottom: 1px solid var(--mid);
    user-select: none;
  }
  button {
    display: grid;
    place-items: center;
    width: 28px;
    height: 26px;
    color: inherit;
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
  }
  button:hover:not(:disabled) {
    background: var(--button);
  }
  button:disabled {
    color: var(--disabled);
  }
  button.on {
    background: var(--button);
    border-color: var(--mid);
  }
  .sep {
    width: 1px;
    height: 18px;
    margin: 0 6px;
    background: var(--mid);
  }
  select {
    font: inherit;
    color: var(--text);
    background: var(--base);
    border: 1px solid var(--mid);
    border-radius: 3px;
  }
</style>
