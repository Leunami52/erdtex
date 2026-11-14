<script lang="ts">
  // The exact preview (after preview.py): the diagram compiled by pdflatex a moment after each
  // change, with the packages bundled with the app.  One run at a time; a change during a run
  // waits for it, and only the newest one counts.  Errors are listed with their line, and the
  // last good result stays.  Wheel: zoom; drag: move; double click: fit.
  import { onDestroy } from "svelte";
  import type { App, Tab } from "../../lib/app.svelte";
  import * as be from "../../lib/backend";
  import { renderPage } from "../../lib/pdf";

  let { app }: { app: App } = $props();
  let status = $state("");
  let errors = $state<string[]>([]);
  let auto = $state(true);
  let grid = $state(false);
  let missing = $state("");
  let view: HTMLDivElement | undefined = $state();
  let image = $state<HTMLCanvasElement | null>(null);
  let shownTab: Tab | null = null;
  let zoom = $state(1);
  let offset = $state({ x: 0, y: 0 });
  let size = $state({ w: 300, h: 200 });
  let busy = false;
  let pending = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  /** how many runs finished (for tests) */
  let runs = $state(0);

  void be.latexMissing().then((m) => {
    missing = m;
    if (m) status = m;
  });

  function schedule(delay = 700) {
    clearTimeout(timer);
    timer = setTimeout(compile, delay);
  }

  // a change of the document: compile again in a moment (if wanted); another tab: at once
  $effect(() => {
    const t = app.current;
    void t.version;
    void app.preamble;
    void app.language;
    if (t !== shownTab) {
      shownTab = t;
      image = null;
      errors = [];
      status = missing;
      schedule(0);
    } else if (auto) {
      schedule();
    }
  });

  async function compile() {
    if (missing) return;
    if (busy) {
      pending = true;
      return;
    }
    busy = true;
    const tab = app.current;
    status = "Compiling…";
    try {
      const pdf = await be.latexCompile(tab.diagram.text(), tab.kind, app.preamble, app.language, grid);
      if (tab !== app.current) return void schedule(0); // from a tab no longer shown
      image = await renderPage(pdf, 300 / 72, false, 5000);
      errors = [];
      status = "Up to date.";
    } catch (e) {
      if (tab !== app.current) return void schedule(0);
      errors = String(e).split("\n").filter((l) => l);
      status = image ? "LaTeX reported errors (showing the last good result):" : "LaTeX reported errors:";
    } finally {
      busy = false;
      runs++;
      if (pending) {
        pending = false;
        void compile();
      }
    }
  }

  onDestroy(() => clearTimeout(timer));

  $effect(() => {
    if (!view) return;
    const ro = new ResizeObserver(() => view && (size = { w: view.clientWidth, h: view.clientHeight }));
    ro.observe(view);
    return () => ro.disconnect();
  });

  // the page fitted into the view, times the zoom, moved by the offset
  const rect = $derived.by(() => {
    if (!image) return null;
    const fit = Math.max(0.01, Math.min((size.w - 12) / image.width, (size.h - 12) / image.height));
    const s = fit * zoom;
    const [w, h] = [image.width * s, image.height * s];
    return { x: size.w / 2 + offset.x - w / 2, y: size.h / 2 + offset.y - h / 2, w, h };
  });

  let holder: HTMLDivElement | undefined = $state();
  $effect(() => {
    if (!holder) return;
    holder.replaceChildren(...(image ? [image] : []));
  });

  function zoomBy(factor: number, at?: { x: number; y: number }) {
    const old = zoom;
    zoom = Math.max(0.2, Math.min(20, zoom * factor));
    if (at) {
      // the point under the pointer stays where it is
      const c = { x: size.w / 2 + offset.x, y: size.h / 2 + offset.y };
      offset = { x: offset.x + (at.x - c.x) * (1 - zoom / old), y: offset.y + (at.y - c.y) * (1 - zoom / old) };
    }
  }

  function fit() {
    zoom = 1;
    offset = { x: 0, y: 0 };
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    const r = view!.getBoundingClientRect();
    zoomBy(1.2 ** (-e.deltaY / 120), { x: e.clientX - r.left, y: e.clientY - r.top });
  }

  let drag: { x: number; y: number } | null = null;
  function down(e: PointerEvent) {
    drag = { x: e.clientX, y: e.clientY };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!drag) return;
    offset = { x: offset.x + e.clientX - drag.x, y: offset.y + e.clientY - drag.y };
    drag = { x: e.clientX, y: e.clientY };
  }

  function snapshot() {
    return { status, errors, runs, image: image ? [image.width, image.height] : null, auto, grid };
  }
  $effect(() => {
    app.previewSnapshot = snapshot;
    return () => (app.previewSnapshot = null);
  });
</script>

<section class="preview" data-testid="preview">
  <div class="bar">
    <label title="Recompile automatically after every change"><input type="checkbox" bind:checked={auto} /> Auto-update</label>
    <label title="Compile with the package's grid option"><input type="checkbox" bind:checked={grid} onchange={() => schedule(0)} data-testid="preview-grid" /> Grid</label>
    <button onclick={() => schedule(0)} data-testid="preview-update">Update</button>
    <button class="small" onclick={() => zoomBy(0.8)} aria-label="Zoom out">−</button>
    <button onclick={fit}>Fit</button>
    <button class="small" onclick={() => zoomBy(1.25)} aria-label="Zoom in">+</button>
  </div>
  <div class="status" data-testid="preview-status">{status}</div>
  <div
    class="view"
    bind:this={view}
    role="img"
    aria-label="The diagram as LaTeX sets it"
    onwheel={wheel}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={() => (drag = null)}
    ondblclick={fit}
  >
    {#if rect}
      <div class="page" bind:this={holder} style:left="{rect.x}px" style:top="{rect.y}px" style:width="{rect.w}px" style:height="{rect.h}px"></div>
    {/if}
  </div>
  {#if errors.length}
    <textarea class="errors" readonly value={errors.join("\n")} data-testid="preview-errors"></textarea>
  {/if}
</section>

<style>
  .preview {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    color: var(--text);
  }
  .bar {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
    padding: 3px 6px;
  }
  button {
    font: inherit;
    color: var(--text);
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 1px 8px;
  }
  .small {
    width: 28px;
    padding: 1px 0;
  }
  .status {
    padding: 0 8px 3px;
    min-height: 1.2em;
  }
  .view {
    position: relative;
    flex: 1;
    min-height: 60px;
    overflow: hidden;
    background: var(--mid);
    cursor: grab;
  }
  .page {
    position: absolute;
    box-shadow: 2px 2px 0 rgb(0 0 0 / 24%);
  }
  .page :global(canvas) {
    width: 100%;
    height: 100%;
    display: block;
  }
  .errors {
    height: 90px;
    resize: none;
    font: 12px monospace;
    color: var(--text);
    background: var(--base);
    border: 0;
    border-top: 1px solid var(--mid);
  }
</style>
