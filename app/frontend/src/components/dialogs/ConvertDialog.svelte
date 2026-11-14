<script lang="ts">
  // GSM -> ER: choices, assumptions and a live preview (after gsmconvert.py).  The GSM diagram
  // is only read; the result opens in a new tab and is saved only when saved there.
  import { untrack } from "svelte";
  import type { App } from "../../lib/app.svelte";
  import { Diagram } from "../../lib/core";
  import { fit, zoomOf, zoomTo, type View } from "../../lib/view";

  interface Choice {
    key: string;
    title: string;
    question: string;
    options: [string, string, string][];
    default: string;
    current: string;
    /** the rule in the rule books that describes it */
    rule: string;
  }
  interface Result {
    text: string;
    choices: Choice[];
    report: string[];
  }

  let { app, close }: { app: App; close: () => void } = $props();
  const app0 = untrack(() => app);
  const tab = app0.current;
  let names = $state(app0.settings["gsm2er/attr_names"] ?? "auto");
  let keys = $state((app0.settings["gsm2er/add_keys"] ?? "true") === "true");
  const picked = $state<Record<string, string>>({});
  let box: HTMLDivElement | undefined = $state();
  let pane: HTMLDivElement | undefined = $state();
  let size = $state({ w: 600, h: 500 });
  let view = $state<View | null>(null);
  $effect(() => box?.focus());

  const result: Result | { error: string } = $derived.by(() => {
    const settingNames = Object.keys(app.settings);
    try {
      return JSON.parse(
        tab.diagram.convert_to_er(
          names,
          keys,
          Object.keys(picked),
          Object.values(picked),
          settingNames,
          settingNames.map((n) => app.settings[n]),
          tab.path ? tab.path.split(/[\\/]/).pop() : undefined,
        ),
      ) as Result;
    } catch (e) {
      return { error: String(e) };
    }
  });

  // the preview: the converted diagram, drawn by the core as on the canvas
  const preview = $derived.by(() => {
    if ("error" in result) return null;
    const d = new Diagram(result.text, "er");
    d.set_language(app.language);
    const b = d.bounds();
    const svg = d.drawing(false, 2);
    d.free();
    return { svg: `<defs>${svg[0]}</defs>${svg[1]}`, bounds: b.length === 4 ? ([b[0], b[1], b[2], b[3]] as [number, number, number, number]) : null };
  });
  const shown = $derived(view ?? fit(preview?.bounds ?? null, size.w, size.h));

  $effect(() => {
    if (!pane) return;
    const ro = new ResizeObserver(() => {
      if (pane) size = { w: pane.clientWidth, h: pane.clientHeight };
    });
    ro.observe(pane);
    return () => ro.disconnect();
  });

  async function setNames(v: string) {
    names = v;
    await app.setSetting("gsm2er/attr_names", v);
  }

  async function setKeys(v: boolean) {
    keys = v;
    await app.setSetting("gsm2er/add_keys", v ? "true" : "false");
  }

  function open() {
    if ("error" in result) return;
    app.openConverted(result.text, tab);
    close();
  }

  function onKey(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    const r = pane!.getBoundingClientRect();
    const v = shown;
    view = zoomTo(v, zoomOf(v) * 1.15 ** (-e.deltaY / 100), e.clientX - r.left, e.clientY - r.top);
  }

  let drag: { x: number; y: number } | null = null;
  function down(e: PointerEvent) {
    drag = { x: e.clientX, y: e.clientY };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!drag) return;
    const v = shown;
    view = { ...v, tx: v.tx + e.clientX - drag.x, ty: v.ty + e.clientY - drag.y };
    drag = { x: e.clientX, y: e.clientY };
  }
</script>

