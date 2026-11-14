<script lang="ts">
  // A question or a message, with buttons.  Enter takes the first button, Esc "Cancel" (or the
  // last one).
  import type { Question } from "../lib/app.svelte";

  let { q }: { q: Question } = $props();
  let first: HTMLButtonElement | undefined = $state();
  $effect(() => first?.focus());

  function onKey(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Escape") {
      e.preventDefault();
      q.answer(q.buttons.includes("Cancel") ? "Cancel" : q.buttons[q.buttons.length - 1]);
    }
  }
</script>

<div class="backdrop" role="presentation">
  <div class="box" role="alertdialog" aria-modal="true" tabindex="-1" onkeydown={onKey} data-testid="question">
    <p>{q.text}</p>
    <div class="buttons">
      {#each q.buttons as b, i}
        {#if i === 0}
          <button bind:this={first} onclick={() => q.answer(b)}>{b}</button>
        {:else}
          <button onclick={() => q.answer(b)}>{b}</button>
        {/if}
      {/each}
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
    z-index: 50;
  }
  .box {
    max-width: min(560px, 90vw);
    background: var(--window);
    color: var(--text);
    border: 1px solid var(--mid);
    border-radius: 6px;
    padding: 16px 18px 12px;
    box-shadow: 0 8px 30px rgb(0 0 0 / 35%);
  }
  p {
    white-space: pre-wrap;
    margin: 0 0 14px;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  button {
    font: inherit;
    color: var(--text);
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 4px;
    padding: 4px 14px;
  }
  button:focus-visible {
    outline: 2px solid var(--highlight);
  }
</style>
