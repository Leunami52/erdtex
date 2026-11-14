// Tabs side by side (README, "Tabs side by side"): the groups of tabs, laid out by dockview, and what the menus,
// the keys and dragging do with them.  Tabs are dockview's panels, by the tab's id.  The page draws the
// groups (components/groups/Groups.svelte); this is the part without a window.
import type { DockviewApi, DockviewGroupPanel, IDockviewPanel } from "dockview-core";

export type Side = "left" | "right" | "top" | "bottom" | "center";

export class Groups {
  constructor(private readonly api: DockviewApi) {}

  private panel(id: number): IDockviewPanel | undefined {
    return this.api.getPanel(String(id));
  }

  /** The groups in the order they appear: left to right, then top to bottom. */
  ordered(): DockviewGroupPanel[] {
    const at = (g: DockviewGroupPanel) => g.element.getBoundingClientRect();
    return [...this.api.groups].sort((a, b) => at(a).left - at(b).left || at(a).top - at(b).top);
  }

  /** Each group's tabs (ids, in their order) and the one it shows. */
  list(): { tabs: number[]; active: number | null }[] {
    return this.ordered().map((g) => ({
      tabs: g.panels.map((p) => Number(p.id)),
      active: g.activePanel ? Number(g.activePanel.id) : null,
    }));
  }

  /** The tabs of the group a tab is in, in their order. */
  siblings(id: number): number[] {
    return this.panel(id)?.group.panels.map((p) => Number(p.id)) ?? [];
  }

  /** Whether a tab can split off its group: not its only tab (a diagram is in one tab only). */
  canSplit(id: number): boolean {
    return this.siblings(id).length > 1;
  }

  /** The tab into a new group at that side of its group. */
  split(id: number, side: "right" | "bottom"): boolean {
    const p = this.panel(id);
    if (!p || !this.canSplit(id)) return false;
    p.api.moveTo({ group: p.group, position: side });
    return true;
  }

  /** The tab moved by dragging: into `group` (its tab bar, at `index`), or split off at its side. */
  move(id: number, group: DockviewGroupPanel, side: Side, index?: number): void {
    const p = this.panel(id);
    if (!p) return;
    if (group === p.group) {
      if (side === "center" && index === undefined) return; // dropped on its own group
      if (side !== "center" && !this.canSplit(id)) return; // its group's only tab
      if (side === "center" && index !== undefined) {
        // within its own tab bar: `index` counts the tabs as they are, with this one still there
        const from = group.panels.indexOf(p);
        const to = index > from ? index - 1 : index;
        if (to === from) return;
        p.api.moveTo({ group, position: "center", index: to });
        return;
      }
    }
    p.api.moveTo({ group, position: side, index });
  }

  /** Whether there is another group to move a tab to, or one to split off into. */
  canMoveToOther(id: number): boolean {
    return this.api.groups.length > 1 || this.canSplit(id);
  }

  /** The tab into the next group, or, with one group, into a new one at the right. */
  moveToOther(id: number): boolean {
    const p = this.panel(id);
    if (!p) return false;
    const groups = this.ordered();
    if (groups.length < 2) return this.split(id, "right");
    const next = groups[(groups.indexOf(p.group) + 1) % groups.length];
    p.api.moveTo({ group: next, position: "center" });
    return true;
  }

  /** The layout, for keeping it (see keepLayout). */
  layout(): SavedLayout {
    return this.api.toJSON() as unknown as SavedLayout;
  }

  /** A layout from restoreLayout; false if dockview refused it (then the tabs stay in one group). */
  restore(layout: SavedLayout, width: number, height: number): boolean {
    try {
      this.api.fromJSON(layout as unknown as Parameters<DockviewApi["fromJSON"]>[0]);
      this.api.layout(width, height);
      return true;
    } catch {
      return false;
    }
  }

  /** The tab shown by the next (`step` 1) or previous (-1) group. */
  nextGroup(current: number, step: number): number | null {
    const groups = this.ordered();
    const here = this.panel(current)?.group;
    if (groups.length < 2 || !here) return null;
    const next = groups[(groups.indexOf(here) + step + groups.length) % groups.length];
    return next.activePanel ? Number(next.activePanel.id) : null;
  }

