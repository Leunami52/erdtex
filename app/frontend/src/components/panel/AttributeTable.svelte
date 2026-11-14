<script lang="ts">
  // The attributes of a list, top to bottom: ticks for key and unique, the name (indented for
  // parts of a composite attribute), the data type; buttons below; the kind and formula of the
  // current row.  The "next attribute" keys (Shift+Enter) commit the name and start the next.
  import { tick } from "svelte";
  import type { Outcome } from "../../lib/app.svelte";
  import { type_presets } from "../../lib/core";
  import { changed, orig, selectOnFocus, type AttrRow } from "../../lib/panel";

  let {
    rows,
    set,
    isNext,
    nextKeys,
    want,
  }: {
    rows: AttrRow[];
    set: (id: string, value: string) => Promise<Outcome>;
    isNext: (e: KeyboardEvent) => boolean;
    nextKeys: string;
    /** a row to show (and whether to start typing its name), from outside */
    want: { row: number; edit: boolean; n: number } | null;
  } = $props();

  let current = $state(-1);
  let table: HTMLTableElement;
  const presets = type_presets();
  const row = $derived(current >= 0 && current < rows.length ? rows[current] : null);

  async function show(k: number, edit: boolean) {
    current = Math.min(k, rows.length - 1);
    await tick();
    const name = table?.querySelector<HTMLInputElement>(`input[data-name="${current}"]`);
    if (name && edit) {
      name.focus();
      name.select();
    }
  }

  $effect(() => {
    if (want) void show(want.row, want.edit);
  });

  async function apply(id: string, value: string) {
    const out = await set(id, value);
    if (out.kind === "row") await show(out.row, out.edit);
  }

  function name(k: number, el: HTMLInputElement) {
    if (changed(el)) void apply(`attr.name.${k}`, el.value);
  }

  async function onNameKey(e: KeyboardEvent, k: number) {
    const el = e.target as HTMLInputElement;
    if (isNext(e)) {
      e.preventDefault();
      if (changed(el)) await set(`attr.name.${k}`, el.value);
      await apply(`attr.next.${k}`, "");
    } else if (e.key === "Enter") {
      e.preventDefault();
      name(k, el);
    }
  }

  const button = (what: string) => () => apply(`attr.${what}.${current}`, "");
</script>

<div class="attributes" data-field="attributes">
  <div class="scroll">
    <table bind:this={table} title={`Type a name to change it; ${nextKeys || "see Preferences"} adds the next attribute. Tick = key attribute (underlined).`}>
      <thead>
        <tr>
          <th>Key</th>
          <th title="Unique: a further candidate key. It isn't drawn, and becomes UNIQUE in SQL.">Uniq.</th>
          <th class="wide">Attribute</th>
          <th>Data type</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as r, k (k)}
          <tr class:current={k === current} onfocusin={() => (current = k)} onclick={() => (current = k)}>
            <td><input type="checkbox" checked={r.key} data-key={k} onchange={(e) => apply(`attr.key.${k}`, String(e.currentTarget.checked))} /></td>
            <td><input type="checkbox" checked={r.unique} data-unique={k} onchange={(e) => apply(`attr.unique.${k}`, String(e.currentTarget.checked))} /></td>
            <td class="wide">
              <input
                class="name"
                style:padding-left="{4 + 12 * r.depth}px"
                use:orig={r.name}
                data-name={k}
                spellcheck="false"
                use:selectOnFocus
                onkeydown={(e) => onNameKey(e, k)}
                onblur={(e) => name(k, e.currentTarget)}
              />
            </td>
            <td>
              <input
                class="type"
                list="attr-types"
                use:orig={r.type}
                data-type={k}
                disabled={r.composite}
                placeholder={r.guess ? `${r.guess} (guessed)` : ""}
                title="Data type for the relational schema / SQL export, e.g. integer, string, varchar(40), decimal(8,2), date"
                onchange={(e) => changed(e.currentTarget) && apply(`attr.type.${k}`, e.currentTarget.value)}
              />
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <datalist id="attr-types">
    {#each presets as p}<option value={p}></option>{/each}
  </datalist>
  <div class="buttons">
    <button onclick={button("add")}>Add</button>
    <button onclick={button("remove")}>Remove</button>
    <button onclick={button("up")}>Up</button>
    <button onclick={button("down")}>Down</button>
  </div>
  <div class="buttons">
    <button onclick={button("indent")}>Part of the above</button>
    <button onclick={button("outdent")}>Not a part</button>
  </div>
  <div class="kind">
    <label for="attr-kind">Kind</label>
    <select
      id="attr-kind"
      disabled={!row}
      value={row?.kind ?? "normal"}
      title={"Multi-valued: {email}, stored in a relation of its own.\nDerived: /age, not stored."}
      onchange={(e) => apply(`attr.kind.${current}`, e.currentTarget.value)}
    >
      <option value="normal">normal</option>
      <option value="multi-valued">multi-valued</option>
      <option value="derived">derived</option>
    </select>
    {#if row?.kind === "derived"}
      <label for="attr-formula">Formula</label>
      <input
        id="attr-formula"
        use:orig={row.formula}
        placeholder="SQL expression (optional)"
        title="Computes the attribute in a view of the relational schema"
        onchange={(e) => changed(e.currentTarget) && apply(`attr.formula.${current}`, e.currentTarget.value)}
      />
    {/if}
  </div>
</div>

<style>
  .scroll {
    max-height: 340px;
    min-height: 120px;
    overflow: auto;
    border: 1px solid var(--mid);
    background: var(--base);
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th {
    position: sticky;
    top: 0;
    background: var(--button);
    font-weight: normal;
    text-align: left;
    padding: 2px 4px;
    border-bottom: 1px solid var(--mid);
  }
  td {
    padding: 0 2px;
    border-bottom: 1px solid var(--alternate);
  }
  .wide {
    width: 60%;
  }
  tr.current td {
    background: color-mix(in srgb, var(--highlight) 18%, transparent);
  }
  input.name,
  input.type {
    width: 100%;
    box-sizing: border-box;
    font: inherit;
    color: var(--text);
    background: transparent;
    border: 0;
    padding: 2px 4px;
  }
  input.type {
    min-width: 90px;
  }
  input.name:focus,
  input.type:focus {
    background: var(--base);
    outline: 1px solid var(--highlight);
  }
  .buttons {
    display: flex;
    gap: 4px;
    margin-top: 4px;
  }
  .buttons button {
    flex: 1;
    min-width: 30px;
    font: inherit;
    color: var(--text);
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 2px 4px;
  }
  .kind {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 8px;
    align-items: center;
    margin-top: 6px;
  }
  .kind select,
  .kind input {
    font: inherit;
    color: var(--text);
    background: var(--base);
    border: 1px solid var(--mid);
    border-radius: 3px;
  }
</style>
