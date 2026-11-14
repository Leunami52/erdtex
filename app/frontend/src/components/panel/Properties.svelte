<script lang="ts">
  // The properties panel: draws the form the core describes for the selection (panel.rs) and
  // reports what its fields change.  It is built again when the document or the selection
  // changes; fields being typed in keep their text.
  import { tick } from "svelte";
  import type { App, Tab } from "../../lib/app.svelte";
  import { display } from "../../lib/keys";
  import { changed, orig, SEVERITY_COLORS, type Form, type Row } from "../../lib/panel";
  import AttributeTable from "./AttributeTable.svelte";
  import CardField from "./CardField.svelte";
  import ColorField from "./ColorField.svelte";
  import LabelField from "./LabelField.svelte";
  import LineageTable from "./LineageTable.svelte";

  let { app, tab }: { app: App; tab: Tab } = $props();
  let box: HTMLDivElement;
  let invalid = $state<Record<string, string>>({});
  let want = $state<{ row: number; edit: boolean; n: number } | null>(null);
  let wants = 0;

  const form: Form = $derived.by(() => {
    void tab.version;
    void tab.sel;
    return JSON.parse(tab.diagram.panel()) as Form;
  });

  $effect(() => {
    void form;
    invalid = {};
  });

  async function set(id: string, value: string) {
    const out = await app.panelSet(id, value);
    if (out.kind === "invalid") invalid = { ...invalid, [id]: out.text };
    if (out.kind === "row") want = { row: out.row, edit: out.edit, n: ++wants };
    return out;
  }

  // an attribute opened from an entity's list of all attributes: its row in the attribute list
  $effect(() => {
    const want0 = app.attributeRow;
    if (!want0) return;
    const rows = form.rows.find((r) => r.type === "attributes");
    if (rows) want = { row: want0, edit: true, n: ++wants };
    app.attributeRow = null;
  });

  // the cursor into the label field: after placing an element, a double click on it
  $effect(() => {
    if (tab.labelRequest === 0) return;
    void tick().then(focusLabel);
  });

  function focusLabel() {
    const which = form.focus;
    if (!which || !box) return;
    if (which === "attributes") {
      const rows = form.rows.find((r) => r.type === "attributes");
      const n = rows && rows.type === "attributes" ? rows.rows.length : 0;
      if (n) want = { row: n - 1, edit: true, n: ++wants };
      return;
    }
    const el = box.querySelector<HTMLElement>(
      which === "card" ? '[data-field="card"] button, [data-field="card"] input' : `[data-row="${which}"] textarea, [data-row="${which}"] input`,
    );
    el?.focus();
    if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) el.select();
  }

  function commitLine(r: Row, el: HTMLInputElement) {
    if (changed(el)) void set(r.id, el.value);
  }

  const onEnter = (r: Row) => (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      commitLine(r, e.currentTarget as HTMLInputElement);
    }
  };

  /** Rows without a label: ticks sit in the field column, the rest spans the panel. */
  const spans = (r: Row) => (!r.label && r.type !== "check") || r.type === "lineage";
  const nextKeys = $derived(display(app.bindings?.keys.get("next attribute") ?? ""));
  /** buttons that are commands too: their shortcut is shown on them */
  const BUTTON_COMMANDS: Record<string, string> = { "add-tree": "a_add_attributes" };
  const buttonKeys = (id: string) => {
    const keys = BUTTON_COMMANDS[id] ? display(app.bindings?.keys.get(BUTTON_COMMANDS[id]) ?? "") : "";
    return keys ? ` (${keys})` : "";
  };
  $effect(() => {
    app.panelButton = (id: string) => set(id, "");
    return () => (app.panelButton = null);
  });
  const isNext = (e: KeyboardEvent) => !!app.bindings?.matches("next attribute", e);
  const num = (v: number, decimals: number) => (Number.isFinite(v) ? v.toFixed(decimals) : "");
</script>

