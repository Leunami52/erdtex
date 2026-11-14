// The panes and where they are (as the Python app's dock widgets): docked at the left, the right
// or the bottom, floating, folded, or closed.  Kept in the settings as "dock layout".

export type PaneId = "tools" | "element" | "preview" | "problems";
export type Area = "left" | "right" | "bottom";

export interface Pane {
  visible: boolean;
  area: Area;
  floating: boolean;
  collapsed: boolean;
  /** share of its area, among the panes there */
  weight: number;
  /** its place in the area */
  order: number;
  /** where it floats (window pixels) */
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Layout {
  panes: Record<PaneId, Pane>;
  size: Record<Area, number>;
}

export const PANES: PaneId[] = ["tools", "element", "preview", "problems"];

const pane = (area: Area, order: number, visible = true): Pane => ({
  visible,
  area,
  floating: false,
  collapsed: false,
  weight: 1,
  order,
  x: 120 + 40 * order,
  y: 120 + 30 * order,
  w: 340,
  h: 420,
});

export function defaultLayout(): Layout {
  return {
    panes: {
      tools: pane("left", 0),
      element: pane("right", 0),
      preview: pane("right", 1, false), // as in the Python app: shown with View → LaTeX Preview
      problems: pane("bottom", 0),
    },
    size: { left: 160, right: 330, bottom: 150 },
  };
}

/** A stored layout, completed with the defaults (a broken one gives the defaults). */
export function readLayout(text: string | undefined): Layout {
  const d = defaultLayout();
  try {
    const l = JSON.parse(text ?? "") as Partial<Layout>;
    for (const id of PANES) d.panes[id] = { ...d.panes[id], ...(l.panes?.[id] ?? {}) };
    d.size = { ...d.size, ...(l.size ?? {}) };
  } catch {
    // the defaults
  }
  return d;
}

export class Docks {
  layout = $state<Layout>(defaultLayout());
  /** the pane being dragged by its title, and where it would dock */
  drag = $state<{ id: PaneId; target: Area | null } | null>(null);
  private save: (text: string) => void;
  private timer: ReturnType<typeof setTimeout> | undefined;

  constructor(save: (text: string) => void) {
    this.save = save;
  }

  load(text: string | undefined): void {
    this.layout = readLayout(text);
  }

  private changed(): void {
    clearTimeout(this.timer);
    this.timer = setTimeout(() => this.save(JSON.stringify(this.layout)), 300);
  }

  pane(id: PaneId): Pane {
    return this.layout.panes[id];
  }

  /** Visible and docked in `area`, in their order. */
  docked(area: Area): PaneId[] {
    return PANES.filter((id) => {
      const p = this.layout.panes[id];
      return p.visible && !p.floating && p.area === area;
    }).sort((a, b) => this.layout.panes[a].order - this.layout.panes[b].order);
  }

  floating(): PaneId[] {
    return PANES.filter((id) => this.layout.panes[id].visible && this.layout.panes[id].floating);
  }

  /** Whether all panes of an area are folded (the area is a narrow strip then). */
  folded(area: Area): boolean {
    const ids = this.docked(area);
    return ids.length > 0 && ids.every((id) => this.layout.panes[id].collapsed);
  }

  /** Shown with its contents (the preview compiles only then). */
  shown(id: PaneId): boolean {
    const p = this.layout.panes[id];
    return p.visible && (p.floating || !p.collapsed);
  }

  set(id: PaneId, change: Partial<Pane>): void {
    this.layout.panes[id] = { ...this.layout.panes[id], ...change };
    this.changed();
  }

  setSize(area: Area, size: number): void {
    this.layout.size[area] = size;
    this.changed();
  }

  toggle(id: PaneId): void {
    const p = this.pane(id);
    this.set(id, p.visible ? { visible: false } : { visible: true, collapsed: false });
  }

  /** Docked at the end of `area`. */
  dock(id: PaneId, area: Area): void {
    const last = Math.max(-1, ...this.docked(area).filter((i) => i !== id).map((i) => this.pane(i).order));
    this.set(id, { floating: false, area, order: last + 1, collapsed: false, visible: true });
  }

  float(id: PaneId, rect?: { x: number; y: number; w: number; h: number }): void {
    this.set(id, { floating: true, collapsed: false, ...(rect ?? {}) });
  }
}
