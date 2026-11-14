<script lang="ts">
  import { untrack } from "svelte";
  // Every command with its keys, by group.  Pick one, press the new keys; a combination another
  // command uses is refused and that command named.  ER and GSM tools may share keys.
  import type { App } from "../../lib/app.svelte";
  import { canonical_keys, shortcut_conflict } from "../../lib/core";
  import { combos, display, settingOf, type Command } from "../../lib/keys";

  let { app, commands }: { app: App; commands: Command[] } = $props();
  const app0 = untrack(() => app);
  const commands0 = untrack(() => commands);
  const draft = $state<Record<string, string>>(Object.fromEntries(commands0.map((c) => [c.id, app0.bindings?.keys.get(c.id) ?? c.keys])));
  let current = $state<string | null>(null);
  let search = $state("");
  let message = $state("");
  const groups = [...new Set(commands0.map((c) => c.group))];
  const shown = (c: Command) => {
    const q = search.trim().toLowerCase();
    return !q || c.title.toLowerCase().includes(q) || display(draft[c.id]).toLowerCase().includes(q);
  };

  export function problem(): string | null {
    return null;
  }

  export async function apply(): Promise<void> {
    for (const c of commands) await app.setSetting(settingOf(c.id), draft[c.id] === c.keys ? null : draft[c.id]);
  }

  function assign(keys: string) {
    if (!current || keys === draft[current]) return;
    const ids = Object.keys(draft);
    const other = keys ? shortcut_conflict(current, keys, ids, ids.map((i) => draft[i])) : "";
    if (other) {
      message = `${display(keys)} is already used for “${other}”. Change that one first.`;
      return;
    }
    message = "";
    draft[current] = keys;
  }

  function capture(e: KeyboardEvent) {
    if (e.key === "Escape" || e.key === "Tab") return; // the dialog's
    e.preventDefault();
    e.stopPropagation();
    const c = combos(e)[0];
    const k = c ? canonical_keys(c) : "";
    if (k) assign(k);
  }

  function resetAll() {
    for (const c of commands) draft[c.id] = c.keys;
    message = "";
  }

  const defaultOf = (id: string | null) => commands.find((c) => c.id === id)?.keys ?? "";
</script>

<div class="shortcuts" data-testid="shortcuts">
  <input class="search" placeholder="Search" bind:value={search} data-testid="shortcut-search" />
  <div class="tree" role="tree">
    {#each groups as g}
      <div class="group">{g}</div>
      {#each commands.filter((c) => c.group === g && shown(c)) as c}
        <button role="treeitem" aria-selected={current === c.id} class:on={current === c.id} data-id={c.id} onclick={() => (current = c.id)}>
          <span>{c.title}</span><span class="keys">{display(draft[c.id])}</span>
        </button>
      {/each}
    {/each}
  </div>
  <div class="row">
    <span>Press the new shortcut:</span>
    <input class="capture" readonly disabled={!current} value={current ? display(draft[current]) : ""} onkeydown={capture} data-testid="shortcut-capture" />
    <button disabled={!current} onclick={() => assign("")}>None</button>
    <button disabled={!current} onclick={() => assign(defaultOf(current))}>Default</button>
    <button onclick={resetAll}>Reset all</button>
  </div>
  <div class="message" data-testid="shortcut-message">{message}</div>
</div>

<style>
  .shortcuts {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 6px;
  }
  .search {
    width: 100%;
  }
  .tree {
    flex: 1;
    min-height: 200px;
    overflow: auto;
    background: var(--base);
    border: 1px solid var(--mid);
  }
  .group {
    font-weight: 600;
    padding: 3px 6px;
    background: var(--alternate);
  }
  .tree button {
    display: flex;
    width: 100%;
    border: 0 !important;
    border-radius: 0 !important;
    background: none !important;
    text-align: left;
    padding: 2px 6px 2px 20px !important;
  }
  .tree button.on {
    background: var(--highlight) !important;
    color: #fff !important;
  }
  .tree button span:first-child {
    flex: 1;
  }
  .keys {
    min-width: 120px;
  }
  .row {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .capture {
    flex: 1;
  }
  .message {
    min-height: 1.2em;
    color: #c00;
  }
</style>
