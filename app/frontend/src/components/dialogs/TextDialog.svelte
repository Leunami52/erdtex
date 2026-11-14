<script lang="ts">
  // A short text page in Markdown (Help → Privacy, Help → Imprint).
  import { Marked } from "marked";

  let { title, markdown, close }: { title: string; markdown: string; close: () => void } = $props();
  const html = $derived(new Marked({ gfm: true }).parse(markdown, { async: false }) as string);
  let box: HTMLDivElement | undefined = $state();
  $effect(() => box?.focus());
</script>

<div class="backdrop" role="presentation">
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="dialog" role="dialog" aria-modal="true" aria-label={title} tabindex="-1" bind:this={box} onkeydown={(e) => e.key === "Escape" && close()} data-testid="text-page">
    <div class="title">{title}</div>
    <div class="body">{@html html}</div>
    <div class="buttons"><button onclick={close} data-testid="text-page-close">Close</button></div>
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
    width: min(620px, calc(100vw - 40px));
    max-height: calc(100vh - 80px);
    display: flex;
    flex-direction: column;
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
  .body {
    padding: 4px 16px;
    overflow: auto;
    line-height: 1.45;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
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
