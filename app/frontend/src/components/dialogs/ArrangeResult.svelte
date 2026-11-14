<script lang="ts">
  // After arranging: what changed (the figures before and after), Keep or Undo. Gone with the next
  // change of the diagram or another tab.
  import type { App } from "../../lib/app.svelte";

  let { app }: { app: App } = $props();
  const r = $derived(app.arrangeResult);
  const pair = (a: number, b: number) => (a === b ? `${a}` : `${a} → ${b}`);
  $effect(() => {
    const res = app.arrangeResult;
    if (res && (app.current.id !== res.tab || app.current.version !== res.version)) app.arrangeResult = null;
  });
</script>

{#if r}
  <div class="bar" role="status" data-testid="arrange-result">
    <span>
      Arranged ({r.name}): overlaps {pair(r.before.overlaps, r.after.overlaps)}, lines through elements
      {pair(r.before.through, r.after.through)}, lines on top of one another {pair(r.before.close, r.after.close)}, crossings {pair(r.before.crossings, r.after.crossings)}, size
      {r.after.width.toFixed(0)} × {r.after.height.toFixed(0)} cm.
    </span>
    <button onclick={() => (app.arrangeResult = null)} data-testid="arrange-keep">Keep</button>
    <button
      onclick={() => {
        app.arrangeResult = null;
        app.undo();
      }}
      data-testid="arrange-undo">Undo</button
    >
  </div>
{/if}

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    background: var(--base);
    color: var(--text);
    border-top: 1px solid var(--mid);
  }
  span {
    flex: 1;
  }
  button {
    font: inherit;
    color: var(--text);
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 4px;
    padding: 2px 12px;
  }
</style>
