<script lang="ts">
  import type { App } from "../lib/app.svelte";
  import { zoomOf } from "../lib/view";

  let { app }: { app: App } = $props();
  // a message that names a rule (a refused connection: "… (DRAW-1)") links it to the rule books
  const parts = $derived(app.status.split(/\b((?:ER|GSM|DRAW|G2E|REL|NOT)-\d+)\b/));
</script>

<footer class="status" data-testid="status">
  <span class="message"
    >{#each parts as p, i}{#if i % 2}<button class="rule" title="Open rule {p} in the rule books" onclick={() => app.openRule(p)} data-rule={p}>{p}</button
        >{:else}{p}{/if}{/each}</span
  >
  <span>{app.current?.kind === "gsm" ? "GSM diagram" : "ER diagram"}</span>
  <span class="pos">{app.cursor}</span>
  <span class="zoom">{app.current ? (zoomOf(app.current.view) * 100).toFixed(0) : 100} %</span>
</footer>

<style>
  .status {
    display: flex;
    gap: 14px;
    padding: 2px 10px;
    background: var(--window);
    color: var(--text);
    border-top: 1px solid var(--mid);
    white-space: nowrap;
  }
  .message {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .rule {
    font: inherit;
    font-family: monospace;
    color: var(--highlight);
    text-decoration: underline;
    background: none;
    border: 0;
    padding: 0;
    cursor: pointer;
  }
  .pos {
    min-width: 150px;
    text-align: right;
  }
  .zoom {
    min-width: 50px;
    text-align: right;
  }
</style>
