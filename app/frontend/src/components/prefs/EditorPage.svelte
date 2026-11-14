<script lang="ts">
  import { untrack } from "svelte";
  import type { App } from "../../lib/app.svelte";
  import { SNAPS } from "../../lib/view";

  let { app }: { app: App } = $props();
  const app0 = untrack(() => app);
  let snap = $state(String(app0.snap));
  let grid = $state(app0.showGrid);
  let hints = $state(app0.showHints);
  let gap = $state(Number(app0.settings["arrange/spacing"] ?? 0.5) || 0.5);

  export function problem(): string | null {
    return null;
  }

  export async function apply(): Promise<void> {
    await app.setSnap(Number(snap));
    if (grid !== app.showGrid) await app.toggleGrid();
    if (hints !== app.showHints) await app.setShowHints(hints);
    await app.setSetting("arrange/spacing", String(Math.max(0.1, Math.min(3, Math.round(gap * 20) / 20 || 0.5))));
  }
</script>

<div class="form">
  <label for="pref-snap">Snap to</label>
  <select id="pref-snap" bind:value={snap} data-testid="pref-snap">
    {#each SNAPS as [text, v]}<option value={String(v)}>{text}</option>{/each}
  </select>
  <label class="field"><input type="checkbox" bind:checked={grid} data-testid="pref-grid" /> Show the grid</label>
  <label class="field"><input type="checkbox" bind:checked={hints} data-testid="pref-hints" /> Show hints in the Problems list</label>
  <label for="pref-gap">Arranging: gap between elements</label>
  <span><input id="pref-gap" type="number" min="0.1" max="3" step="0.1" bind:value={gap} data-testid="pref-gap" /> cm</span>
</div>
