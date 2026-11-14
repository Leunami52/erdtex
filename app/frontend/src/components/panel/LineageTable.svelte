<script lang="ts">
  // Every attribute of an entity type (ER) or object type (GSM), its own and the inherited ones, marked
  // as in the drawing (key underlined, a weak entity's key dashed, {multi-valued}, /derived). Inherited
  // ones say where they come from; hovering shows every chain they are inherited along. A double click
  // (or Enter) opens the attribute where it is defined.
  import type { LineageRow } from "../../lib/panel";

  let { rows, open }: { rows: LineageRow[]; open: (target: string) => void } = $props();
  const types = $derived(rows.some((r) => r.type));
  const shown = (r: LineageRow) => (r.kind === "derived" ? `/${r.name}` : r.kind === "multi-valued" ? `{${r.name}}` : r.name);
</script>

{#if rows.length}
  <div class="scroll">
    <table title="Double-click an attribute to edit it where it is defined">
      <thead>
        <tr>
          <th>Attribute</th>
          {#if types}<th>Type</th>{/if}
          <th>From</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as r, k (k)}
          <tr
            class:inherited={!!r.from}
            title={r.trace}
            tabindex="0"
            data-lineage={k}
            ondblclick={() => open(r.target)}
            onkeydown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                open(r.target);
              }
            }}
          >
            <td style:padding-left="{4 + r.depth * 14}px"
              ><span class:key={r.key && !r.weakKey} class:weak={r.weakKey}>{shown(r)}</span>{#if r.unique}<span class="note"> (unique)</span>{/if}</td
            >
            {#if types}<td>{r.type}</td>{/if}
            <td class="from">{r.from ? `↑ ${r.from}` : "own"}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{:else}
  <div class="none">No attributes, own or inherited.</div>
{/if}

<style>
  .scroll {
    max-height: 300px;
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
    padding: 2px 4px;
    border-bottom: 1px solid var(--alternate);
    cursor: default;
  }
  tr:focus td,
  tr:hover td {
    background: color-mix(in srgb, var(--highlight) 18%, transparent);
  }
  tr:focus {
    outline: none;
  }
  tr.inherited td {
    font-style: italic;
    color: var(--disabled);
  }
  .key {
    text-decoration: underline;
  }
  .weak {
    text-decoration: underline dashed;
  }
  .from {
    white-space: nowrap;
  }
  .note,
  .none {
    color: var(--disabled);
  }
</style>
