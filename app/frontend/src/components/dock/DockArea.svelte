<script lang="ts">
  // The panes docked at one side, one after the other (side by side at the bottom), with a
  // splitter between two unfolded ones.  When all of them are folded, the side is a narrow strip
  // with a button for each.
  import type { Snippet } from "svelte";
  import type { Area, Docks, PaneId } from "../../lib/docks.svelte";
  import DockPanel from "./DockPanel.svelte";

  let { docks, area, title, content }: { docks: Docks; area: Area; title: (id: PaneId) => string; content: Snippet<[PaneId]> } = $props();
  const ids = $derived(docks.docked(area));
  const across = $derived(area === "bottom");
  let box: HTMLDivElement | undefined = $state();

  function split(e: PointerEvent, a: PaneId, b: PaneId) {
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const size = across ? box!.clientWidth : box!.clientHeight;
    const start = across ? e.clientX : e.clientY;
    const [wa, wb] = [docks.pane(a).weight, docks.pane(b).weight];
    const total = ids.filter((i) => !docks.pane(i).collapsed).reduce((s, i) => s + docks.pane(i).weight, 0);
    const move = (m: PointerEvent) => {
      const d = (((across ? m.clientX : m.clientY) - start) / Math.max(1, size)) * total;
      const na = Math.max(0.1, wa + d);
      const nb = Math.max(0.1, wa + wb - na);
      docks.set(a, { weight: na });
      docks.set(b, { weight: nb });
    };
    const up = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
  }
</script>

{#if docks.folded(area) && !across}
  <div class="strip" data-area={area}>
    {#each ids as id}
      <button title="Unfold {title(id)}" data-unfold={id} onclick={() => docks.set(id, { collapsed: false })}><span>{title(id)}</span></button>
    {/each}
  </div>
{:else}
  <div class="area" class:across bind:this={box} data-area={area}>
    {#each ids as id, k (id)}
      {#if k > 0 && !docks.pane(ids[k - 1]).collapsed && !docks.pane(id).collapsed}
        <div class="split" class:v={across} role="separator" onpointerdown={(e) => split(e, ids[k - 1], id)}></div>
      {/if}
      <div class="slot" style:flex={docks.pane(id).collapsed ? "0 0 auto" : `${docks.pane(id).weight} 1 0`}>
        <DockPanel {docks} {id} title={title(id)}>{@render content(id)}</DockPanel>
      </div>
    {/each}
  </div>
{/if}

<style>
  .area {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    min-height: 0;
  }
  .area.across {
    flex-direction: row;
  }
  .slot {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
  }
  .slot > :global(.pane) {
    flex: 1;
  }
  .split {
    flex: none;
    height: 4px;
    background: var(--mid);
    cursor: ns-resize;
  }
  .split.v {
    height: auto;
    width: 4px;
    cursor: ew-resize;
  }
  .strip {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 24px;
    height: 100%;
    padding: 4px 0;
    background: var(--alternate);
  }
  .strip button {
    writing-mode: vertical-rl;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 8px 0;
    cursor: pointer;
  }
</style>
