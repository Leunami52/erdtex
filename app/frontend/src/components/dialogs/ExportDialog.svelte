<script lang="ts">
  // Export (after export.py): the LaTeX result (exactly as in the document) or the editor
  // rendering, as PNG, PDF or SVG; the choices are kept for next time (export/... settings,
  // also on Preferences' Output page).
  import { untrack } from "svelte";
  import type { App } from "../../lib/app.svelte";
  import * as be from "../../lib/backend";
  import { baseName, dirName } from "../../lib/backend";
  import { pngBytes, renderPage } from "../../lib/pdf";

  let { app, close }: { app: App; close: () => void } = $props();
  const s = untrack(() => app.settings);
  let missing = $state("");
  let source = $state(s["export/source"] ?? "latex");
  let format = $state(s["export/format"] ?? "pdf");
  let grid = $state(s["export/grid"] === "true");
  let dpi = $state(Number(s["export/dpi"] ?? 300) || 300);
  let transparent = $state(s["export/transparent"] === "true");
  let working = $state(false);
  let box: HTMLDivElement | undefined = $state();
  $effect(() => box?.focus());

  void be.latexMissing().then((m) => {
    missing = m;
    if (m) source = "editor";
  });

  async function exportNow() {
    const b = (v: boolean) => (v ? "true" : "false");
    dpi = Math.max(36, Math.min(2400, Math.round(dpi) || 300));
    for (const [k, v] of [["source", source], ["format", format], ["grid", b(grid)], ["dpi", String(dpi)], ["transparent", b(transparent)]]) {
      await app.setSetting(`export/${k}`, v);
    }
    const tab = app.current;
    const stem = tab.path ? baseName(tab.path).replace(/\.[^.]*$/, "") : "diagram";
    const folder = app.outputDir(tab, "export dirs", "export dir");
    const names: Record<string, string> = { png: "PNG image", pdf: "PDF document", svg: "SVG image" };
    const answer = app.nextSavePath;
    app.nextSavePath = null;
    let path = await be.saveFileDialog("Export Diagram", `${folder ? folder + "/" : ""}${stem}.${format}`, names[format], `.${format}`, answer);
    if (!path) return;
    if (!path.toLowerCase().endsWith(`.${format}`)) path = path.replace(/\.[^./\\]*$/, "") + `.${format}`;
    await app.rememberOutputDir(tab, "export dirs", "export dir", dirName(path));
    working = true;
    try {
      const job = { text: tab.diagram.text(), kind: tab.kind, language: app.language, preamble: app.preamble, format, grid, dpi, transparent, path };
      if (source === "latex") await exportLatex(job);
      else await be.exportEditor(job);
    } catch (e) {
      working = false;
      return fail(e);
    }
    working = false;
    close();
    app.message(`Exported ${path}`, 5000);
  }

  /** The LaTeX result: by pdftocairo, or else drawn here with pdf.js (PNG). */
  async function exportLatex(job: be.ExportJob) {
    try {
      await be.exportLatex(job);
    } catch (e) {
      if (String(e) !== be.NEEDS_PAGE) throw e;
      const pdf = await be.latexCompile(job.text, job.kind, job.preamble, job.language, job.grid);
      await be.writeBytes(job.path, await pngBytes(await renderPage(pdf, job.dpi / 72, job.transparent, 30000)));
    }
  }

  async function fail(e: unknown) {
    close();
    await app.alert(`Export failed:\n${e}`);
  }

  function onKey(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Escape" && !app.question) {
      e.preventDefault();
      close();
    }
  }
</script>

<div class="backdrop" role="presentation">
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Export Diagram" tabindex="-1" bind:this={box} onkeydown={onKey} data-testid="export">
    <div class="title">Export Diagram</div>
    <div class="form">
      <span>Source</span>
      <div>
        <label class:off={!!missing} title={missing || undefined}
          ><input type="radio" name="ex-source" value="latex" bind:group={source} disabled={!!missing} data-testid="export-latex" /> LaTeX result (exactly as in the document){missing
            ? be.IS_WEB
              ? " — needs pdflatex: in the desktop version"
              : " — pdflatex not found"
            : ""}</label
        >
        <label><input type="radio" name="ex-source" value="editor" bind:group={source} data-testid="export-editor" /> Editor rendering (as on the canvas)</label>
      </div>
      <label for="ex-fmt">Format</label>
      <select id="ex-fmt" bind:value={format} data-testid="export-format">
        <option value="png">PNG image</option>
        <option value="pdf">PDF (vector)</option>
        <option value="svg">SVG (vector)</option>
      </select>
      <span></span>
      <label><input type="checkbox" bind:checked={grid} /> Include the coordinate grid</label>
      {#if format === "png"}
        <span></span>
        <span class="row">
          Resolution <input type="number" min="36" max="2400" bind:value={dpi} data-testid="export-dpi" /> dpi
          <label><input type="checkbox" bind:checked={transparent} /> Transparent background</label>
        </span>
      {/if}
    </div>
    <div class="buttons">
      {#if working}<span class="grow">Exporting…</span>{/if}
      <button onclick={exportNow} disabled={working} data-testid="export-ok">Export…</button>
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
    min-width: 460px;
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
  .row label {
    margin-left: 10px;
  }
  .off {
    color: var(--disabled);
  }
  input[type="number"] {
    width: 70px;
  }
  select,
  input[type="number"] {
    font: inherit;
    color: var(--text);
    background: var(--base);
    border: 1px solid var(--mid);
    border-radius: 3px;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-top: 1px solid var(--mid);
  }
  .grow {
    flex: 1;
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