<div class="properties" bind:this={box} data-testid="properties">
  <h3>{form.title}</h3>
  <div class="form">
    {#each form.rows as r, i (r.id ? r.id : `#${i}`)}
      {#if !spans(r)}
        <div class="label">{r.label}</div>
      {/if}
      <div class="field" class:span={spans(r)} data-row={r.id} data-type={r.type}>
        {#if r.type === "text"}
          <div class="text" title={r.tip}>
            <span>{r.text}</span>
            {#if r.button}<button onclick={() => set(r.id, "")}>{r.button}</button>{/if}
          </div>
        {:else if r.type === "problem"}
          <div class="problem">
            <span><b style:color={SEVERITY_COLORS[r.severity]}>{r.severity[0].toUpperCase() + r.severity.slice(1)}:</b> {r.message}</span>
            {#if r.fix}<button onclick={() => set(r.id, "")}>{r.fix}</button>{/if}
          </div>
        {:else if r.type === "check"}
          <label class="check" title={r.tip}>
            <input type="checkbox" checked={r.value} disabled={!r.enabled} onchange={(e) => set(r.id, String(e.currentTarget.checked))} />
            {r.text}
          </label>
        {:else if r.type === "choice"}
          {#if r.editable}
            <input
              list="choices-{r.id}"
              use:orig={r.current}
              title={r.tip}
              onchange={(e) => commitLine(r, e.currentTarget)}
              onkeydown={onEnter(r)}
            />
            <datalist id="choices-{r.id}">
              {#each r.items as it}<option value={it.value}>{it.text}</option>{/each}
            </datalist>
          {:else}
            <select title={r.tip} value={r.current} onchange={(e) => set(r.id, e.currentTarget.value)}>
              {#each r.items as it}
                <option value={it.value} title={it.tip}>{it.text}</option>
              {/each}
            </select>
          {/if}
        {:else if r.type === "line"}
          <input
            use:orig={r.value}
            placeholder={r.placeholder}
            title={r.tip}
            spellcheck="false"
            onblur={(e) => commitLine(r, e.currentTarget)}
            onkeydown={onEnter(r)}
          />
        {:else if r.type === "label"}
          <LabelField value={r.value} tip={r.tip} commit={(v) => set(r.id, v)} />
        {:else if r.type === "number"}
          <span class="number">
            <input
              type="number"
              min={r.min}
              max={r.max}
              step={r.step}
              use:orig={num(r.value, r.decimals)}
              onchange={(e) => commitLine(r, e.currentTarget)}
              onkeydown={onEnter(r)}
            />{r.suffix}
          </span>
        {:else if r.type === "position"}
          <span class="position">
            x <input type="number" step={r.step} min="-500" max="500" data-axis="x" use:orig={num(r.x, 2)} onchange={(e) => changed(e.currentTarget) && set(`${r.id}.x`, e.currentTarget.value)} />
            y <input type="number" step={r.step} min="-500" max="500" data-axis="y" use:orig={num(r.y, 2)} onchange={(e) => changed(e.currentTarget) && set(`${r.id}.y`, e.currentTarget.value)} />
          </span>
        {:else if r.type === "button"}
          <button title={r.tip} onclick={() => set(r.id, "")}>{r.text}{buttonKeys(r.id)}</button>
        {:else if r.type === "color"}
          <ColorField id={r.id} swatches={r.swatches} current={r.current} tinted={r.tinted} commit={(v) => set(r.id, v)} />
        {:else if r.type === "datatype"}
          <input
            list="types-{r.id}"
            use:orig={r.value}
            placeholder={r.guess ? `${r.guess} (guessed)` : ""}
            title="Data type for the relational schema / SQL export, e.g. integer, string, varchar(40), decimal(8,2), date"
            onchange={(e) => commitLine(r, e.currentTarget)}
            onkeydown={onEnter(r)}
          />
        {:else if r.type === "card"}
          <CardField c={r} {set} />
        {:else if r.type === "attributes"}
          <AttributeTable rows={r.rows} {set} {isNext} {nextKeys} {want} />
        {:else if r.type === "lineage"}
          <div class="label lineage">{r.label.replace("\n", " ")}</div>
          <LineageTable rows={r.rows} open={(t) => app.gotoAttribute(t)} />
        {:else if r.type === "code"}
          <textarea class="code" readonly value={r.text}></textarea>
        {/if}
        {#if invalid[r.id]}<div class="bad">{invalid[r.id]}</div>{/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .properties {
    padding: 6px 8px 12px;
  }
  h3 {
    font-size: 13px;
    margin: 2px 0 8px;
  }
  .form {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 5px 8px;
    align-items: center;
  }
  .label {
    color: var(--text);
    white-space: pre-line; /* a label may have a line break of its own, and no others */
  }
  .field {
    min-width: 0;
  }
  .span {
    grid-column: 1 / -1;
  }
  .lineage {
    margin: 6px 0 3px;
  }
  .text,
  .problem {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .text span,
  .problem span {
    flex: 1;
    overflow-wrap: anywhere;
  }
  input:not([type="checkbox"]),
  select,
  textarea.code {
    font: inherit;
    width: 100%;
    box-sizing: border-box;
    color: var(--text);
    background: var(--base);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 2px 4px;
  }
  input::placeholder {
    color: var(--placeholder);
  }
  .number input {
    width: 80px;
    margin-right: 4px;
  }
  .position {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .position input {
    width: 0;
    flex: 1;
  }
  button {
    font: inherit;
    color: var(--text);
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 2px 8px;
  }
  .check {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  textarea.code {
    min-height: 120px;
    font-family: monospace;
  }
  .bad {
    color: #c00;
  }
</style>
