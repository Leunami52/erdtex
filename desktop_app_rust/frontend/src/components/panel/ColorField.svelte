<script lang="ts">
  // A colour: swatches with plain names (the LaTeX value is the tooltip); "Other…" takes any
  // xcolor expression, with a preview.  After colorpicker.py.
  import { color_rgb } from "../../lib/core";
  import type { Swatch } from "../../lib/panel";

  let {
    id,
    swatches,
    current,
    tinted,
    commit,
  }: { id: string; swatches: Swatch[]; current: string; tinted: boolean; commit: (v: string) => void } = $props();

  let open = $state(false);
  let other = $state<string | null>(null);
  let input: HTMLInputElement | undefined = $state();
  const shown = $derived(swatches.find((s) => s.value === current) ?? swatches[0]);
  const preview = $derived(other === null || other.trim() === "" ? "" : color_rgb(other, tinted));
  const ok = $derived(other !== null && (other.trim() === "" || other.trim() === "none" || preview !== ""));

  function choose(v: string) {
    open = false;
    if (v !== current) commit(v);
  }

  function startOther() {
    open = false;
    other = current;
    queueMicrotask(() => input?.focus());
  }

  function finishOther(take: boolean) {
    const v = other?.trim() ?? "";
    other = null;
    if (take && v !== current) commit(v);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      e.preventDefault();
      if (other !== null) finishOther(false);
      open = false;
    } else if (e.key === "Enter" && other !== null && ok) {
      e.preventDefault();
      finishOther(true);
    }
  }
</script>

{#snippet chip(s: Swatch | undefined)}
  {#if s?.rgb}
    <span class="chip" style:background={s.rgb}></span>
  {:else}
    <span class="chip none"></span>
  {/if}
{/snippet}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="color" data-field={id} onkeydown={onKey}>
  <button class="shown" title={shown?.tip} onclick={() => (open = !open)} aria-haspopup="listbox" aria-expanded={open}>
    {@render chip(shown)}<span class="name">{shown?.name}</span><span class="arrow">▾</span>
  </button>
  {#if open}
    <div class="list" role="listbox">
      {#each swatches as s}
        <button role="option" aria-selected={s.value === current} title={s.tip} data-value={s.value} onclick={() => choose(s.value)}>
          {@render chip(s)}{s.name}
        </button>
      {/each}
      <button role="option" aria-selected="false" title="Enter any xcolor expression, e.g. orange!35" onclick={startOther}>Other…</button>
    </div>
  {/if}
  {#if other !== null}
    <div class="other">
      <p>An xcolor expression: a colour name, optionally mixed with a percentage (<i>red!30</i> = 30 % red, rest white; <i>red!30!black</i> = 30 % red, rest black).</p>
      <input bind:this={input} bind:value={other} placeholder="e.g. orange!35, blue!50!black, teal" />
      {#if preview}
        <div class="preview" style:background={preview}></div>
      {:else if !ok}
        <div class="bad">unknown colour</div>
      {/if}
      <div class="buttons">
        <button disabled={!ok} onclick={() => finishOther(true)}>OK</button>
        <button onclick={() => finishOther(false)}>Cancel</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .color {
    position: relative;
  }
  button {
    font: inherit;
    color: var(--text);
  }
  .shown {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 2px 6px;
    text-align: left;
  }
  .name {
    flex: 1;
  }
  .chip {
    display: inline-block;
    width: 13px;
    height: 13px;
    border: 1px solid rgb(110 110 110);
    border-radius: 3px;
    margin-right: 6px;
    vertical-align: -2px;
    flex: none;
  }
  .shown .chip {
    margin-right: 0;
  }
  .chip.none {
    background: linear-gradient(to top right, #fff 45%, rgb(200 0 0) 47%, rgb(200 0 0) 53%, #fff 55%);
  }
  .list {
    position: absolute;
    z-index: 20;
    left: 0;
    right: 0;
    max-height: 320px;
    overflow: auto;
    background: var(--base);
    border: 1px solid var(--mid);
    box-shadow: 0 4px 14px rgb(0 0 0 / 25%);
  }
  .list button {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    padding: 3px 8px;
  }
  .list button:hover {
    background: var(--highlight);
    color: #fff;
  }
  .other {
    margin-top: 4px;
    padding: 6px;
    border: 1px solid var(--mid);
    border-radius: 3px;
    background: var(--alternate);
  }
  .other p {
    margin: 0 0 6px;
    font-size: 12px;
  }
  .other input {
    width: 100%;
    box-sizing: border-box;
    font: inherit;
    color: var(--text);
    background: var(--base);
    border: 1px solid var(--mid);
  }
  .preview {
    height: 22px;
    margin-top: 4px;
    border: 1px solid #888;
  }
  .bad {
    color: #c00;
    margin-top: 4px;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 6px;
  }
  .buttons button {
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 2px 12px;
  }
</style>
