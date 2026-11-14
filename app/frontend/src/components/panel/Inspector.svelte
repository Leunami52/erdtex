<script lang="ts">
  // The Element dock: the properties of the selection, and its source text with its problems.
  import type { App } from "../../lib/app.svelte";
  import Properties from "./Properties.svelte";

  let { app }: { app: App } = $props();
  let page = $state<"properties" | "source">("properties");
  const source = $derived.by(() => {
    const t = app.current;
    void t.version;
    void t.sel;
    return page === "source" ? t.diagram.source() : "";
  });
</script>

<section class="inspector" data-testid="inspector">
  <div class="tabs" role="tablist">
    <button role="tab" aria-selected={page === "properties"} class:on={page === "properties"} onclick={() => (page = "properties")}>Properties</button>
    <button role="tab" aria-selected={page === "source"} class:on={page === "source"} onclick={() => (page = "source")}>Source</button>
  </div>
  <div class="page">
    {#if page === "properties"}
      {#key app.currentId}
        <Properties {app} tab={app.current} />
      {/key}
    {:else}
      <textarea class="source" readonly value={source} data-testid="source" spellcheck="false"></textarea>
    {/if}
  </div>
</section>

<style>
  .inspector {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    color: var(--text);
  }
  .tabs {
    display: flex;
    gap: 2px;
    padding: 4px 6px 0;
    border-bottom: 1px solid var(--mid);
  }
  .tabs button {
    font: inherit;
    color: var(--text);
    background: transparent;
    border: 1px solid transparent;
    border-bottom: 0;
    border-radius: 4px 4px 0 0;
    padding: 3px 10px;
  }
  .tabs button.on {
    background: var(--base);
    border-color: var(--mid);
  }
  .page {
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: var(--window);
  }
  .source {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    resize: none;
    font: 12px monospace;
    color: var(--text);
    background: var(--base);
    border: 0;
    padding: 6px;
  }
</style>
