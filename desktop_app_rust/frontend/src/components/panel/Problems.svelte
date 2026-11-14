<script lang="ts">
  // The Problems dock: what the rules find and what the layout couldn't draw, with their fixes.
  // A click shows the element; the context menu offers its fix and opens its rule in the rule books.
  import type { App } from "../../lib/app.svelte";
  import { SEVERITY_COLORS } from "../../lib/panel";
  import { centreOn } from "../../lib/view";
  import type { MenuEntry } from "../ContextMenu.svelte";

  interface Listed {
    severity: "error" | "warning" | "hint";
    text: string;
    fix: string | null;
    /** the rule in the rule books, empty where none is written yet */
    rule: string;
  }

  let { app }: { app: App } = $props();
  let current = $state(-1);

  const data = $derived.by(() => {
    const t = app.current;
    void t.version;
    void app.showHints;
    return JSON.parse(t.diagram.problems()) as { summary: string; count: number; rows: Listed[] };
  });
  const fixable = $derived(data.rows.filter((r) => r.fix).length);

  $effect(() => {
    void data;
    if (current >= data.rows.length) current = -1;
  });

  function show(k: number) {
    current = k;
    const tab = app.current;
    const at = app.edit((d) => d.goto_problem(k));
    if (at.length === 2) tab.view = centreOn(tab.view, at[0], at[1], tab.size.w, tab.size.h);
  }

  function fix(k: number) {
    app.edit((d) => d.fix_problem(k));
  }

  function fixAll() {
    const n = app.edit((d) => d.fix_all());
    if (n) app.message(`Applied ${n} fixes.`, 4000);
  }

  function onMenu(e: MouseEvent, k: number) {
    e.preventDefault();
    current = k;
    const items: MenuEntry[] = [{ label: "Show element", action: () => show(k) }];
    const f = data.rows[k]?.fix;
    if (f) items.push({ label: f, action: () => fix(k) });
    const rule = data.rows[k]?.rule;
    if (rule) items.push({ label: `Rule ${rule} (Rule Books)`, action: () => app.openRule(rule) });
    app.menu = { x: e.clientX, y: e.clientY, items };
  }
</script>

<section class="problems" data-testid="problems">
  <div class="bar">
    <span class="summary" data-testid="problem-summary">{data.summary}</span>
    <button title="Apply the suggested fix to the selected problem" disabled={!(current >= 0 && data.rows[current]?.fix)} onclick={() => fix(current)}>Fix</button>
    <button title="Apply all suggested fixes" disabled={!fixable} data-testid="fix-all" onclick={fixAll}>{fixable ? `Fix all (${fixable})` : "Fix all"}</button>
    <label title="Style suggestions such as entities without a key attribute">
      <input type="checkbox" checked={app.showHints} onchange={(e) => app.setShowHints(e.currentTarget.checked)} /> Show hints
    </label>
  </div>
  <ul role="listbox" aria-label="Problems">
    {#each data.rows as r, k}
      <li role="option" aria-selected={k === current} class:current={k === current}>
        <button onclick={() => show(k)} oncontextmenu={(e) => onMenu(e, k)}>
          <span class="mark" style:background={SEVERITY_COLORS[r.severity]}>{r.severity === "hint" ? "i" : "!"}</span>
          <span class="message">{r.text}</span>
        </button>
        {#if r.rule}
          <button class="rule" title="Open rule {r.rule} in the rule books" onclick={() => app.openRule(r.rule)} data-rule={r.rule}>{r.rule}</button>
        {/if}
      </li>
    {/each}
  </ul>
</section>

<style>
  .problems {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    color: var(--text);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 6px;
    border-bottom: 1px solid var(--mid);
  }
  .summary {
    flex: 1;
  }
  .bar button {
    font: inherit;
    color: var(--text);
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 1px 10px;
  }
  .bar button:disabled {
    color: var(--disabled);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow: auto;
    flex: 1;
    background: var(--base);
  }
  li {
    display: flex;
    align-items: center;
  }
  li button.rule {
    width: auto;
    flex: none;
    font-family: monospace;
    color: var(--highlight);
    text-decoration: underline;
    padding: 0 8px;
  }
  .message {
    flex: 1;
  }
  li button {
    display: flex;
    gap: 6px;
    align-items: center;
    width: 100%;
    font: inherit;
    color: var(--text);
    background: none;
    border: 0;
    padding: 2px 6px;
    text-align: left;
  }
  li.current button {
    background: var(--highlight);
    color: #fff;
  }
  .mark {
    flex: none;
    display: inline-grid;
    place-items: center;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    color: #fff;
    font-weight: bold;
    font-size: 10px;
  }
</style>
