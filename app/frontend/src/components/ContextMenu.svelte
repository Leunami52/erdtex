<script lang="ts" module>
  export type MenuEntry = { label: string; action: () => unknown } | "-";
</script>

<script lang="ts">
  // The canvas's context menu, at the pointer.  Esc or a click elsewhere closes it.
  let { x, y, items, close }: { x: number; y: number; items: MenuEntry[]; close: () => void } = $props();
</script>

<svelte:window
  onkeydown={(e) => e.key === "Escape" && (e.stopPropagation(), close())}
  onpointerdown={(e) => !(e.target as Element).closest?.(".context") && close()}
/>

<div class="context" style:left="{x}px" style:top="{y}px" role="menu" data-testid="context-menu">
  {#each items as it}
    {#if it === "-"}
      <hr />
    {:else}
      <button role="menuitem" onclick={() => (close(), it.action())}>{it.label}</button>
    {/if}
  {/each}
</div>

<style>
  .context {
    position: fixed;
    min-width: 160px;
    background: var(--base);
    color: var(--text);
    border: 1px solid var(--mid);
    box-shadow: 0 3px 10px rgb(0 0 0 / 25%);
    padding: 3px 0;
    z-index: 30;
  }
  button {
    display: block;
    width: 100%;
    font: inherit;
    color: inherit;
    background: none;
    border: none;
    text-align: left;
    padding: 4px 14px;
  }
  button:hover {
    background: var(--highlight);
    color: #fff;
  }
  hr {
    border: none;
    border-top: 1px solid var(--mid);
    margin: 3px 0;
  }
</style>
