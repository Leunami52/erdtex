<script lang="ts">
  // The Relational Schema / SQL window (after sqlexport.py): decisions with the relations of
  // every option, data types, problems, and the output in the chosen format.  It doesn't block
  // the window and follows the active tab: any change, also on the canvas or by undo, shows.
  // Choices, data types and the rule set are written into the diagram (undoable).
  import { untrack } from "svelte";
  import type { App } from "../../lib/app.svelte";
  import * as be from "../../lib/backend";
  import { baseName, dirName } from "../../lib/backend";
  import { SEVERITY_COLORS } from "../../lib/panel";
  import { type_presets } from "../../lib/core";

  interface Option {
    title: string;
    description: string;
    reason: string;
    valid: boolean;
    result: string;
  }
  interface Decision {
    element: string;
    title: string;
    question: string;
    current: number;
    default: number;
    minor: boolean;
    options: Option[];
    /** the rule in the rule books that describes it */
    rule: string;
  }
  interface RmView {
    er: boolean;
    output: string;
    decisions: Decision[];
    types: { owner: string; name: string; type: string; guessed: boolean }[];
    problems: { severity: "error" | "warning" | "hint"; text: string; element: string | null }[];
    issues: number;
    summary: string;
    typesShown: boolean;
  }

  const FORMATS: [string, string][] = [
    ["text", "Relational schema (plain text)"],
    ["latex", "Relational schema (LaTeX)"],
    ["markdown", "Relational schema (Markdown)"],
    ["postgresql", "SQL — PostgreSQL"],
    ["oracle", "SQL — Oracle"],
    ["sqlite", "SQL — SQLite"],
  ];
  const EXT: Record<string, string> = { text: ".txt", latex: ".tex", markdown: ".md", postgresql: ".sql", oracle: ".sql", sqlite: ".sql" };
  const SUFFIX: Record<string, string> = { text: "_schema", latex: "_schema", markdown: "_schema", postgresql: "_postgresql", oracle: "_oracle", sqlite: "_sqlite" };

  let { app, close }: { app: App; close: () => void } = $props();
  const s = untrack(() => app.settings);
  const o = $state({
    format: s["rm/format"] ?? "text",
    rel: s["rm/rel_naming"] ?? "entities",
    fk: s["rm/fk_naming"] ?? "plain",
    marks: s["rm/marks"] ?? "underscore",
    types: (s["rm/text_types"] ?? "true") === "true",
    quote: s["rm/quote_all"] === "true",
    drop: s["rm/drop"] === "true",
  });
  let page = $state<"decisions" | "types" | "problems">("decisions");
  let showMinor = $state(false);
  let summary = $state("");
  const presets = type_presets();

  const v: RmView = $derived.by(() => {
    const t = app.current;
    void t.version;
    return JSON.parse(
      t.diagram.relational(o.format, o.rel, o.fk, o.marks, o.types, o.quote, o.drop, t.path ? baseName(t.path) : ""),
    ) as RmView;
  });
  const majors = $derived(v.decisions.filter((d) => !d.minor));
  const minors = $derived(v.decisions.filter((d) => d.minor));
  const minorOpen = $derived(showMinor || minors.some((d) => d.current !== d.default));
  $effect(() => {
    summary = v.summary;
  });

  async function option(key: keyof typeof o, value: string | boolean) {
    (o as Record<string, string | boolean>)[key] = value;
    const names: Record<string, string> = { format: "format", rel: "rel_naming", fk: "fk_naming", marks: "marks", types: "text_types", quote: "quote_all", drop: "drop" };
    await app.setSetting(`rm/${names[key]}`, typeof value === "boolean" ? (value ? "true" : "false") : value);
  }

  const choose = (d: Decision, i: number) => app.edit((x) => x.rm_choose(d.element, i, o.rel, o.fk));

  async function save() {
    const t = app.current;
    const stem = t.path ? baseName(t.path).replace(/\.[^.]*$/, "") : "diagram";
    const folder = app.outputDir(t, "rm/dirs", "rm/dir");
    const ext = EXT[o.format];
    const name = { ".txt": "Text", ".tex": "LaTeX", ".md": "Markdown", ".sql": "SQL" }[ext] ?? "Text";
    let path = await be.saveFileDialog("Save Relational Schema", `${folder ? folder + "/" : ""}${stem}${SUFFIX[o.format]}${ext}`, name, ext);
    if (!path) return;
    if (!be.hasSuffix(path)) path += ext;
    try {
      const abs = await be.writeTextFile(path, v.output);
      await app.rememberOutputDir(t, "rm/dirs", "rm/dir", dirName(abs));
      summary = `Saved ${baseName(abs)}`;
    } catch (e) {
      await app.alert(String(e));
    }
  }

  async function copy() {
    await be.copyText(v.output);
    app.message("Copied the relational schema", 3000);
  }

  // the window can be moved by its title
  let pos = $state({ x: 80, y: 60 });
  let grab: { x: number; y: number } | null = null;
  function down(e: PointerEvent) {
    if ((e.target as Element).closest("button")) return; // the close button keeps its click
    grab = { x: e.clientX - pos.x, y: e.clientY - pos.y };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (grab) pos = { x: Math.max(0, e.clientX - grab.x), y: Math.max(0, e.clientY - grab.y) };
  }
