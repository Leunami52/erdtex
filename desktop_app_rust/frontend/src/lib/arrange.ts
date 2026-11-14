// Arranging a diagram automatically (the root README, "Arranging a diagram automatically"): candidate
// layouts of the core's graph (`arrange.rs`), computed with WebCola. The core measures each candidate on the drawing and the best one
// is taken. No imports of our own: runs in a worker, and in Node.js for the check harness
// (spike/arrange_check.mjs).
//
// A candidate: (anew: WebCola's constraint layout first, with attribute lists as nodes of their own;)
// each attribute list's side chosen; a local improvement against crossings and lines through boxes;
// overlaps removed with WebCola's VPSC solver, moving as little as possible, level by level (inside each
// subdomain box first, then the boxes as wholes), generalisations kept top to bottom; the lines that
// still run through a box routed around it (WebCola's visibility graph).
//
// The graph is in cm with y up; here mm with y down.
import { Constraint, Layout, Solver, Variable } from "webcola";

export type Rect = [number, number, number, number];
export interface GraphList {
  id: string;
  chen: boolean;
  current: string;
  sides: { anchor: string; rect: Rect }[];
}
export interface GraphNode {
  id: string;
  kind: string;
  cx: number;
  cy: number;
  w: number;
  h: number;
  lists: GraphList[];
}
export interface Graph {
  nodes: GraphNode[];
  /** `via`: its way points now (cm, y up); `curved`: `bend=` or `ortho`, relative to its ends */
  edges: { id: string; src: string; dst: string; label: string; via: [number, number][]; curved: boolean }[];
  /** `rect`: the box's outline now (cm, y up) */
  groups: { id: string; members: string[]; parent: string | null; rect?: Rect }[];
  above: [string, string][];
  /** connectors joining elements of different subdomain boxes, with the box their centre is in now: in
   * no box, placed between their elements (or, as a candidate, left in that box) */
  between?: [string, string | null][];
  /** the nodes the selection stands for */
  selected?: string[];
  /** (set here, not by the core) the nodes that stay where they are, and whether boxes without elements
   * stay too (arranging the selection only) */
  fixed?: Set<string>;
  emptyBoxesStay?: boolean;
}
/** What the core writes back (see `arrange::Arrangement`). */
export interface Arrangement {
  nodes: Record<string, [number, number]>;
  lists: Record<string, string>;
  edges: Record<string, [number, number][]>;
  groups: Record<string, Rect>;
}
export interface Options {
  /** the gap between elements, cm */
  spacing: number;
  /** lay out anew (several starts) instead of tidying the current layout */
  anew: boolean;
  /** what is arranged: everything; only the selected elements (the rest stays); everything but the
   * selected elements (they stay where they are) */
  scope?: "all" | "selection" | "pin";
}

/** How much more a fixed element weighs than one that may move: the others make room. */
const FIXED = 1e4;

type P = [number, number];
const MM = 10;
const DIRS: Record<string, P> = {
  east: [1, 0], "north east": [Math.SQRT1_2, -Math.SQRT1_2], north: [0, -1], "north west": [-Math.SQRT1_2, -Math.SQRT1_2],
  west: [-1, 0], "south west": [-Math.SQRT1_2, Math.SQRT1_2], south: [0, 1], "south east": [Math.SQRT1_2, Math.SQRT1_2],
}; // y down

/** A side's box (cm, y up, relative to the shape's centre) in mm, y down. */
const sideBox = (r: Rect): Rect => [r[0] * MM, -r[3] * MM, r[2] * MM, -r[1] * MM];

const sideOf = (l: GraphList, sides: Record<string, string>) =>
  l.sides.find((x) => x.anchor === (sides[l.id] ?? l.current)) ?? l.sides[0];

/** A node's shape and its lists on their sides: the union box, relative to the shape's centre (mm, y down). */
function unionBox(n: GraphNode, sides: Record<string, string>, withLists = true): Rect {
  let u: Rect = [(-n.w * MM) / 2, (-n.h * MM) / 2, (n.w * MM) / 2, (n.h * MM) / 2];
  for (const l of n.lists) {
    if (!withLists && !l.chen) continue;
    const s = sideOf(l, sides);
    if (!s) continue;
    const b = sideBox(s.rect);
    u = [Math.min(u[0], b[0]), Math.min(u[1], b[1]), Math.max(u[2], b[2]), Math.max(u[3], b[3])];
  }
  return u;
}

