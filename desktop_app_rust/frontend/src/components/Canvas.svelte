<script lang="ts">
  // The canvas: the diagram as SVG from the core, with the grid (1 cm, as \grid), the axes
  // through the origin and ruler numbers along the top and left edge, as in the Python app.
  // Wheel: zoom around the cursor; Shift+wheel: scroll; middle button or Space+drag: pan.
  // The left button and the pointer go to the editor in the core, which draws the overlay
  // (selection, handles, the rubber band, the connection being drawn).
  import type { App, Tab } from "../lib/app.svelte";
  import type { MenuEntry } from "./ContextMenu.svelte";
  import { canvasColors } from "../lib/theme";
  import { CM, gridSteps, multiples, position, toDiagram, zoomOf } from "../lib/view";

  let { app, tab }: { app: App; tab: Tab } = $props();

  let box: HTMLDivElement;
  /** the problems of the element under the pointer */
  let tip = $state("");
  // with groups side by side there are several canvases: the current tab's takes the keyboard
  $effect(() => {
    if (tab.id === app.currentId) app.focusCanvas = () => box?.focus();
  });
  let content: SVGGElement;
  let over: SVGGElement;
  let editing = false;
  let cursor = $state("default");
  let w = $state(0);
  let h = $state(0);
  let panning = $state(false);
  let last = { x: 0, y: 0 };

  const colors = $derived(canvasColors(app.canvasDark));
  // the thinnest line is one device pixel (as in the Python app, and PDF viewers), so the
  // drawing follows the zoom and the screen's scale, not the panning
  const k = $derived(tab.view.k);
  let dpr = $state(window.devicePixelRatio || 1);
  $effect(() => {
    const m = window.matchMedia(`(resolution: ${dpr}dppx)`);
    const update = () => (dpr = window.devicePixelRatio || 1);
    m.addEventListener("change", update);
    return () => m.removeEventListener("change", update);
  });
  const drawing = $derived.by(() => {
    void tab.version;
    return tab.diagram.drawing(app.canvasDark, k * dpr);
  });

  $effect(() => {
    content.innerHTML = `<defs>${drawing[0]}</defs>${drawing[1]}`;
  });

  // the overlay follows the document, the selection, the drag and the zoom (handle sizes)
  $effect(() => {
    void tab.ui;
    tab.diagram.set_scale(1 / k);
    over.innerHTML = tab.diagram.overlay();
  });

  // the first view, once the size is known: the whole diagram, or the origin at 100 %
  $effect(() => {
    if (w > 0 && h > 0 && tab.pending) {
      tab.size = { w, h };
      if (tab.pending === "fit") tab.fit();
      else tab.origin();
      tab.pending = null;
    }
  });

  function resized(nw: number, nh: number) {
    if (nw === w && nh === h) return;
    // the middle stays where it is
    if (w > 0 && h > 0 && nw > 0 && nh > 0 && !tab.pending) {
      tab.view = { ...tab.view, tx: tab.view.tx + (nw - w) / 2, ty: tab.view.ty + (nh - h) / 2 };
    }
    w = nw;
    h = nh;
    tab.size = { w, h };
  }

  /** Measures the canvas again.  In a group, a canvas is mounted before dockview shows it (0 × 0), and
   * the ResizeObserver doesn't always report the size it gets then: dockview's layout and showing the
   * tab call this (Groups.svelte), as does the tab becoming the current one. */
  export function measure() {
    if (box?.isConnected) resized(box.clientWidth, box.clientHeight);
  }

  $effect(() => {
    const ro = new ResizeObserver(([entry]) => resized(Math.round(entry.contentRect.width), Math.round(entry.contentRect.height)));
    ro.observe(box);
    return () => ro.disconnect();
  });

  $effect(() => {
    if (tab.id === app.currentId) requestAnimationFrame(measure);
  });

  // grid lines one device pixel wide, on device pixels (as the Python app's cosmetic pens)
  const crisp = (v: number) => (Math.round(v * dpr) + 0.5) / dpr;
  // Panning has to be the compositor's work only: nothing is painted again while the view only
  // moves.  The grid is drawn once on a canvas a little larger than the view and only moved,
  // shifted by its own spacing (it repeats); the axes are thin lines that are moved; the
  // diagram and the overlay are on a layer that is moved; only the ruler numbers are drawn
  // again, on two thin strips.
  let gridCanvas: HTMLCanvasElement;
  let rulerTop: HTMLCanvasElement;
  let rulerLeft: HTMLCanvasElement;
  const snap = (v: number) => Math.round(v * dpr) / dpr;
  const steps = $derived(gridSteps(k));
  /** the grid repeats with its major step */
  const period = $derived(steps.step * k);

  function sized(c: HTMLCanvasElement, cw: number, ch: number): CanvasRenderingContext2D | null {
    const [pw, ph] = [Math.max(1, Math.round(cw * dpr)), Math.max(1, Math.round(ch * dpr))];
    if (c.width !== pw || c.height !== ph) {
      c.width = pw;
      c.height = ph;
    }
    const ctx = c.getContext("2d");
    ctx?.setTransform(dpr, 0, 0, dpr, 0, 0);
    return ctx;
  }

  // the grid's tile: drawn again only for another zoom, size, colours or grid setting
  $effect(() => {
    const c = gridCanvas;
    if (!c) return;
    const P = period;
    const [gw, gh] = [w + 2 * P, h + 2 * P];
    const ctx = sized(c, gw, gh);
    if (!ctx) return;
    ctx.fillStyle = colors.background;
    ctx.fillRect(0, 0, gw, gh);
    if (!app.showGrid) return;
    ctx.lineWidth = 1 / dpr;
    const lines = (color: string, every: number, skip: number) => {
      ctx.strokeStyle = color;
      ctx.beginPath();
      for (let j = 0; j * every <= gw + every; j++) {
        if (skip && j % skip === 0) continue;
        ctx.moveTo(crisp(j * every), 0);
        ctx.lineTo(crisp(j * every), gh);
      }
      for (let j = 0; j * every <= gh + every; j++) {
        if (skip && j % skip === 0) continue;
        ctx.moveTo(0, crisp(j * every));
        ctx.lineTo(gw, crisp(j * every));
      }
      ctx.stroke();
    };
    const minor = steps.minor * k;
    if (minor > 0) lines(colors.minor, minor, Math.round(P / minor));
    lines(colors.major, P, 0);
  });

  /** where the tile goes: its lines on the view's grid lines */
  const gridShift = $derived.by(() => {
    const P = period;
    const m = (v: number) => ((v % P) + P) % P;
    return { x: snap(m(tab.view.tx) - P), y: snap(m(tab.view.ty) - P) };
  });

  // the ruler numbers along the top and the left edge
  $effect(() => {
    const v = tab.view;
    const g = steps;
    const top = rulerTop && sized(rulerTop, w, 16);
    const left = rulerLeft && sized(rulerLeft, 40, h);
    if (!top || !left) return;
    top.clearRect(0, 0, w, 16);
    left.clearRect(0, 0, 40, h);
    if (!app.showGrid) return;
    const [x0, y1] = toDiagram(v, 0, 0);
    const [x1, y0] = toDiagram(v, w, h);
    for (const ctx of [top, left]) {
      ctx.fillStyle = colors.ruler;
      ctx.font = "10px sans-serif";
    }
    for (const x of multiples(x0, x1, g.label)) top.fillText(String(Math.round(x / CM) + 0), x * v.k + v.tx + 3, 12);
    for (const y of multiples(y0, y1, g.label)) left.fillText(String(Math.round(y / CM) + 0), 3, -y * v.k + v.ty - 3);
  });

  // the diagram's own layer: drawn for a zoom, then only moved while panning; its size is the
  // drawing's, with room for strokes around it
  const layer = $derived.by(() => {
    void tab.version;
    const b = tab.diagram.bounds();
    const pad = 40;
    if (b.length !== 4) return { x: 0, y: 0, w: 1, h: 1 };
    return { x: b[0] * k - pad, y: -b[3] * k - pad, w: (b[2] - b[0]) * k + 2 * pad, h: (b[3] - b[1]) * k + 2 * pad };
  });

  /** the overlay covers the view where it was when panning began, and moves with the layer */
  let overBox = $state({ x: 0, y: 0, w: 1, h: 1 });
  $effect(() => {
    const v = tab.view;
    if (!panning) overBox = { x: -v.tx - 2, y: -v.ty - 2, w: w + 4, h: h + 4 };
  });

  // pointer movements while panning, applied once per frame
  let panBy = { x: 0, y: 0 };
  let panFrame = 0;
  function pan(dx: number, dy: number) {
    panBy = { x: panBy.x + dx, y: panBy.y + dy };
    panFrame ||= requestAnimationFrame(() => {
      panFrame = 0;
      tab.view = { ...tab.view, tx: tab.view.tx + panBy.x, ty: tab.view.ty + panBy.y };
      panBy = { x: 0, y: 0 };
    });
  }

  function local(e: MouseEvent): [number, number] {
    const r = box.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    const unit = e.deltaMode === 1 ? 40 : e.deltaMode === 2 ? 400 : 1;
    if (e.shiftKey) {
      // scroll: the wheel moves the view up and down, a sideways wheel left and right
      tab.view = { ...tab.view, tx: tab.view.tx - e.deltaX * unit, ty: tab.view.ty - e.deltaY * unit };
      return;
    }
    if (e.deltaX) tab.view = { ...tab.view, tx: tab.view.tx - e.deltaX * unit };
    if (e.deltaY) {
      const [sx, sy] = local(e);
      tab.zoomAround(zoomOf(tab.view) * 1.15 ** ((-e.deltaY * unit) / 100), sx, sy);
    }
  }

  function at(e: MouseEvent): [number, number] {
    const [sx, sy] = local(e);
    return toDiagram(tab.view, sx, sy);
  }

  // Mouse events, not pointer events: WebKit loses the right button's pointerup when a
  // context menu opens under it, and then sends no pointerdown until the page is reloaded.
  // During a drag, the window gets the moves and the release (as pointer capture would).
  function onMouseDown(e: MouseEvent) {
    app.menu = null;
    if (e.button === 1 || (app.spaceDown && e.button === 0)) {
      e.preventDefault();
      panning = true;
      last = { x: e.clientX, y: e.clientY };
      follow();
      return;
    }
    if (e.button !== 0) return;
    e.preventDefault();
    box.focus();
    const [x, y] = at(e);
    if (e.detail === 2) {
      // the second press of a double click, as Qt reports it
      app.edit((d) => d.double_click(x, y));
      return;
    }
    editing = true;
    follow();
    app.edit((d) => d.press(x, y, e.shiftKey, e.ctrlKey || e.metaKey, e.altKey));
  }

  function follow() {
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  }

  function unfollow() {
    window.removeEventListener("mousemove", onMouseMove);
    window.removeEventListener("mouseup", onMouseUp);
  }

  function onMouseMove(e: MouseEvent) {
    const [sx, sy] = local(e);
    const inside = sx >= 0 && sy >= 0 && sx < w && sy < h;
    tab.pointer = inside ? [sx, sy] : null;
    const [x, y] = toDiagram(tab.view, sx, sy);
    if (inside && !panning) app.cursor = position(x, y);
    if (panning) {
      pan(e.clientX - last.x, e.clientY - last.y);
      last = { x: e.clientX, y: e.clientY };
      return;
    }
    app.edit((d) => d.move_to(x, y, e.shiftKey, e.ctrlKey || e.metaKey, e.altKey));
    if (!editing) {
      cursor = tab.diagram.cursor(x, y);
      tip = inside ? tab.diagram.tooltip(x, y) : "";
    }
  }

  function onMouseUp(e: MouseEvent) {
    if (panning && (e.button === 1 || e.button === 0)) {
      panning = false;
      unfollow();
      return;
    }
    if (editing && e.button === 0) {
      editing = false;
      unfollow();
      const [x, y] = at(e);
      app.edit((d) => d.release(x, y, e.shiftKey, e.ctrlKey || e.metaKey, e.altKey));
      cursor = tab.diagram.cursor(x, y);
    }
  }

  $effect(() => () => unfollow());

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    const [x, y] = at(e);
    const kind = app.edit((d) => d.context(x, y));
    const items: MenuEntry[] = [];
    if (kind === "none") {
      items.push({ label: "Paste", action: () => app.paste([x / CM, y / CM]) });
      items.push({ label: "Select all", action: () => app.selectAll() });
    } else {
      if (kind === "node") items.push({ label: "Add attribute list", action: () => app.edit((d) => d.add_attribute_list()) });
      if (kind === "edge") items.push({ label: "Straighten", action: () => app.edit((d) => d.straighten()) });
      if (kind === "node" || kind === "edge") items.push("-");
      items.push({ label: "Cut", action: () => app.cut() });
      items.push({ label: "Copy", action: () => app.copy() });
      items.push({ label: "Duplicate", action: () => app.duplicate() });
      items.push({ label: "Delete", action: () => app.deleteSelection() });
    }
    app.menu = { x: e.clientX, y: e.clientY, items };
  }
