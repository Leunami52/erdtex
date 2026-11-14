<script lang="ts">
  // The cardinality of an edge: only the choices of the notation in use; below them, what the
  // value means with the real names; and the (discouraged) change of notation for the whole
  // relationship.  After cardpicker.py.
  import type { Outcome } from "../../lib/app.svelte";
  import { changed, orig, type CardField } from "../../lib/panel";

  let { c, set }: { c: CardField; set: (id: string, value: string) => Promise<Outcome> } = $props();
  let invalid = $state<string | null>(null);
  let lo: HTMLInputElement | undefined = $state();
  let hi: HTMLInputElement | undefined = $state();

  $effect(() => {
    void c;
    invalid = null;
  });

  async function minMax() {
    if (!lo || !hi) return;
    const a = lo.value.trim();
    const b = hi.value.trim();
    // both remembered, so that a value that's refused can be corrected
    changed(lo);
    changed(hi);
    if (!a || !b) return;
    const out = await set("card.minmax", `${a},${b}`);
    invalid = out.kind === "invalid" ? out.text : null;
  }

  const onEnter = (e: KeyboardEvent) => e.key === "Enter" && (e.preventDefault(), void minMax());
</script>

<div class="card" data-field="card">
  <div class="row">
    {#if c.notation === "minmax"}
      <span>(</span>
      <input
        bind:this={lo}
        use:orig={c.lo}
        list="card-lo"
        class="num"
        data-part="lo"
        title="The minimum: 0, 1, any positive number, or · if it is unknown"
        onchange={minMax}
        onkeydown={onEnter}
      />
      <span>,</span>
      <input
        bind:this={hi}
        use:orig={c.hi}
        list="card-hi"
        class="num"
        data-part="hi"
        title="The maximum: any positive number, or * for no limit"
        onchange={minMax}
        onkeydown={onEnter}
      />
      <span>)</span>
      <datalist id="card-lo"><option value="0"></option><option value="1"></option><option value="2"></option><option value="3"></option><option value="·"></option></datalist>
      <datalist id="card-hi"><option value="1"></option><option value="2"></option><option value="3"></option><option value="4"></option><option value="*"></option></datalist>
    {:else}
      {#each c.choices as it}
        <button class="choice" class:on={c.current === it.value} aria-pressed={c.current === it.value} title={it.tip} data-value={it.value} onclick={() => set("card", it.value)}>{it.text}</button>
      {/each}
    {/if}
    <button class="choice none" title="No cardinality" onclick={() => set("card", "")}>none</button>
  </div>
  {#if c.note}<div class="note">{c.note}</div>{/if}
  {#if invalid}
    <div class="sentence bad">{invalid}</div>
  {:else if c.sentence}
    <div class="sentence">{c.sentence}</div>
  {/if}
  {#if c.deviation}
    <button class="deviate" title={c.deviation[1]} onclick={() => set("card.deviate", "")}>{c.deviation[0]}</button>
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-wrap: wrap;
  }
  button {
    font: inherit;
    color: var(--text);
  }
  .choice {
    min-width: 38px;
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 2px 6px;
  }
  .choice.on {
    background: var(--highlight);
    color: #fff;
    border-color: var(--highlight);
  }
  .none {
    margin-left: 4px;
  }
  .num {
    width: 46px;
    font: inherit;
    color: var(--text);
    background: var(--base);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 2px 4px;
  }
  .note,
  .sentence {
    margin-top: 4px;
  }
  .bad {
    color: #c00;
  }
  .deviate {
    display: block;
    background: none;
    border: 0;
    padding: 2px;
    margin-top: 2px;
    text-align: left;
    color: var(--link);
    cursor: pointer;
  }
</style>