function random(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// -- anew: WebCola's constraint layout ----------------------------------------------------------------

/** WebCola's layout from `start` (shape centres): links of lengths from the elements' sizes, attribute
 * lists as nodes tied to their node, subdomain boxes as groups, generalisations top to bottom. Returns the
 * shape centres and where the lists went. */
function colaLayout(g: Graph, start: Map<string, P>, sides: Record<string, string>, margin: number, stress: number) {
  const nodes: { x: number; y: number; width: number; height: number; fixed: number }[] = [];
  const index = new Map<string, number>();
  const offsets: P[] = [];
  for (const n of g.nodes) {
    const [x, y] = start.get(n.id)!;
    const b = unionBox(n, sides, false);
    index.set(n.id, nodes.length);
    offsets.push([(b[0] + b[2]) / 2, (b[1] + b[3]) / 2]);
    const pad = n.kind === "point" ? 0 : margin;
    nodes.push({ x: x + (b[0] + b[2]) / 2, y: y + (b[1] + b[3]) / 2, width: b[2] - b[0] + pad, height: b[3] - b[1] + pad, fixed: g.fixed?.has(n.id) ? 1 : 0 });
  }
  const size = (i: number) => Math.max(nodes[i].width, nodes[i].height);
  const links: { source: number; target: number; length: number }[] = [];
  for (const e of g.edges) {
    const a = index.get(e.src), b = index.get(e.dst);
    if (a !== undefined && b !== undefined) links.push({ source: a, target: b, length: (size(a) + size(b)) * 0.35 + margin });
  }
  const listNode = new Map<string, number>();
  const ownerOf = new Map<number, string>();
  for (const n of g.nodes) {
    for (const l of n.lists) {
      const s = sideOf(l, sides);
      if (l.chen || !s) continue;
      const b = sideBox(s.rect);
      const [ox, oy] = start.get(n.id)!;
      listNode.set(l.id, nodes.length);
      ownerOf.set(nodes.length, n.id);
      links.push({ source: index.get(n.id)!, target: nodes.length, length: Math.hypot((b[0] + b[2]) / 2, (b[1] + b[3]) / 2) + margin / 2 });
      nodes.push({ x: ox + (b[0] + b[2]) / 2, y: oy + (b[1] + b[3]) / 2, width: b[2] - b[0] + margin / 2, height: b[3] - b[1] + margin / 2, fixed: g.fixed?.has(n.id) ? 1 : 0 });
    }
  }
  const constraints = g.above
    .filter(([a, b]) => index.has(a) && index.has(b))
    .map(([a, b]) => ({ axis: "y", left: index.get(a)!, right: index.get(b)!, gap: (nodes[index.get(a)!].height + nodes[index.get(b)!].height) / 2 }));
  const used = g.groups.filter((x) => x.members.length || g.groups.some((c) => c.parent === x.id));
  const gi = new Map(used.map((x, i) => [x.id, i]));
  const groups = used.map((grp) => {
    const leaves = grp.members.map((m) => index.get(m)).filter((i): i is number => i !== undefined);
    for (const [k, owner] of ownerOf) if (grp.members.includes(owner)) leaves.push(k);
    return { leaves, groups: [] as number[], padding: margin };
  });
  for (const grp of used) if (grp.parent && gi.has(grp.parent)) groups[gi.get(grp.parent)!].groups.push(gi.get(grp.id)!);
  const layout = new Layout()
    .size([4000, 4000])
    .nodes(nodes)
    .links(links)
    .groups(groups as never) // leaves and groups by index, as WebCola takes them at run time
    .constraints(constraints)
    .avoidOverlaps(true)
    .handleDisconnected(false)
    .linkDistance(((l: { length: number }) => l.length) as never)
    .start(stress, 30, 60, 0, false, false);
  const laid = layout.nodes() as unknown as { x: number; y: number }[];
  const at = new Map<string, P>();
  for (const [id, i] of index) at.set(id, [laid[i].x - offsets[i][0], laid[i].y - offsets[i][1]]);
  const listAt = new Map<string, P>();
  for (const [id, i] of listNode) listAt.set(id, [laid[i].x, laid[i].y]);
  return { at, listAt };
}

// -- attribute list sides ----------------------------------------------------------------------------

/** Each list's side: away from its node's lines and from other elements, near where WebCola put it. */
function chooseSides(g: Graph, at: Map<string, P>, listAt: Map<string, P>): Record<string, string> {
  const sides: Record<string, string> = {};
  const boxes = g.nodes.map((n) => {
    const [x, y] = at.get(n.id)!;
    return { id: n.id, r: [x - (n.w * MM) / 2, y - (n.h * MM) / 2, x + (n.w * MM) / 2, y + (n.h * MM) / 2] as Rect };
  });
  for (const n of g.nodes) {
    const [x, y] = at.get(n.id)!;
    const dirs = g.edges
      .filter((e) => e.src === n.id || e.dst === n.id)
      .map((e) => at.get(e.src === n.id ? e.dst : e.src))
      .filter((p): p is P => !!p)
      .map(([px, py]) => {
        const d = Math.hypot(px - x, py - y) || 1;
        return [(px - x) / d, (py - y) / d] as P;
      });
    for (const l of n.lists) {
      if (l.chen || g.fixed?.has(n.id)) {
        sides[l.id] = l.current;
        continue;
      }
      const want = listAt.get(l.id);
      let best = l.current, bestCost = Infinity;
      for (const s of l.sides) {
        const b = sideBox(s.rect);
        const r: Rect = [x + b[0], y + b[1], x + b[2], y + b[3]];
        const [dx, dy] = DIRS[s.anchor] ?? [1, 0];
        let cost = 0;
        for (const [ex, ey] of dirs) cost += Math.max(0, ex * dx + ey * dy - 0.2) * 20; // a line leaves that way
        for (const o of boxes) {
          if (o.id === n.id) continue;
          const w = Math.min(r[2], o.r[2]) - Math.max(r[0], o.r[0]), h = Math.min(r[3], o.r[3]) - Math.max(r[1], o.r[1]);
          if (w > 0 && h > 0) cost += 3 + (w * h) / 200;
        }
        if (want) cost += Math.hypot(x + (b[0] + b[2]) / 2 - want[0], y + (b[1] + b[3]) / 2 - want[1]) / 30;
        if (s.anchor === l.current) cost -= 1; // the side it has, when it is as good
        if (cost < bestCost) {
          bestCost = cost;
          best = s.anchor;
        }
      }
      sides[l.id] = best;
    }
  }
  return sides;
}

// -- crossings and lines through boxes, as the local improvement counts them -------------------------------
function crosses(a: P, b: P, c: P, d: P) {
  const s = (p: P, q: P, r: P) => (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
  return s(a, b, c) * s(a, b, d) < 0 && s(c, d, a) * s(c, d, b) < 0;
}
function clips(a: P, b: P, r: Rect) {
  let t0 = 0, t1 = 1;
  const dx = b[0] - a[0], dy = b[1] - a[1];
  for (const [p, q] of [[-dx, a[0] - r[0]], [dx, r[2] - a[0]], [-dy, a[1] - r[1]], [dy, r[3] - a[1]]]) {
    if (p === 0) {
      if (q < 0) return false;
      continue;
    }
    const t = q / p;
    if (p < 0) {
      if (t > t1) return false;
      t0 = Math.max(t0, t);
    } else {
      if (t < t0) return false;
      t1 = Math.min(t1, t);
    }
  }
  return t0 < t1;
}

function boxesAt(g: Graph, at: Map<string, P>, sides: Record<string, string>, shrink: number) {
  return g.nodes.map((n) => {
    const [x, y] = at.get(n.id)!;
    const u = unionBox(n, sides);
    return { id: n.id, r: [x + u[0] + shrink, y + u[1] + shrink, x + u[2] - shrink, y + u[3] - shrink] as Rect };
  });
}

/** Crossings, lines through boxes and lines close together of straight lines between shape centres,
 * their length, and elements overlapping. */
function cost(g: Graph, at: Map<string, P>, sides: Record<string, string>) {
  const boxes = boxesAt(g, at, sides, -1.5);
  let overlaps = 0;
  for (let i = 0; i < boxes.length; i++)
    for (let j = i + 1; j < boxes.length; j++) {
      const [a, b] = [boxes[i].r, boxes[j].r];
      if (Math.min(a[2], b[2]) > Math.max(a[0], b[0]) && Math.min(a[3], b[3]) > Math.max(a[1], b[1])) overlaps++;
    }
  const segs: { e: { src: string; dst: string }; a: P; b: P }[] = [];
  for (const e of g.edges) {
    const a = at.get(e.src), b = at.get(e.dst);
    if (a && b) segs.push({ e, a, b });
  }
  let crossings = 0, through = 0, length = 0, close = 0;
  for (let i = 0; i < segs.length; i++) {
    const { e, a, b } = segs[i];
    length += Math.hypot(b[0] - a[0], b[1] - a[1]);
    for (const o of boxes) if (o.id !== e.src && o.id !== e.dst && clips(a, b, o.r)) through++;
    for (let j = i + 1; j < segs.length; j++) {
      const f = segs[j].e;
      const l = closeLength(g, at, { e, poly: [a, b] }, { e: f, poly: [segs[j].a, segs[j].b] });
      if (l > 3) close += STACKED + 3 * l;
      if (f.src === e.src || f.src === e.dst || f.dst === e.src || f.dst === e.dst) continue;
      if (crosses(a, b, segs[j].a, segs[j].b)) crossings++;
    }
  }
  return 30 * overlaps + close + 50 * crossings + THROUGH * through + length / MM;
}

/** Local improvement: swap elements of the same kind, move a relationship to the middle of its
 * neighbours, where that lowers crossings and lines through boxes; generalisation order and subdomain
 * membership stay. */
/** The items in an order of their own (seed 0: as they are). The greedy steps end up in different
 * places for different orders; a few orders give the core more to choose from. */
function shuffled<T>(items: T[], seed: number): T[] {
  if (!seed) return items;
  const r = random(seed), out = [...items];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(r() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

function improve(g: Graph, at: Map<string, P>, sides: Record<string, string>, seed = 0) {
  const groupOf = new Map<string, string>();
  for (const grp of g.groups) for (const m of grp.members) groupOf.set(m, grp.id);
  const ok = (p: Map<string, P>) => g.above.every(([a, b]) => !p.has(a) || !p.has(b) || p.get(a)![1] < p.get(b)![1]);
  let best = cost(g, at, sides);
  const nodes = shuffled(g.nodes.filter((n) => n.kind !== "point" && n.kind !== "note" && !g.fixed?.has(n.id)), seed);
  for (let round = 0; round < 3; round++) {
    let better = false;
    for (let i = 0; i < nodes.length; i++) {
      for (let j = i + 1; j < nodes.length; j++) {
        const a = nodes[i], b = nodes[j];
        if (a.kind !== b.kind || groupOf.get(a.id) !== groupOf.get(b.id)) continue;
        const pa = at.get(a.id)!, pb = at.get(b.id)!;
        at.set(a.id, pb);
        at.set(b.id, pa);
        const c = ok(at) ? cost(g, at, sides) : Infinity;
        if (c < best - 1e-6) {
          best = c;
          better = true;
        } else {
          at.set(a.id, pa);
          at.set(b.id, pb);
        }
      }
    }
    // a nudge sideways: lines fanning out of an element at nearly the same angle spread apart
    for (const n of nodes) {
      const old = at.get(n.id)!;
      for (const [dx, dy] of [[10, 0], [-10, 0], [0, 10], [0, -10], [20, 0], [-20, 0], [0, 20], [0, -20]]) {
        at.set(n.id, [old[0] + dx, old[1] + dy]);
        const c = ok(at) ? cost(g, at, sides) : Infinity;
        if (c < best - 1e-6) {
          best = c;
          better = true;
          break;
        }
        at.set(n.id, old);
      }
    }
    for (const n of nodes.filter((x) => x.kind === "relationship")) {
      const nb = g.edges.filter((e) => e.src === n.id || e.dst === n.id).map((e) => at.get(e.src === n.id ? e.dst : e.src)).filter((p): p is P => !!p);
      if (nb.length < 2) continue;
      const old = at.get(n.id)!;
      at.set(n.id, [nb.reduce((s, p) => s + p[0], 0) / nb.length, nb.reduce((s, p) => s + p[1], 0) / nb.length]);
      const c = ok(at) ? cost(g, at, sides) : Infinity;
      if (c < best - 1e-6) {
        best = c;
        better = true;
      } else at.set(n.id, old);
    }
    if (!better) break;
  }
}

// -- removing overlaps, level by level ---------------------------------------------------------------------
interface Item {
  id: string;
  /** a node (its id) or a subdomain box (its children) */
  children: Item[] | null;
  x: number;
  y: number;
  w: number;
  h: number;
  /** how hard it is to move (FIXED: it stays) */
  weight: number;
  /** a box's outline now (mm, y down), when it holds elements that stay: it doesn't shrink below it */
  keep?: Rect;
}

/** Moves an item, and a box's contents with it. */
function shift(it: Item, dx: number, dy: number) {
  it.x += dx;
  it.y += dy;
  for (const c of it.children ?? []) shift(c, dx, dy);
}

/** Separates the items of one level (VPSC: each overlapping pair apart on the axis that needs the
 * smaller move; `above` pairs one above the other), moving them as little as possible. */
function separate(items: Item[], above: [number, number][]) {
  const xc: [number, number, number][] = [], yc: [number, number, number][] = [];
  const seen = new Set<string>();
  for (const [a, b] of above) yc.push([a, b, (items[a].h + items[b].h) / 2]);
  for (let round = 0; round < 40; round++) {
    let added = false;
    for (let i = 0; i < items.length; i++) {
      for (let j = i + 1; j < items.length; j++) {
        const a = items[i], b = items[j];
        const nx = (a.w + b.w) / 2 - Math.abs(a.x - b.x), ny = (a.h + b.h) / 2 - Math.abs(a.y - b.y);
        if (nx <= 0.01 || ny <= 0.01 || seen.has(`${i},${j}`)) continue;
        seen.add(`${i},${j}`);
        added = true;
        if (nx < ny) {
          if (a.x <= b.x) xc.push([i, j, (a.w + b.w) / 2]);
          else xc.push([j, i, (a.w + b.w) / 2]);
        } else if (a.y <= b.y) yc.push([i, j, (a.h + b.h) / 2]);
        else yc.push([j, i, (a.h + b.h) / 2]);
      }
    }
    if (!added && round > 0) break;
    for (const [axis, cs] of [["x", xc], ["y", yc]] as const) {
      if (!cs.length) continue;
      const vars = items.map((it) => new Variable(axis === "x" ? it.x : it.y, it.weight));
      new Solver(vars, cs.map(([l, r, gap]) => new Constraint(vars[l], vars[r], gap))).solve();
      items.forEach((it, k) => {
        const d = vars[k].position() - (axis === "x" ? it.x : it.y);
        if (Math.abs(d) > 1e-9) shift(it, axis === "x" ? d : 0, axis === "y" ? d : 0);
      });
    }
  }
}

/** A box made to fit its contents, with room for its title at the top (and no smaller than its outline
 * now, when it holds elements that stay). */
function fit(it: Item, margin: number) {
  const kids = it.children ?? [];
  if (!kids.length) return;
  let x0 = Math.min(...kids.map((c) => c.x - c.w / 2)), x1 = Math.max(...kids.map((c) => c.x + c.w / 2));
  let y0 = Math.min(...kids.map((c) => c.y - c.h / 2)) - margin, y1 = Math.max(...kids.map((c) => c.y + c.h / 2));
  if (it.keep) [x0, y0, x1, y1] = [Math.min(x0, it.keep[0]), Math.min(y0, it.keep[1]), Math.max(x1, it.keep[2]), Math.max(y1, it.keep[3])];
  Object.assign(it, { x: (x0 + x1) / 2, y: (y0 + y1) / 2, w: x1 - x0 + margin, h: y1 - y0 + margin });
}

/** The boxes of a level fitted to their contents (separated first), then the level separated. */
function settle(level: Item[], above: [string, string][], margin: number, path: Map<string, string[]>) {
  for (const it of level) {
    if (!it.children) continue;
    settle(it.children, above, margin, path);
    fit(it, margin);
  }
  const index = new Map(level.map((it, i) => [it.id, i]));
  const pairs: [number, number][] = [];
  for (const [a, b] of above) {
    // the items of this level that contain a and b (a generalisation across boxes: between those boxes)
    const pa = path.get(a), pb = path.get(b);
    if (!pa || !pb) continue;
    const ia = pa.find((id) => index.has(id)), ib = pb.find((id) => index.has(id));
    if (ia !== undefined && ib !== undefined && ia !== ib) pairs.push([index.get(ia)!, index.get(ib)!]);
  }
  separate(level, pairs);
}

/** Overlaps removed: each node with its lists as one box (and the gap), subdomain boxes as wholes. */
function removeOverlaps(g: Graph, at: Map<string, P>, sides: Record<string, string>, margin: number) {
  const offset = new Map<string, P>();
  const items = new Map<string, Item>();
  for (const n of g.nodes) {
    const [x, y] = at.get(n.id)!;
    const u = unionBox(n, sides);
    const pad = n.kind === "point" ? 0 : margin;
    offset.set(n.id, [(u[0] + u[2]) / 2, (u[1] + u[3]) / 2]);
    const weight = g.fixed?.has(n.id) ? FIXED : 1;
    items.set(n.id, { id: n.id, children: null, x: x + (u[0] + u[2]) / 2, y: y + (u[1] + u[3]) / 2, w: u[2] - u[0] + pad, h: u[3] - u[1] + pad, weight });
  }
  const used = g.groups.filter((x) => x.members.length || g.groups.some((c) => c.parent === x.id));
  const groups = new Map(used.map((x) => [x.id, { id: `box:${x.id}`, children: [] as Item[], x: 0, y: 0, w: 0, h: 0, weight: 1 } as Item]));
  const parentOf = new Map<string, string>();
  for (const grp of used) {
    const box = groups.get(grp.id)!;
    for (const m of grp.members) {
      const it = items.get(m);
      if (it) {
        box.children!.push(it);
        parentOf.set(m, `box:${grp.id}`);
      }
    }
    if (grp.parent && groups.has(grp.parent)) {
      groups.get(grp.parent)!.children!.push(box);
      parentOf.set(`box:${grp.id}`, `box:${grp.parent}`);
    }
  }
  // a box without elements: an item of its own size, moved out of the way like an element
  const emptyStay = new Set<string>();
  for (const grp of g.groups) {
    if (groups.has(grp.id) || !grp.rect) continue;
    const [x0, y0, x1, y1] = [grp.rect[0] * MM, -grp.rect[3] * MM, grp.rect[2] * MM, -grp.rect[1] * MM];
    const weight = g.emptyBoxesStay ? FIXED : 1;
    const box: Item = { id: `box:${grp.id}`, children: [], x: (x0 + x1) / 2, y: (y0 + y1) / 2, w: x1 - x0 + margin, h: y1 - y0 + margin, weight };
    groups.set(grp.id, box);
    if (g.emptyBoxesStay) emptyStay.add(grp.id);
    if (grp.parent && used.some((x) => x.id === grp.parent)) {
      groups.get(grp.parent)!.children!.push(box);
      parentOf.set(box.id, `box:${grp.parent}`);
    }
  }
  const root: Item[] = [...items.values()].filter((it) => !parentOf.has(it.id));
  for (const [id, box] of groups) if (!parentOf.has(`box:${id}`)) root.push(box);
  // a box holding elements that stay: it stays too (it would move them), no smaller than its outline;
  // a box holding only such elements isn't written at all
  const fixedIn = (it: Item): [boolean, boolean] => {
    if (!it.children) return [it.weight === FIXED, it.weight === FIXED];
    const kids = it.children.map(fixedIn);
    return [kids.some(([any]) => any), kids.length > 0 && kids.every(([, all]) => all)];
  };
  const unchanged = new Set<string>();
  for (const grp of used) {
    const box = groups.get(grp.id)!;
    const [any, all] = fixedIn(box);
    if (!any) continue;
    box.weight = FIXED;
    if (grp.rect) box.keep = [grp.rect[0] * MM, -grp.rect[3] * MM, grp.rect[2] * MM, -grp.rect[1] * MM];
    if (all) unchanged.add(grp.id);
  }
  // each element's chain of containers, innermost first (for generalisations across boxes)
  const path = new Map<string, string[]>();
  for (const id of items.keys()) {
    const chain = [id];
    for (let p = parentOf.get(id); p; p = parentOf.get(p)) chain.push(p);
    path.set(id, chain);
  }
  settle(root, g.above, margin, path);
  // a connector between boxes: in the middle of its elements, once the boxes have settled; then the top
  // level separated again (the boxes move a little, so twice)
  const free = (g.between ?? []).map(([id]) => id).filter((id) => items.has(id) && !parentOf.has(id) && !g.fixed?.has(id));
  for (let round = 0; round < 2 && free.length; round++) {
    for (const id of free) {
      const it = items.get(id)!;
      const ends = g.edges
        .filter((e) => e.src === id || e.dst === id)
        .map((e) => (e.src === id ? e.dst : e.src))
        .filter((o) => items.has(o))
        .map((o) => [items.get(o)!.x - offset.get(o)![0], items.get(o)!.y - offset.get(o)![1]] as P);
      if (!ends.length) continue;
      const [cx, cy] = [ends.reduce((s, p) => s + p[0], 0) / ends.length, ends.reduce((s, p) => s + p[1], 0) / ends.length];
      shift(it, cx + offset.get(id)![0] - it.x, cy + offset.get(id)![1] - it.y);
    }
    settle(root, g.above, margin, path);
  }
  const out = new Map<string, P>();
  for (const [id, it] of items) out.set(id, [it.x - offset.get(id)![0], it.y - offset.get(id)![1]]);
  const rects = new Map<string, Rect>();
  for (const [id, box] of groups) if (!unchanged.has(id) && !emptyStay.has(id)) rects.set(id, [box.x - box.w / 2 + margin / 2, box.y - box.h / 2 + margin / 2, box.x + box.w / 2 - margin / 2, box.y + box.h / 2 - margin / 2]);
  return { at: out, groups: rects };
}

// -- after the overlaps: elements on crossing lines moved to free spots ---------------------------------------

/** The elements on lines that cross, or that run through an element (straight lines between centres). */
function troubled(g: Graph, at: Map<string, P>, sides: Record<string, string>): Set<string> {
  const boxes = boxesAt(g, at, sides, -1.5);
  const segs = g.edges.filter((e) => at.has(e.src) && at.has(e.dst)).map((e) => ({ e, a: at.get(e.src)!, b: at.get(e.dst)! }));
  const out = new Set<string>();
  for (let i = 0; i < segs.length; i++) {
    const { e, a, b } = segs[i];
    if (boxes.some((o) => o.id !== e.src && o.id !== e.dst && clips(a, b, o.r))) [e.src, e.dst].forEach((id) => out.add(id));
    for (let j = i + 1; j < segs.length; j++) {
      const f = segs[j].e;
      if (f.src === e.src || f.src === e.dst || f.dst === e.src || f.dst === e.dst) continue;
      if (crosses(a, b, segs[j].a, segs[j].b)) [e.src, e.dst, f.src, f.dst].forEach((id) => out.add(id));
    }
  }
  return out;
}

/** Elements on crossing lines (or lines through elements) moved, one by one, to a free spot where the
 * lines cost less: around the middle of its neighbours, next to each neighbour, mirrored across its two
 * neighbours. A spot is free if the element (with its lists and the gap) overlaps nothing there, lies
 * inside its own box and outside every other box. No overlaps come back, so nothing is separated again. */
export function relocate(g: Graph, at: Map<string, P>, sides: Record<string, string>, rects: Map<string, Rect>, margin: number) {
  const box = new Map<string, string>(); // a node's innermost box
  for (const grp of g.groups) for (const m of grp.members) box.set(m, grp.id);
  const parent = new Map(g.groups.map((grp) => [grp.id, grp.parent]));
  const chain = (id: string) => {
    const out = new Set<string>();
    for (let b = box.get(id); b; b = parent.get(b) ?? undefined) out.add(b);
    return out;
  };
  const union = (n: GraphNode, [x, y]: P): Rect => {
    const u = unionBox(n, sides);
    return [x + u[0], y + u[1], x + u[2], y + u[3]];
  };
  const apart = (a: Rect, b: Rect, gap: number) => a[2] + gap <= b[0] || b[2] + gap <= a[0] || a[3] + gap <= b[1] || b[3] + gap <= a[1];
  const within = (a: Rect, b: Rect) => a[0] >= b[0] && a[1] >= b[1] && a[2] <= b[2] && a[3] <= b[3];
  const free = (n: GraphNode, p: P, mine: Set<string>) => {
    const r = union(n, p);
    for (const o of g.nodes) if (o.id !== n.id && !apart(r, union(o, at.get(o.id)!), o.kind === "point" || n.kind === "point" ? 0 : margin)) return false;
    for (const [id, b] of rects) {
      if (mine.has(id) ? !within(r, b) : !apart(r, b, margin / 2)) return false;
    }
    return true;
  };
  const ok = () => g.above.every(([a, b]) => !at.has(a) || !at.has(b) || at.get(a)![1] < at.get(b)![1]);
  let best = cost(g, at, sides);
  for (let round = 0; round < 2; round++) {
    let better = false;
    for (const id of troubled(g, at, sides)) {
      const n = g.nodes.find((x) => x.id === id);
      if (!n || n.kind === "note" || g.fixed?.has(id)) continue;
      const nb = g.edges.filter((e) => e.src === id || e.dst === id).map((e) => at.get(e.src === id ? e.dst : e.src)).filter((p): p is P => !!p);
      if (!nb.length) continue;
      const old = at.get(id)!;
      const mine = chain(id);
      const spots: P[] = [];
      const [bx, by] = [nb.reduce((s, p) => s + p[0], 0) / nb.length, nb.reduce((s, p) => s + p[1], 0) / nb.length];
      const step = Math.max(n.w, n.h) * MM + margin;
      for (const k of [0, 0.6, 1.2, 2]) for (let d = 0; d < 8; d++) spots.push([bx + Math.cos((d * Math.PI) / 4) * k * step, by + Math.sin((d * Math.PI) / 4) * k * step]);
      for (const p of nb) for (let d = 0; d < 8; d++) spots.push([p[0] + Math.cos((d * Math.PI) / 4) * step * 1.3, p[1] + Math.sin((d * Math.PI) / 4) * step * 1.3]);
      if (nb.length === 2) {
        // mirrored across the line through its two neighbours
        const [a, b] = nb;
        const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
        const l2 = dx * dx + dy * dy || 1;
        const t = ((old[0] - a[0]) * dx + (old[1] - a[1]) * dy) / l2;
        const [fx, fy] = [a[0] + t * dx, a[1] + t * dy];
        spots.push([2 * fx - old[0], 2 * fy - old[1]]);
      }
      let pick: P | null = null;
      for (const p of spots) {
        if (!free(n, p, mine)) continue;
        at.set(id, p);
        const c = ok() ? cost(g, at, sides) : Infinity;
        if (c < best - 1e-6) {
          best = c;
          pick = p;
        }
        at.set(id, old);
      }
      if (pick) {
        at.set(id, pick);
        better = true;
      }
    }
    if (!better) break;
  }
}

// -- lines routed around elements, and apart from one another ---------------------------------------------

/** Lines closer than this (mm), in nearly the same direction, can't be told apart. */
const CLOSE = 2;
/** What lines on top of one another cost: far more than a crossing or a line through an element (the
 * core's score weighs them alike). */
const STACKED = 400;
/** What a line through an element costs: more than two crossings (a way round with two is better). */
const THROUGH = 150;

interface Obstacle {
  owner: string;
  list: boolean;
  r: Rect;
}
interface Line {
  e: { src: string; dst: string };
  poly: P[];
}

/** The shapes and attribute lists, each a box (mm, y down). */
function obstacles(g: Graph, at: Map<string, P>, sides: Record<string, string>): Obstacle[] {
  const out: Obstacle[] = [];
  for (const n of g.nodes) {
    const [x, y] = at.get(n.id)!;
    out.push({ owner: n.id, list: false, r: [x - (n.w * MM) / 2, y - (n.h * MM) / 2, x + (n.w * MM) / 2, y + (n.h * MM) / 2] });
    for (const l of n.lists) {
      const s = sideOf(l, sides);
      if (!s) continue;
      const b = sideBox(s.rect);
      out.push({ owner: n.id, list: true, r: [x + b[0], y + b[1], x + b[2], y + b[3]] });
    }
  }
  return out;
}

/** How long (mm) segment a-b runs within CLOSE of segment c-d, in nearly the same direction; the part
 * of a-b within `skip` of a point in `near` (the centre of an element both lines meet at) left out. */
function closeAlong(a: P, b: P, c: P, d: P, near: P[], skip: number[]): number {
  const la = Math.hypot(b[0] - a[0], b[1] - a[1]), lc = Math.hypot(d[0] - c[0], d[1] - c[1]);
  if (la < 1e-6 || lc < 1e-6) return 0;
  const [ux, uy] = [(b[0] - a[0]) / la, (b[1] - a[1]) / la];
  if (Math.abs(ux * (d[0] - c[0]) + uy * (d[1] - c[1])) / lc < 0.94) return 0; // more than 20° apart
  // quick: bounding boxes apart
  if (Math.min(a[0], b[0]) - CLOSE > Math.max(c[0], d[0]) || Math.min(c[0], d[0]) - CLOSE > Math.max(a[0], b[0])) return 0;
  if (Math.min(a[1], b[1]) - CLOSE > Math.max(c[1], d[1]) || Math.min(c[1], d[1]) - CLOSE > Math.max(a[1], b[1])) return 0;
  // c-d in a-b's frame: along (s) and across (t); the part of it within CLOSE across
  const s0 = (c[0] - a[0]) * ux + (c[1] - a[1]) * uy, s1 = (d[0] - a[0]) * ux + (d[1] - a[1]) * uy;
  const t0 = -(c[0] - a[0]) * uy + (c[1] - a[1]) * ux, t1 = -(d[0] - a[0]) * uy + (d[1] - a[1]) * ux;
  let lo = 0, hi = 1; // the part of c-d (as a fraction) with |t| < CLOSE
  if (Math.abs(t1 - t0) < 1e-9) {
    if (Math.abs(t0) >= CLOSE) return 0;
  } else {
    const fa = (-CLOSE - t0) / (t1 - t0), fb = (CLOSE - t0) / (t1 - t0);
    lo = Math.max(0, Math.min(fa, fb));
    hi = Math.min(1, Math.max(fa, fb));
    if (lo >= hi) return 0;
  }
  let from = Math.max(0, Math.min(s0 + lo * (s1 - s0), s0 + hi * (s1 - s0)));
  let to = Math.min(la, Math.max(s0 + lo * (s1 - s0), s0 + hi * (s1 - s0)));
  for (let k = 0; k < near.length && from < to; k++) {
    // the stretch of a-b within skip of the shared element's centre
    const p = near[k], r = skip[k];
    const along = (p[0] - a[0]) * ux + (p[1] - a[1]) * uy, off = Math.abs(-(p[0] - a[0]) * uy + (p[1] - a[1]) * ux);
    if (off >= r) continue;
    const half = Math.sqrt(r * r - off * off);
    const [x0, x1] = [along - half, along + half];
    if (x0 <= from) from = Math.max(from, x1);
    else if (x1 >= to) to = Math.min(to, x0);
  }
  return Math.max(0, to - from);
}

/** How long two lines run close together (mm), apart from near the elements they share. */
function closeLength(g: Graph, at: Map<string, P>, p: Line, q: Line): number {
  const shared = [p.e.src, p.e.dst].filter((id) => id === q.e.src || id === q.e.dst);
  const near = shared.map((id) => at.get(id)!);
  const skip = shared.map((id) => {
    const n = g.nodes.find((x) => x.id === id)!;
    return (Math.min(n.w, n.h) * MM) / 2 + 6; // the shorter side: never laxer than the core, which measures from the border
  });
  let sum = 0;
  for (let i = 1; i < p.poly.length; i++)
    for (let j = 1; j < q.poly.length; j++) sum += closeAlong(p.poly[i - 1], p.poly[i], q.poly[j - 1], q.poly[j], near, skip);
  return sum;
}

/** Whether a bend of `p` lies on or next to `q` (the lines would seem to join there). */
function touches(p: P[], q: P[]): boolean {
  for (let i = 1; i < p.length - 1; i++)
    for (let j = 1; j < q.length; j++) if (pointSegment(p[i], q[j - 1], q[j]) < 2 * CLOSE) return true;
  return false;
}

function pointSegment(p: P, a: P, b: P): number {
  const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
  const l2 = dx * dx + dy * dy;
  const t = l2 > 0 ? Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2)) : 0;
  return Math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy);
}

/** What a line costs where it is: elements it runs through, crossings, lines it runs close beside,
 * its length and bends (the weights of the core's score). Stops early above `limit`. */
function lineCost(g: Graph, at: Map<string, P>, line: Line, others: Line[], obs: Obstacle[], limit = Infinity): number {
  const { e, poly } = line;
  let c = 0;
  for (let i = 1; i < poly.length; i++) c += Math.hypot(poly[i][0] - poly[i - 1][0], poly[i][1] - poly[i - 1][1]) / MM;
  c += (poly.length - 2) * 0.5;
  for (const o of obs) {
    if (!o.list && (o.owner === e.src || o.owner === e.dst)) continue;
    const r: Rect = [o.r[0] - 1.5, o.r[1] - 1.5, o.r[2] + 1.5, o.r[3] + 1.5]; // a little clear of it: the grid rounds
    for (let i = 1; i < poly.length; i++)
      if (clips(poly[i - 1], poly[i], r)) {
        c += THROUGH;
        break;
      }
  }
  if (c > limit) return c;
  for (const q of others) {
    const f = q.e;
    if (f.src !== e.src && f.src !== e.dst && f.dst !== e.src && f.dst !== e.dst)
      for (let i = 1; i < poly.length; i++) for (let j = 1; j < q.poly.length; j++) if (crosses(poly[i - 1], poly[i], q.poly[j - 1], q.poly[j])) c += 50;
  }
  if (c > limit) return c;
  for (const q of others) {
    const l = closeLength(g, at, line, q);
    if (l > 3) c += STACKED + 3 * l;
    else if (touches(poly, q.poly) || touches(q.poly, poly)) c += STACKED;
  }
  return c;
}

/** Ways round: one or two bends at the corners of the elements near the line, at a few distances (so
 * that two lines past one corner don't lie on top of each other), and bulges to either side. */
function detours(a: P, b: P, obs: Obstacle[], clearance: number): P[][] {
  const pts: P[] = [];
  const x0 = Math.min(a[0], b[0]) - 25, x1 = Math.max(a[0], b[0]) + 25, y0 = Math.min(a[1], b[1]) - 25, y1 = Math.max(a[1], b[1]) + 25;
  for (const o of obs) {
    if (o.r[2] < x0 || o.r[0] > x1 || o.r[3] < y0 || o.r[1] > y1) continue;
    for (const k of [1, 2, 3.2]) {
      const m = clearance * k;
      pts.push([o.r[0] - m, o.r[1] - m], [o.r[2] + m, o.r[1] - m], [o.r[0] - m, o.r[3] + m], [o.r[2] + m, o.r[3] + m]);
    }
  }
  const len = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
  const [nx, ny] = [-(b[1] - a[1]) / len, (b[0] - a[0]) / len];
  for (const f of [0.25, 0.5, 0.75])
    for (const d of [-30, -18, -10, -5, 5, 10, 18, 30]) pts.push([a[0] + (b[0] - a[0]) * f + nx * d, a[1] + (b[1] - a[1]) * f + ny * d]);
  return pts.map((p) => [a, p, b]);
}

/** The bends of each line (cm, y up): straight where nothing is in the way; otherwise the cheapest
 * detour (`lineCost`), chosen line by line, twice over. With `kept` (where the elements were), the
 * user's way points move with their ends and stay where they still cost nothing, and curves stay. */
function route(g: Graph, at: Map<string, P>, sides: Record<string, string>, clearance: number, kept: Map<string, P> | null, seed = 0) {
  const obs = obstacles(g, at, sides);
  const lines: (Line & { id: string; fixed: boolean })[] = [];
  for (const e of g.edges) {
    const a = at.get(e.src), b = at.get(e.dst);
    if (!a || !b) continue;
    let poly: P[] = [a, b];
    let fixed = !e.id; // drawn by the packages: stays
    if (g.fixed?.has(e.src) && g.fixed?.has(e.dst)) {
      // between two elements that stay: stays as it is
      fixed = true;
      poly = [a, ...e.via.map(([x, y]) => [x * MM, -y * MM] as P), b];
    }
    if (kept && e.id) {
      if (e.via.length) {
        const moved = (id: string) => {
          const was = kept.get(id), now = at.get(id);
          return was && now ? [now[0] - was[0], now[1] - was[1]] : [0, 0];
        };
        const [ax, ay] = moved(e.src), [bx, by] = moved(e.dst);
        poly = [a, ...e.via.map(([x, y]) => [x * MM + (ax + bx) / 2, -y * MM + (ay + by) / 2] as P), b];
      } else if (e.curved) fixed = true;
    }
    lines.push({ e, poly, id: e.id, fixed });
  }
  const changed = new Set<string>();
  for (let pass = 0; pass < 2; pass++) {
    for (const line of shuffled(lines, seed)) {
      if (line.fixed) continue;
      const others = lines.filter((l) => l !== line);
      const [a, b] = [line.poly[0], line.poly[line.poly.length - 1]];
      let best = line.poly, bestCost = lineCost(g, at, line, others, obs);
      const straight = lineCost(g, at, { e: line.e, poly: [a, b] }, others, obs);
      if (straight <= bestCost) [best, bestCost] = [[a, b], straight];
      if (bestCost < 1 + Math.hypot(b[0] - a[0], b[1] - a[1]) / MM + 1e-6) {
        if (best !== line.poly) changed.add(line.id);
        line.poly = best;
        continue; // nothing in its way
      }
      const one = detours(a, b, obs, clearance)
        .map((poly) => ({ poly, cost: lineCost(g, at, { e: line.e, poly }, others, obs, bestCost) }))
        .sort((x, y) => x.cost - y.cost);
      if (one.length && one[0].cost < bestCost) [best, bestCost] = [one[0].poly, one[0].cost];
      // two bends: the best first bends, each with a second
      for (const first of one.slice(0, 6)) {
        const p = first.poly[1];
        for (const q of detours(p, b, obs, clearance).slice(0, 400)) {
          const poly: P[] = [a, p, q[1], b];
          const c = lineCost(g, at, { e: line.e, poly }, others, obs, bestCost);
          if (c < bestCost) [best, bestCost] = [poly, c];
        }
      }
      if (best !== line.poly) changed.add(line.id);
      line.poly = best;
    }
  }
  const edges: Record<string, P[]> = {};
  for (const l of lines) {
    if (!l.id || l.fixed) continue;
    if (!kept || changed.has(l.id) || l.poly.length > 2) edges[l.id] = l.poly.slice(1, -1).map(([x, y]) => [x / MM, -y / MM]);
  }
  return edges;
}

// -- candidates -------------------------------------------------------------------------------------------

function candidate(
  g: Graph,
  start: Map<string, P>,
  opts: Options,
  how: { stress: number; sides: boolean; improve: boolean; keepLines?: boolean; inBoxes?: boolean; seed?: number },
): Arrangement {
  const margin = opts.spacing * MM;
  // connectors between boxes left in the box they are in, instead of placed between
  if (how.inBoxes && g.between?.length) {
    const home = new Map(g.between.filter(([, b]) => b).map(([id, b]) => [id, b!]));
    g = { ...g, between: [], groups: g.groups.map((grp) => ({ ...grp, members: [...grp.members, ...[...home].filter(([, b]) => b === grp.id).map(([id]) => id)] })) };
  }
  // tidying keeps the generalisations' order only where the layout has it (it doesn't overturn what the
  // user arranged); laying out anew puts every supertype above
  if (how.stress === 0) g = { ...g, above: g.above.filter(([a, b]) => (start.get(a)?.[1] ?? 0) < (start.get(b)?.[1] ?? 0)) };
  let sides: Record<string, string> = Object.fromEntries(g.nodes.flatMap((n) => n.lists.map((l) => [l.id, l.current])));
  let at = new Map(start);
  let listAt = new Map<string, P>();
  if (how.stress > 0) ({ at, listAt } = colaLayout(g, start, sides, margin, how.stress));
  if (how.sides) sides = chooseSides(g, at, listAt);
  if (how.improve) improve(g, at, sides, how.seed);
  const free = removeOverlaps(g, at, sides, margin);
  if (how.improve) relocate(g, free.at, sides, free.groups, margin);
  // elements that stay aren't named at all: their text stays as it is (relative positions too)
  const nodes: Record<string, P> = {};
  for (const [id, [x, y]] of free.at) if (!g.fixed?.has(id)) nodes[id] = [x / MM, -y / MM];
  const lists: Record<string, string> = {};
  for (const n of g.nodes) if (!g.fixed?.has(n.id)) for (const l of n.lists) lists[l.id] = sides[l.id];
  const groups: Record<string, Rect> = {};
  for (const [id, r] of free.groups) groups[id] = [r[0] / MM, -r[3] / MM, r[2] / MM, -r[1] / MM];
  return { nodes, lists, edges: route(g, free.at, sides, margin / 3, how.keepLines ? start : null, how.seed), groups };
}

/** The candidates for the core to measure. Tidying keeps the layout recognisable; anew starts from
 * several places. */
export function candidates(g: Graph, opts: Options): { name: string; arrangement: Arrangement }[] {
  const current = new Map<string, P>(g.nodes.map((n) => [n.id, [n.cx * MM, -n.cy * MM]]));
  const selected = new Set(g.selected ?? []);
  if (opts.scope === "selection") {
    if (!selected.size) return [];
    g = { ...g, fixed: new Set(g.nodes.map((n) => n.id).filter((id) => !selected.has(id))), emptyBoxesStay: true };
  } else if (opts.scope === "pin" && selected.size) g = { ...g, fixed: selected };
  const out: { name: string; arrangement: Arrangement }[] = [];
  const add = (name: string, f: () => Arrangement) => {
    try {
      out.push({ name, arrangement: f() });
    } catch {
      // a candidate that couldn't be laid out: the others remain
    }
  };
  for (const keepLines of [true, false]) {
    const lines = keepLines ? "" : ", lines straightened";
    add(`tidy (overlaps only)${lines}`, () => candidate(g, current, opts, { stress: 0, sides: false, improve: false, keepLines }));
    add(`tidy, list sides${lines}`, () => candidate(g, current, opts, { stress: 0, sides: true, improve: false, keepLines }));
    add(`tidy, improved${lines}`, () => candidate(g, current, opts, { stress: 0, sides: true, improve: true, keepLines }));
    if (g.between?.length)
      add(`tidy, improved, connectors in their boxes${lines}`, () => candidate(g, current, opts, { stress: 0, sides: true, improve: true, keepLines, inBoxes: true }));
  }
  for (const seed of [1, 2, 3, 4]) add(`tidy, improved, order ${seed}`, () => candidate(g, current, opts, { stress: 0, sides: true, improve: true, seed }));
  if (opts.anew) {
    add("anew from the current layout", () => candidate(g, current, opts, { stress: 50, sides: true, improve: true }));
    const span = Math.sqrt(g.nodes.reduce((s, n) => s + (n.w * MM + 20) * (n.h * MM + 20), 0)) * 1.5;
    const [x0, y0] = [Math.min(...[...current.values()].map((p) => p[0])), Math.min(...[...current.values()].map((p) => p[1]))];
    for (let seed = 1; seed <= 4; seed++) {
      const r = random(seed);
      const start = new Map<string, P>(g.nodes.map((n) => [n.id, g.fixed?.has(n.id) ? current.get(n.id)! : [x0 + r() * span, y0 + r() * span]]));
      add(`anew, start ${seed}`, () => candidate(g, start, opts, { stress: 80, sides: true, improve: true }));
    }
  }
  return out;
}