</script>

<div
  class="canvas"
  class:pan={app.spaceDown}
  class:panning
  style:cursor={app.spaceDown || panning ? null : cursor}
  title={tip || null}
  bind:this={box}
  tabindex="-1"
  oncontextmenu={onContextMenu}
  onmouseleave={() => !editing && !panning && (tab.pointer = null)}
  onwheel={onWheel}
  onmousedown={onMouseDown}
  onmousemove={(e) => !editing && !panning && onMouseMove(e)}
  role="presentation"
  data-testid="canvas"
  data-background={colors.background}
>
  <canvas
    class="grid moved"
    bind:this={gridCanvas}
    style:width="{w + 2 * period}px"
    style:height="{h + 2 * period}px"
    style:transform="translate3d({gridShift.x}px, {gridShift.y}px, 0)"
  ></canvas>
  {#if app.showGrid}
    <div class="axis h moved" style:height="{1 / dpr}px" style:background={colors.axes} style:transform="translate3d(0, {snap(tab.view.ty)}px, 0)"></div>
    <div class="axis v moved" style:width="{1 / dpr}px" style:background={colors.axes} style:transform="translate3d({snap(tab.view.tx)}px, 0, 0)"></div>
  {/if}
  <canvas class="ruler" bind:this={rulerTop} style:width="{w}px" style:height="16px"></canvas>
  <canvas class="ruler" bind:this={rulerLeft} style:width="40px" style:height="{h}px"></canvas>
  <div class="layer moved" style:transform="translate3d({snap(tab.view.tx)}px, {snap(tab.view.ty)}px, 0)">
    <svg
      class="drawing"
      data-testid="drawing"
      style:left="{layer.x}px"
      style:top="{layer.y}px"
      width={layer.w}
      height={layer.h}
      viewBox="{layer.x} {layer.y} {layer.w} {layer.h}"
      xmlns="http://www.w3.org/2000/svg"
    >
      <g bind:this={content} transform="matrix({k} 0 0 {-k} 0 0)"></g>
    </svg>
    <svg
      class="overlay"
      style:left="{overBox.x}px"
      style:top="{overBox.y}px"
      width={overBox.w}
      height={overBox.h}
      viewBox="{overBox.x} {overBox.y} {overBox.w} {overBox.h}"
      xmlns="http://www.w3.org/2000/svg"
    >
      <g bind:this={over} transform="matrix({k} 0 0 {-k} 0 0)"></g>
    </svg>
  </div>
</div>

<style>
  .canvas {
    position: absolute;
    inset: 0;
    overflow: hidden;
    touch-action: none;
    outline: none;
  }
  .canvas.pan {
    cursor: grab;
  }
  .canvas.panning {
    cursor: grabbing;
  }
  svg,
  canvas {
    display: block;
    user-select: none;
    position: absolute;
    left: 0;
    top: 0;
    pointer-events: none;
  }
  .layer {
    position: absolute;
    left: 0;
    top: 0;
    pointer-events: none;
  }
  /* moved by the compositor, never painted again for it */
  .moved {
    will-change: transform;
  }
  .axis {
    position: absolute;
    left: 0;
    top: 0;
    pointer-events: none;
  }
  .axis.h {
    width: 100%;
  }
  .axis.v {
    height: 100%;
  }
  .drawing {
    overflow: visible;
  }
</style>