  /** The tab to show when a tab closes: its neighbour in its group (null: the group closes with it). */
  successor(id: number): number | null {
    const s = this.siblings(id);
    const i = s.indexOf(id);
    if (s.length < 2 || i < 0) return null;
    return s[i + 1] ?? s[i - 1] ?? null;
  }
}

// -- the layout kept for the next start --------------------------------------------------

/** A part of dockview's saved layout: a group (leaf) or a split (branch) of parts. */
interface GridNode {
  type: "leaf" | "branch";
  data: unknown;
  size?: number;
  visible?: boolean;
}

interface GroupData {
  views: string[];
  activeView?: string;
  id: string;
}

/** dockview's saved layout, as far as it is used here. */
export interface SavedLayout {
  grid: { root: GridNode; width: number; height: number; orientation: string };
  panels?: Record<string, unknown>;
  activeGroup?: string;
}

const total = (nodes: GridNode[]) => nodes.reduce((s, n) => s + (n.size ?? 0), 0);

/** A split's parts with their tabs renamed (`rename` null: the tab is left out): empty groups go,
 * and the others take the space they leave. */
function prunedParts(branch: GridNode, rename: (view: string) => string | null): GridNode[] {
  const parts = branch.data as GridNode[];
  const kept = parts.flatMap((p) => pruned(p, rename));
  const before = total(parts);
  const after = total(kept);
  if (after > 0 && before > 0 && Math.abs(after - before) > 0.5) {
    return kept.map((n) => ({ ...n, size: ((n.size ?? 0) * before) / after }));
  }
  return kept;
}

/** A part with its tabs renamed, as parts of its parent: none (it is empty now), itself, or, for a
 * split left with one split inside, that one's parts (they run along the parent's direction). */
function pruned(node: GridNode, rename: (view: string) => string | null): GridNode[] {
  if (node.type === "leaf") {
    const d = node.data as GroupData;
    const views = d.views.map(rename).filter((v): v is string => v !== null);
    if (!views.length) return [];
    const active = d.activeView === undefined ? null : rename(d.activeView);
    return [{ ...node, data: { ...d, views, activeView: active ?? views[0] } }];
  }
  const kept = prunedParts(node, rename);
  if (kept.length === 0) return [];
  if (kept.length > 1) return [{ ...node, data: kept }];
  const only = kept[0];
  return only.type === "leaf" ? [{ ...only, size: node.size }] : (only.data as GridNode[]);
}

/** The layout with its tabs renamed; null if no tab is left. */
function renamed(layout: SavedLayout, rename: (view: string) => string | null): SavedLayout | null {
  const root = layout.grid.root;
  const kept = root.type === "branch" ? prunedParts(root, rename) : pruned(root, rename);
  if (!kept.length) return null;
  let grid: SavedLayout["grid"] = { ...layout.grid, root: { ...root, type: "branch", data: kept } };
  // a root left with one split: that split becomes the root, which runs the other way
  if (kept.length === 1 && kept[0].type === "branch") {
    grid = { ...layout.grid, root: kept[0], orientation: layout.grid.orientation === "HORIZONTAL" ? "VERTICAL" : "HORIZONTAL" };
  }
  return { grid };
}

/** The layout as kept in the settings: the tabs are their files; tabs without a file are left out. */
export function keepLayout(layout: SavedLayout, fileOf: (tab: string) => string | null): SavedLayout | null {
  return renamed(layout, fileOf);
}

/** A kept layout for dockview again: the files that are open get their tab (`tabOf`, null: not open,
 * left out), and `panel` describes each tab. Null if none is open. */
export function restoreLayout(
  kept: SavedLayout,
  tabOf: (file: string) => string | null,
  panel: (tab: string) => Record<string, unknown>,
): SavedLayout | null {
  const layout = renamed(kept, tabOf);
  if (!layout) return null;
  const panels: Record<string, unknown> = {};
  const collect = (n: GridNode) => {
    if (n.type === "leaf") for (const v of (n.data as GroupData).views) panels[v] = panel(v);
    else (n.data as GridNode[]).forEach(collect);
  };
  collect(layout.grid.root);
  return { ...layout, panels };
}
