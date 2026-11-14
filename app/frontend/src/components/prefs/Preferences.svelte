<script lang="ts">
  import { untrack } from "svelte";
  // One window for all settings of the app (after preferences.py).  Nothing here is stored in a
  // diagram, except with "Use for the open diagram".  The pages keep what was changed while
  // switching between them; OK and Apply check every page first, then store.
  import type { App } from "../../lib/app.svelte";
  import type { Command } from "../../lib/keys";
  import AppearancePage from "./AppearancePage.svelte";
  import DefaultsPage from "./DefaultsPage.svelte";
  import EditorPage from "./EditorPage.svelte";
  import OutputPage from "./OutputPage.svelte";
  import ShortcutsPage from "./ShortcutsPage.svelte";

  interface Page {
    problem(): string | null;
    apply(): Promise<void>;
  }

  let { app, commands, close, applied }: { app: App; commands: Command[]; close: () => void; applied: () => void } = $props();
  const app0 = untrack(() => app);
  const TITLES = ["Appearance", "Keyboard shortcuts", "New ER diagrams", "New GSM diagrams", "Editor", "Output"];
  let page = $state((Number(app0.settings["preferences page"] ?? 0) || 0) % TITLES.length);
  const pages: (Page | undefined)[] = $state([]);
  let box: HTMLDivElement | undefined = $state();
  $effect(() => box?.focus());

  async function apply(): Promise<boolean> {
    for (const [i, p] of pages.entries()) {
      const wrong = p?.problem();
      if (wrong) {
        page = i;
        await app.alert(wrong);
        return false;
      }
    }
    for (const p of pages) await p?.apply();
    applied();
    return true;
  }

  async function ok() {
    if (await apply()) {
      await app.setSetting("preferences page", String(page));
      close();
    }
  }

  function onKey(e: KeyboardEvent) {
    e.stopPropagation(); // the dialog keeps its keys
    if (e.key === "Escape" && !app.question) {
      e.preventDefault();
      close();
    }
  }
</script>

<div class="backdrop" role="presentation">
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Preferences" tabindex="-1" bind:this={box} onkeydown={onKey} data-testid="preferences">
    <div class="title">Preferences</div>
    <div class="main">
      <ul class="names" role="listbox" aria-label="Pages">
        {#each TITLES as t, i}
          <li><button role="option" aria-selected={page === i} class:on={page === i} data-page={i} onclick={() => (page = i)}>{t}</button></li>
        {/each}
      </ul>
      <div class="pages">
        <div class="page" class:shown={page === 0}><AppearancePage {app} bind:this={pages[0]} /></div>
        <div class="page" class:shown={page === 1}><ShortcutsPage {app} {commands} bind:this={pages[1]} /></div>
        <div class="page" class:shown={page === 2}><DefaultsPage {app} kind="er" bind:this={pages[2]} /></div>
        <div class="page" class:shown={page === 3}><DefaultsPage {app} kind="gsm" bind:this={pages[3]} /></div>
        <div class="page" class:shown={page === 4}><EditorPage {app} bind:this={pages[4]} /></div>
        <div class="page" class:shown={page === 5}><OutputPage {app} bind:this={pages[5]} /></div>
      </div>
    </div>
    <div class="buttons">
      <button onclick={ok} data-testid="prefs-ok">OK</button>
      <button onclick={close} data-testid="prefs-cancel">Cancel</button>
      <button onclick={apply} data-testid="prefs-apply">Apply</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 30%);
    display: grid;
    place-items: center;
    z-index: 40;
  }
  .dialog {
    display: flex;
    flex-direction: column;
    width: min(860px, 94vw);
    height: min(640px, 92vh);
    background: var(--window);
    color: var(--text);
    border: 1px solid var(--mid);
    border-radius: 6px;
    box-shadow: 0 8px 30px rgb(0 0 0 / 35%);
    outline: none;
  }
  .title {
    padding: 8px 12px;
    font-weight: 600;
    border-bottom: 1px solid var(--mid);
  }
  .main {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .names {
    list-style: none;
    margin: 8px;
    padding: 0;
    width: 180px;
    flex: none;
    background: var(--base);
    border: 1px solid var(--mid);
  }
  .names button {
    display: block;
    width: 100%;
    text-align: left;
    font: inherit;
    color: var(--text);
    background: none;
    border: 0;
    padding: 4px 8px;
  }
  .names button.on {
    background: var(--highlight);
    color: #fff;
  }
  .pages {
    flex: 1;
    min-width: 0;
    position: relative;
  }
  .page {
    display: none;
    position: absolute;
    inset: 8px 10px 0 2px;
    overflow: auto;
  }
  .page.shown {
    display: block;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 8px 12px;
    border-top: 1px solid var(--mid);
  }
  .buttons button {
    font: inherit;
    color: var(--text);
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 4px;
    padding: 4px 16px;
  }
  /* what the pages share */
  .page :global(.form) {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 6px 10px;
    align-items: center;
  }
  .page :global(.form .span) {
    grid-column: 1 / -1;
  }
  .page :global(.form .field) {
    grid-column: 2;
  }
  .page :global(h4) {
    margin: 10px 0 2px;
    font-size: 13px;
  }
  .page :global(p) {
    margin: 0 0 8px;
  }
  .page :global(input:not([type="checkbox"])),
  .page :global(select),
  .page :global(textarea) {
    font: inherit;
    color: var(--text);
    background: var(--base);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 2px 4px;
    box-sizing: border-box;
  }
  .page :global(input::placeholder) {
    color: var(--placeholder);
  }
  .page :global(button) {
    font: inherit;
    color: var(--text);
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 2px 10px;
  }
  .page :global(button:disabled) {
    color: var(--disabled);
  }
</style>