<div class="backdrop" role="presentation">
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Convert GSM to ER Diagram" tabindex="-1" bind:this={box} onkeydown={onKey} data-testid="convert">
    <div class="title">Convert GSM to ER Diagram</div>
    <div class="main">
      <div class="left">
        <div class="form">
          <label for="cv-names">Attribute names</label>
          <select id="cv-names" value={names} onchange={(e) => setNames(e.currentTarget.value)} data-testid="convert-names">
            <option value="auto">Type name, or edge label for generic types</option>
            <option value="type">Type name (PName)</option>
            <option value="edge">Edge label (has name)</option>
          </select>
          <span>Keys</span>
          <label><input type="checkbox" checked={keys} onchange={(e) => setKeys(e.currentTarget.checked)} /> Add an ID key where no identifier is found</label>
        </div>
        <h4>Choices</h4>
        <div class="choices">
          {#if "error" in result}
            <p class="bad">{result.error}</p>
          {:else if !result.choices.length}
            <p>The conversion is unambiguous for this diagram.</p>
          {:else}
            {#each result.choices as c}
              <fieldset>
                <legend
                  >{c.title}{#if c.rule}
                    <button class="rule" title="Open rule {c.rule} in the rule books" onclick={() => app.openRule(c.rule)} data-rule={c.rule}>{c.rule}</button
                    >{/if}</legend
                >
                <p>{c.question}</p>
                {#each c.options as [value, title, text]}
                  <label class="option">
                    <input type="radio" name="cv-{c.key}" checked={value === c.current} onchange={() => (picked[c.key] = value)} />
                    {title}{value === c.default ? "  (default)" : ""}
                  </label>
                  <div class="text">{text}</div>
                {/each}
              </fieldset>
            {/each}
          {/if}
        </div>
        <h4>Assumptions and notes <span class="normal">— please check these in the ER diagram</span></h4>
        <ul class="report" data-testid="convert-report">
          {#each "error" in result ? [] : result.report.length ? result.report : ["(nothing to note)"] as line}<li>{line}</li>{/each}
        </ul>
      </div>
      <div
        class="preview"
        bind:this={pane}
        role="img"
        aria-label="Preview"
        title="Preview — wheel: zoom, drag: move, double-click: fit"
        onwheel={wheel}
        onpointerdown={down}
        onpointermove={move}
        onpointerup={() => (drag = null)}
        ondblclick={() => (view = null)}
      >
        {#if preview}
          <svg width={size.w} height={size.h}>
            <g transform="matrix({shown.k} 0 0 {-shown.k} {shown.tx} {shown.ty})">{@html preview.svg}</g>
          </svg>
        {/if}
      </div>
    </div>
    <p class="note">The GSM diagram is not changed. The ER diagram opens in a new tab and is saved only when you save it there.</p>
    <div class="buttons">
      <button onclick={open} disabled={"error" in result} data-testid="convert-ok">Open as New ER Diagram</button>
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
    display: flex;
    flex-direction: column;
    width: min(1200px, 96vw);
    height: min(780px, 94vh);
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
    gap: 8px;
    padding: 8px;
  }
  .left {
    width: 40%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .form {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 10px;
    align-items: center;
  }
  select {
    font: inherit;
    color: var(--text);
    background: var(--base);
    border: 1px solid var(--mid);
  }
  h4 {
    margin: 10px 0 4px;
    font-size: 13px;
  }
  .normal {
    font-weight: normal;
  }
  .choices {
    flex: 3;
    overflow: auto;
    border: 1px solid var(--mid);
    background: var(--base);
    padding: 6px;
  }
  fieldset {
    border: 1px solid var(--mid);
    margin: 0 0 8px;
  }
  .rule {
    font: inherit;
    font-family: monospace;
    color: var(--highlight);
    text-decoration: underline;
    background: none;
    border: 0;
    padding: 0 4px;
    cursor: pointer;
  }
  .option {
    display: block;
  }
  .text {
    margin: 0 0 4px 22px;
    font-size: 12px;
  }
  .report {
    flex: 2;
    overflow: auto;
    margin: 0;
    padding: 4px 4px 4px 20px;
    border: 1px solid var(--mid);
    background: var(--base);
  }
  .preview {
    flex: 1;
    min-width: 0;
    background: #fff;
    border: 1px solid var(--mid);
    overflow: hidden;
    cursor: grab;
  }
  .note {
    margin: 0 12px;
  }
  .bad {
    color: #c00;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 8px 12px;
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
