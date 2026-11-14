<script lang="ts">
  // Edit → Arrange Diagram… / Arrange Selection…: what is arranged (everything, only the selected
  // elements, or everything around the selected ones, which stay), tidy up (the layout stays
  // recognisable) or lay out anew (several attempts), and the gap between elements; how and the gap are
  // kept for next time (arrange/... settings, the gap also on Preferences' Editor page).
  import { untrack } from "svelte";
  import type { App } from "../../lib/app.svelte";

  let { app, close }: { app: App; close: () => void } = $props();
  const s = untrack(() => app.settings);
  let mode = $state(s["arrange/mode"] === "anew" ? "anew" : "tidy");
  let spacing = $state(Number(s["arrange/spacing"] ?? 0.5) || 0.5);
  const selected = untrack(() => app.current.selected);
  let scope = $state(untrack(() => (selected ? app.arrangeScope : "all")));
  let box: HTMLDivElement | undefined = $state();
  $effect(() => box?.focus());

  async function arrange() {
    spacing = Math.max(0.1, Math.min(3, Math.round(spacing * 20) / 20 || 0.5));
    await app.setSetting("arrange/mode", mode);
    await app.setSetting("arrange/spacing", String(spacing));
    close();
    await app.arrange({ anew: mode === "anew", spacing, scope });
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape" && !app.question) {
      e.preventDefault();
      close();
    } else if (e.key === "Enter") {
      e.preventDefault();
      void arrange();
    }
  }
</script>

<div class="backdrop" role="presentation">
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Arrange Diagram" tabindex="-1" bind:this={box} onkeydown={onKey} data-testid="arrange">
    <div class="title">Arrange Diagram</div>
    <div class="form">
      <span>What</span>
      <div>
        <label><input type="radio" name="ar-scope" value="all" bind:group={scope} data-testid="arrange-all" /> The whole diagram</label>
        <label class:off={!selected}
          ><input type="radio" name="ar-scope" value="selection" bind:group={scope} disabled={!selected} data-testid="arrange-selection" /> Only the selected elements: nothing
          else moves</label
        >
        <label class:off={!selected}
          ><input type="radio" name="ar-scope" value="pin" bind:group={scope} disabled={!selected} data-testid="arrange-pin" /> Everything but the selected elements: they
          stay where they are</label
        >
      </div>
      <span>How</span>
      <div>
        <label><input type="radio" name="ar-mode" value="tidy" bind:group={mode} data-testid="arrange-tidy" /> Tidy up: overlaps removed, attribute lists turned, lines kept clear; the layout stays recognisable</label>
        <label><input type="radio" name="ar-mode" value="anew" bind:group={mode} data-testid="arrange-anew" /> Lay out anew: several fresh attempts as well (supertypes above their subtypes)</label>
      </div>
      <label for="ar-gap">Gap between elements</label>
      <span class="row"><input id="ar-gap" type="number" min="0.1" max="3" step="0.1" bind:value={spacing} data-testid="arrange-gap" /> cm</span>
      <span></span>
      <span class="note">The best of the attempts is taken, measured by overlaps, lines through elements, crossings, line length and size. The result can be undone.</span>
    </div>
    <div class="buttons">
      <button onclick={arrange} data-testid="arrange-ok">Arrange</button>
      <button onclick={close}>Cancel</button>
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
    max-width: 640px;
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
  .form {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 8px 12px;
    align-items: center;
    padding: 12px;
  }
  .form label {
    display: block;
  }
  .row {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .note,
  .off {
    color: var(--disabled);
  }
  input[type="number"] {
    width: 70px;
    font: inherit;
    color: var(--text);
    background: var(--base);
    border: 1px solid var(--mid);
    border-radius: 3px;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 8px 12px;
    border-top: 1px solid var(--mid);
  }
  button {
    font: inherit;
    color: var(--text);
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 4px;
    padding: 4px 14px;
  }
</style>