</script>

{#snippet decision(d: Decision)}
  <fieldset class="decision">
    <legend
      >{d.title}{#if d.rule}
        <button class="rule" title="Open rule {d.rule} in the rule books" onclick={() => app.openRule(d.rule)} data-rule={d.rule}>{d.rule}</button
        >{/if}</legend
    >
    <p>{d.question}</p>
    {#each d.options as op, i}
      <label class="option" class:invalid={!op.valid}>
        <input type="radio" name="rm-{d.element}" checked={i === d.current} disabled={!op.valid} data-option={i} onchange={() => choose(d, i)} />
        {op.title}
      </label>
      <div class="description" class:invalid={!op.valid}>
        {@html op.description}{#if op.reason}<br /><i>{op.reason}</i>{/if}
      </div>
      {#if op.valid}<pre class="result">{op.result}</pre>{/if}
    {/each}
  </fieldset>
{/snippet}

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="window"
  role="dialog"
  aria-label="Relational Schema / SQL"
  tabindex="-1"
  style:left="{pos.x}px"
  style:top="{pos.y}px"
  onkeydown={(e) => e.key === "Escape" && (e.stopPropagation(), close())}
  data-testid="relational"
>
  <div class="title" role="presentation" onpointerdown={down} onpointermove={move} onpointerup={() => (grab = null)}>
    <span>Relational Schema / SQL</span>
    <button class="x" onclick={close} aria-label="Close">×</button>
  </div>
  <div class="main">
    <div class="left">
      <div class="tabs" role="tablist">
        <button role="tab" aria-selected={page === "decisions"} class:on={page === "decisions"} onclick={() => (page = "decisions")}>Decisions ({majors.length})</button>
        <button role="tab" aria-selected={page === "types"} class:on={page === "types"} onclick={() => (page = "types")}>Data types</button>
        <button role="tab" aria-selected={page === "problems"} class:on={page === "problems"} onclick={() => (page = "problems")}>{v.issues ? `Problems (${v.issues})` : "Problems"}</button>
      </div>
      <div class="page" data-testid="rm-page">
        {#if page === "decisions"}
          {#if !v.decisions.length}
            <p>
              This diagram can be transformed without choices: the rules (ER → Relational in the rule books) determine every relation.<br /><br />Choices appear here for
              generalisations, 1:1 and c:c relationships, recursive relationships and some n-ary relationships.
            </p>
          {/if}
          {#each majors as d}{@render decision(d)}{/each}
          {#if minors.length}
            <button class="toggle" aria-pressed={minorOpen} data-testid="rm-minor" onclick={() => (showMinor = !minorOpen)}>
              Optional alternatives ({minors.length}): an own relation instead of a foreign key
            </button>
            {#if minorOpen}{#each minors as d}{@render decision(d)}{/each}{/if}
          {/if}
        {:else if page === "types"}
          <p>Types in <i>italics</i> are guessed from the name. They are stored in the diagram as <tt>name : type</tt> and printed in the diagram only with <tt>show types</tt>.</p>
          <table class="types">
            <thead><tr><th>Element</th><th>Attribute</th><th>Data type</th></tr></thead>
            <tbody>
              {#each v.types as r, k}
                <tr>
                  <td>{r.owner}</td>
                  <td>{r.name}</td>
                  <td>
                    <input
                      list="rm-types"
                      class:guessed={r.guessed}
                      value={r.type}
                      title={r.guessed ? "guessed from the name" : ""}
                      data-row={k}
                      onchange={(e) => app.edit((x) => x.rm_set_type(k, e.currentTarget.value))}
                    />
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
          <datalist id="rm-types">{#each presets as p}<option value={p}></option>{/each}</datalist>
          <div class="row">
            <button disabled={!v.types.some((r) => r.guessed)} title="Write all guessed types into the diagram" onclick={() => app.edit((x) => x.rm_store_guesses())}>Store guessed types</button>
            <span class="grow"></span>
            <label><input type="checkbox" checked={v.typesShown} onchange={(e) => { const on = e.currentTarget.checked; app.edit((x) => x.set_types_shown(on)); }} /> Show data types in the diagram</label>
          </div>
        {:else}
          <ul class="problems" data-testid="rm-problems">
            {#each v.problems as p}
              <li>
                <button ondblclick={() => p.element && app.edit((x) => x.select_ids([p.element!]))}>
                  <span style:color={SEVERITY_COLORS[p.severity]}>{p.severity === "error" ? "✖" : p.severity === "warning" ? "⚠" : "ℹ"}</span>
                  {p.text}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>
    <div class="right">
      <div class="form">
        <label for="rm-fmt">Format</label>
        <select id="rm-fmt" value={o.format} onchange={(e) => option("format", e.currentTarget.value)} data-testid="rm-format">
          {#each FORMATS as [k, t]}<option value={k}>{t}</option>{/each}
        </select>
        <label for="rm-rel2">Relationship relations</label>
        <select id="rm-rel2" value={o.rel} onchange={(e) => option("rel", e.currentTarget.value)}>
          <option value="entities">Entity_relationship_Entity (BandMember_plays_Instrument)</option>
          <option value="relationship">Relationship name (plays)</option>
        </select>
        <label for="rm-fk2">Foreign keys</label>
        <select id="rm-fk2" value={o.fk} onchange={(e) => option("fk", e.currentTarget.value)}>
          <option value="plain">Key name (deptNo); qualified only on clashes</option>
          <option value="relationship">Relationship + key name (employsDeptNo)</option>
        </select>
        {#if o.format === "text"}
          <span>Keys</span>
          <span class="row">
            <select value={o.marks} onchange={(e) => option("marks", e.currentTarget.value)}>
              <option value="underscore">_key_ &nbsp;/foreign/ &nbsp;_/both/_</option>
              <option value="tags">key (PK) &nbsp;foreign (FK)</option>
            </select>
            <label><input type="checkbox" checked={o.types} onchange={(e) => option("types", e.currentTarget.checked)} /> Data types</label>
          </span>
        {:else if o.format === "markdown"}
          <span>Keys</span>
          <span class="row">
            <span>**key** &nbsp;*foreign* &nbsp;***both***</span>
            <label><input type="checkbox" checked={o.types} onchange={(e) => option("types", e.currentTarget.checked)} /> Data types</label>
          </span>
        {:else if o.format !== "latex"}
          <span>SQL</span>
          <span class="row">
            <label title={'Otherwise only names that need it (reserved words such as "from", spaces) are quoted.\nQuoted names are case-sensitive.'}
              ><input type="checkbox" checked={o.quote} onchange={(e) => option("quote", e.currentTarget.checked)} /> Quote all names</label
            >
            <label><input type="checkbox" checked={o.drop} onchange={(e) => option("drop", e.currentTarget.checked)} data-testid="rm-drop" /> DROP TABLE statements first</label>
          </span>
        {/if}
      </div>
      {#if o.format === "markdown"}
        <p class="note" data-testid="rm-note">
          Markdown can't underline: primary keys are <b>bold</b> instead, foreign keys <i>italic</i>, columns that are both <b><i>bold italic</i></b>. The first lines of the output say so too.
        </p>
      {:else if o.format === "latex"}
        <p class="note" data-testid="rm-note">
          This needs ERDTeX's own LaTeX package <code>erdiagram</code> (<code>\usepackage{"{"}erdiagram{"}"}</code>); it isn't part of TeX Live or MiKTeX.
          <button class="link" onclick={() => void import("../../lib/latexPackages").then((m) => m.savePackages(app))} data-testid="rm-save-packages">Save LaTeX Packages…</button>
        </p>
      {/if}
      <textarea class="output" readonly value={v.output} spellcheck="false" data-testid="rm-output"></textarea>
      <div class="row">
        <span class="grow" data-testid="rm-summary">{summary}</span>
        <button onclick={copy}>Copy</button>
        <button onclick={save}>Save as…</button>
        <button onclick={close}>Close</button>
      </div>
    </div>
  </div>
</div>

<style>
  .window {
    position: fixed;
    z-index: 30;
    width: min(1250px, calc(100vw - 100px));
    height: min(800px, calc(100vh - 80px));
    display: flex;
    flex-direction: column;
    background: var(--window);
    color: var(--text);
    border: 1px solid var(--mid);
    border-radius: 6px;
    box-shadow: 0 8px 30px rgb(0 0 0 / 35%);
    resize: both;
    overflow: hidden;
    outline: none;
  }
  .note {
    margin: 0 0 6px;
    font-size: 0.92em;
    color: var(--text);
  }
  .note code {
    font-size: 0.95em;
  }
  .link {
    font: inherit;
    color: var(--link);
    text-decoration: underline;
    background: none !important;
    border: 0 !important;
    padding: 0 0 0 4px;
    cursor: pointer;
  }
  .title {
    display: flex;
    align-items: center;
    padding: 6px 10px;
    font-weight: 600;
    border-bottom: 1px solid var(--mid);
    cursor: move;
    user-select: none;
  }
  .title span {
    flex: 1;
  }
  .x {
    border: 0 !important;
    background: none !important;
    font-size: 18px;
    line-height: 1;
  }
  .main {
    display: flex;
    flex: 1;
    min-height: 0;
    gap: 8px;
    padding: 8px;
  }
  .left {
    flex: 5;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .right {
    flex: 6;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .tabs {
    display: flex;
    gap: 2px;
    border-bottom: 1px solid var(--mid);
  }
  .tabs button {
    border: 1px solid transparent !important;
    border-bottom: 0 !important;
    border-radius: 4px 4px 0 0 !important;
    background: transparent !important;
  }
  .tabs button.on {
    background: var(--base) !important;
    border-color: var(--mid) !important;
  }
  .page {
    flex: 1;
    overflow: auto;
    padding: 6px;
    background: var(--base);
    border: 1px solid var(--mid);
    border-top: 0;
  }
  .decision {
    border: 1px solid var(--mid);
    margin: 0 0 8px;
  }
  .rule {
    font: inherit;
    font-family: monospace;
    color: var(--highlight);
    text-decoration: underline;
    background: none !important;
    border: 0 !important;
    padding: 0 4px;
    cursor: pointer;
  }
  .option {
    display: block;
    margin-top: 4px;
  }
  .description {
    margin-left: 22px;
    font-size: 12px;
  }
  .invalid {
    color: var(--disabled);
  }
  .result {
    margin: 2px 0 6px 22px;
    padding: 3px 6px;
    border: 1px solid var(--mid);
    font: 12px monospace;
    white-space: pre-wrap;
    user-select: text;
  }
  .toggle {
    display: block;
    width: 100%;
    text-align: left;
    margin: 4px 0;
  }
  .types {
    width: 100%;
    border-collapse: collapse;
  }
  .types th {
    text-align: left;
    font-weight: normal;
    background: var(--button);
  }
  .types input {
    width: 100%;
  }
  .types input.guessed {
    font-style: italic;
  }
  .problems {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .problems button {
    width: 100%;
    text-align: left;
    border: 0 !important;
    background: none !important;
  }
  .form {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 4px 10px;
    align-items: center;
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .grow {
    flex: 1;
  }
  .output {
    flex: 1;
    resize: none;
    font: 12px monospace;
    white-space: pre;
    overflow: auto;
  }
  select,
  input:not([type="checkbox"]):not([type="radio"]),
  textarea {
    font: inherit;
    color: var(--text);
    background: var(--base);
    border: 1px solid var(--mid);
    border-radius: 3px;
    box-sizing: border-box;
  }
  textarea.output {
    font: 12px monospace;
  }
  button {
    font: inherit;
    color: var(--text);
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 2px 10px;
  }
  button:disabled {
    color: var(--disabled);
  }
</style>
