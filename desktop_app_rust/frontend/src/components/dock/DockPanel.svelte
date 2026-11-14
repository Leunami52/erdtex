<script lang="ts">
  // A pane with its title bar, as the Python app's dock widgets: fold it (docked), float or dock
  // it, close it (View brings it back).  Dragging the title detaches the pane; dropped near the
  // left, right or bottom edge of the window it docks there.  A double click on the title docks a
  // floating pane again, and floats a docked one.
  import type { Snippet } from "svelte";
  import type { Area, Docks, PaneId } from "../../lib/docks.svelte";

  let { docks, id, title, children }: { docks: Docks; id: PaneId; title: string; children: Snippet } = $props();
  const p = $derived(docks.pane(id));
  let el: HTMLElement | undefined = $state();

  const EDGE = 56;
  function target(x: number, y: number): Area | null {
    if (x < EDGE) return "left";
    if (x > window.innerWidth - EDGE) return "right";
    if (y > window.innerHeight - EDGE - 24) return "bottom";
    return null;
  }

  function down(e: PointerEvent) {
    if (e.button !== 0 || (e.target as Element).closest("button")) return;
    e.preventDefault();
    const start = { x: e.clientX, y: e.clientY };
    const r = el!.getBoundingClientRect();
    let off = { x: e.clientX - r.left, y: e.clientY - r.top };
    let moved = false;
    // on the window: a docked pane that detaches is drawn anew as a floating one, and this
    // title bar goes away
    const move = (m: PointerEvent) => {
      if (!moved && Math.hypot(m.clientX - start.x, m.clientY - start.y) < 6) return;
      const now = docks.pane(id);
      if (!moved) {
        moved = true;
        if (!now.floating) {
          // detached at the pointer, at its floating size
          off = { x: Math.min(off.x, now.w - 40), y: Math.min(off.y, 14) };
          docks.float(id, { x: m.clientX - off.x, y: m.clientY - off.y, w: now.w, h: now.h });
        }
      }
      docks.set(id, { x: Math.max(0, m.clientX - off.x), y: Math.max(0, m.clientY - off.y) });
      docks.drag = { id, target: target(m.clientX, m.clientY) };
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      const t = docks.drag?.target ?? null;
      docks.drag = null;
      if (moved && t) docks.dock(id, t);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  function dblclick(e: MouseEvent) {
    if ((e.target as Element).closest("button")) return;
    if (p.floating) docks.dock(id, p.area);
    else {
      const r = el!.getBoundingClientRect();
      docks.float(id, { x: r.left + 30, y: r.top + 30, w: p.w, h: p.h });
    }
  }

  // a floating pane keeps the size it is given (CSS resize)
  $effect(() => {
    if (!el || !p.floating) return;
    const node = el;
    const ro = new ResizeObserver(() => {
      const w = Math.round(node.offsetWidth);
      const h = Math.round(node.offsetHeight);
      if (docks.pane(id).floating && (w !== docks.pane(id).w || h !== docks.pane(id).h)) docks.set(id, { w, h });
    });
    ro.observe(node);
    return () => ro.disconnect();
  });

  const fold = $derived(p.area === "left" ? (p.collapsed ? "›" : "‹") : p.area === "right" ? (p.collapsed ? "‹" : "›") : p.collapsed ? "˄" : "˅");
</script>

<section
  class="pane"
  class:floating={p.floating}
  class:collapsed={p.collapsed && !p.floating}
  bind:this={el}
  style:left={p.floating ? `${p.x}px` : null}
  style:top={p.floating ? `${p.y}px` : null}
  style:width={p.floating ? `${p.w}px` : null}
  style:height={p.floating ? `${p.h}px` : null}
  data-pane={id}
>
  <!-- svelte-ignore a11y_no_static_element_interactions (the buttons are reachable; dragging is the mouse's) -->
  <header aria-label={title} onpointerdown={down} ondblclick={dblclick} title="Drag to move the pane; double-click to float it or dock it again">
    <span class="title">{title}</span>
    {#if !p.floating}
      <button title={p.collapsed ? "Unfold" : "Fold"} aria-label={p.collapsed ? "Unfold" : "Fold"} data-act="fold" onclick={() => docks.set(id, { collapsed: !p.collapsed })}>{fold}</button>
    {/if}
    <button
      title={p.floating ? "Dock" : "Float"}
      aria-label={p.floating ? "Dock" : "Float"}
      data-act="float"
      onclick={() => (p.floating ? docks.dock(id, p.area) : docks.float(id, { x: (el?.getBoundingClientRect().left ?? 100) + 30, y: (el?.getBoundingClientRect().top ?? 100) + 30, w: p.w, h: p.h }))}
      >{p.floating ? "⊡" : "⧉"}</button
    >
    <button title="Close (View brings it back)" aria-label="Close" data-act="close" onclick={() => docks.set(id, { visible: false })}>×</button>
  </header>
  {#if !p.collapsed || p.floating}
    <div class="body">{@render children()}</div>
  {/if}
</section>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    background: var(--window);
    color: var(--text);
  }
  .pane.floating {
    /* the stored size includes the border: otherwise each size read back is 2 px larger, is stored,
       and the pane grows without end */
    box-sizing: border-box;
    position: fixed;
    z-index: 35;
    border: 1px solid var(--mid);
    border-radius: 4px;
    box-shadow: 0 6px 24px rgb(0 0 0 / 30%);
    resize: both;
    overflow: hidden;
    min-width: 140px;
    min-height: 80px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: none;
    height: 24px;
    padding: 0 2px 0 8px;
    background: var(--alternate);
    border-bottom: 1px solid var(--mid);
    cursor: grab;
    user-select: none;
  }
  .title {
    flex: 1;
    font-weight: 600;
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  header button {
    width: 20px;
    height: 20px;
    padding: 0;
    font: inherit;
    line-height: 1;
    color: var(--text);
    background: none;
    border: 0;
    border-radius: 3px;
    cursor: pointer;
  }
  header button:hover {
    background: var(--button);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
</style>
