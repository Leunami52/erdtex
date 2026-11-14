import { describe, expect, it } from "vitest";
import { candidates, relocate, type Graph, type GraphNode } from "./arrange";

const node = (id: string, cx: number, cy: number, kind = "entity", w = 2, h = 1): GraphNode => ({ id, kind, cx, cy, w, h, lists: [] });
const graph = (nodes: GraphNode[], extra: Partial<Graph> = {}): Graph => ({ nodes, edges: [], groups: [], above: [], ...extra });

/** Whether two nodes' boxes (cm) overlap at the arranged centres. */
function overlap(g: Graph, at: Record<string, [number, number]>, a: string, b: string) {
  const [na, nb] = [g.nodes.find((n) => n.id === a)!, g.nodes.find((n) => n.id === b)!];
  return Math.abs(at[a][0] - at[b][0]) < (na.w + nb.w) / 2 && Math.abs(at[a][1] - at[b][1]) < (na.h + nb.h) / 2;
}

describe("arranging", () => {
  it("tidying separates overlapping elements and moves them little", () => {
    const g = graph([node("a", 0, 0), node("b", 0.5, 0.2), node("c", 10, 10)]);
    const cs = candidates(g, { spacing: 0.5, anew: false });
    expect(cs.length).toBeGreaterThan(0);
    for (const { arrangement: a } of cs) {
      expect(overlap(g, a.nodes, "a", "b")).toBe(false);
      expect(a.nodes.c).toEqual([10, 10]); // in nobody's way: stays
      expect(Math.hypot(a.nodes.a[0], a.nodes.a[1])).toBeLessThan(3);
    }
  });

  it("keeps a supertype above its subtype", () => {
    const g = graph([node("sup", 0, 0), node("sub", 0.3, -0.2)], { above: [["sup", "sub"]] });
    for (const { arrangement: a } of candidates(g, { spacing: 0.5, anew: true })) {
      expect(a.nodes.sup[1]).toBeGreaterThan(a.nodes.sub[1]); // y up
      expect(overlap(g, a.nodes, "sup", "sub")).toBe(false);
    }
  });

  it("keeps subdomain members inside their box and others out", () => {
    const g = graph([node("in1", 0, 0), node("in2", 0.4, 0), node("out", 1, 0.3)], { groups: [{ id: "box", members: ["in1", "in2"], parent: null }] });
    for (const { arrangement: a } of candidates(g, { spacing: 0.5, anew: false })) {
      const [x0, y0, x1, y1] = a.groups.box;
      for (const id of ["in1", "in2"]) {
        const [x, y] = a.nodes[id];
        expect(x - 1 >= x0 - 1e-6 && x + 1 <= x1 + 1e-6 && y - 0.5 >= y0 - 1e-6 && y + 0.5 <= y1 + 1e-6).toBe(true);
      }
      const [x, y] = a.nodes.out;
      expect(x + 1 <= x0 || x - 1 >= x1 || y + 0.5 <= y0 || y - 0.5 >= y1).toBe(true);
    }
  });

  it("routes a line around an element in its way", () => {
    const g = graph([node("a", 0, 0), node("m", 5, 0), node("b", 10, 0)], { edges: [{ id: "e", src: "a", dst: "b", label: "", via: [], curved: false }] });
    const [first] = candidates(g, { spacing: 0.5, anew: false });
    expect(first.arrangement.edges.e.length).toBeGreaterThan(0);
  });

  it("leads two lines past one element apart, not on top of one another", () => {
    const edge = (id: string, src: string, dst: string) => ({ id, src, dst, label: "", via: [] as [number, number][], curved: false });
    const g = graph([node("a1", 0, 0.3), node("b1", 12, 0.3), node("a2", 0, -0.3, "entity", 2, 0.4), node("b2", 12, -0.3, "entity", 2, 0.4), node("m", 6, 0, "entity", 3, 3)], {
      edges: [edge("e1", "a1", "b1"), edge("e2", "a2", "b2")],
    });
    const [first] = candidates(g, { spacing: 0.5, anew: false });
    const { nodes, edges } = first.arrangement;
    const poly = (id: string, s: string, d: string) => [nodes[s], ...edges[id], nodes[d]];
    const [p, q] = [poly("e1", "a1", "b1"), poly("e2", "a2", "b2")];
    // sample p every 0.1 cm: never within 0.2 cm of q, away from the ends
    const dist = (x: [number, number], a: [number, number], b: [number, number]) => {
      const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
      const t = Math.max(0, Math.min(1, ((x[0] - a[0]) * dx + (x[1] - a[1]) * dy) / (dx * dx + dy * dy || 1)));
      return Math.hypot(x[0] - a[0] - t * dx, x[1] - a[1] - t * dy);
    };
    let close = 0;
    for (let i = 1; i < p.length; i++)
      for (let t = 0; t < 1; t += 0.01) {
        const x: [number, number] = [p[i - 1][0] + t * (p[i][0] - p[i - 1][0]), p[i - 1][1] + t * (p[i][1] - p[i - 1][1])];
        if (Math.abs(x[0] - 6) > 4.5) continue;
        if (q.slice(1).some((b, j) => dist(x, q[j], b) < 0.2)) close++;
      }
    expect(edges.e1.length + edges.e2.length).toBeGreaterThan(0);
    expect(close).toBe(0);
  });

  it("places a relationship joining two boxes between them", () => {
    const g = graph([node("a", 0, 0), node("b", 10, 0), node("r", 2, -1.5, "relationship", 2, 1.4), node("c", 0, -3)], {
      edges: [
        { id: "e1", src: "a", dst: "r", label: "", via: [], curved: false },
        { id: "e2", src: "r", dst: "b", label: "", via: [], curved: false },
      ],
      groups: [
        { id: "L", members: ["a", "c"], parent: null },
        { id: "R", members: ["b"], parent: null },
      ],
      between: [["r", "L"]],
    });
    const best = candidates(g, { spacing: 0.5, anew: false }).find((c) => c.name === "tidy, improved")!.arrangement;
    const [x, y] = best.nodes.r;
    const inside = (r: [number, number, number, number]) => x > r[0] && x < r[2] && y > r[1] && y < r[3];
    expect(inside(best.groups.L) || inside(best.groups.R)).toBe(false);
    expect(x).toBeGreaterThan(best.groups.L[2]);
    expect(x).toBeLessThan(best.groups.R[0]);
  });

  it("arranges only the selected elements: the others aren't touched", () => {
    const g = graph([node("a", 0, 0), node("b", 0.5, 0.2), node("c", 1, -0.3), node("d", 10, 10)], { selected: ["b"] });
    for (const anew of [false, true]) {
      const cs = candidates(g, { spacing: 0.5, anew, scope: "selection" });
      expect(cs.length).toBeGreaterThan(0);
      for (const { arrangement: a } of cs) {
        expect(Object.keys(a.nodes)).toEqual(["b"]); // only b is written
        const at = { a: [0, 0] as [number, number], c: [1, -0.3] as [number, number], b: a.nodes.b };
        expect(overlap({ ...g }, at, "a", "b") || overlap(g, at, "c", "b")).toBe(false);
      }
    }
    expect(candidates({ ...g, selected: [] }, { spacing: 0.5, anew: false, scope: "selection" })).toEqual([]);
  });

  it("keeps the selected elements in place and arranges the rest around them", () => {
    const g = graph([node("a", 0, 0), node("b", 0.5, 0.2), node("c", 6, 6)], { selected: ["a"] });
    for (const { arrangement: a } of candidates(g, { spacing: 0.5, anew: true, scope: "pin" })) {
      expect(a.nodes.a).toBeUndefined();
      expect(overlap(g, { a: [0, 0], b: a.nodes.b }, "a", "b")).toBe(false);
    }
  });

  it("leaves a box alone when everything in it stays", () => {
    const g = graph([node("in", 0, 0), node("x", 10, 0), node("y", 10.3, 0.1)], {
      groups: [{ id: "box", members: ["in"], parent: null, rect: [-3, -3, 3, 3] }],
      selected: ["x", "y"],
    });
    for (const { arrangement: a } of candidates(g, { spacing: 0.5, anew: false, scope: "selection" })) {
      expect(a.groups.box).toBeUndefined();
      expect(a.nodes.in).toBeUndefined();
    }
  });

  it("moves a box without elements out of the way (and leaves it when arranging the selection)", () => {
    const g = graph([node("a", 0, 0), node("b", 0.4, 0)], { groups: [{ id: "empty", members: [], parent: null, rect: [-1, -1, 1, 1] }], selected: ["a"] });
    const [first] = candidates(g, { spacing: 0.5, anew: false });
    const [x0, y0, x1, y1] = first.arrangement.groups.empty;
    for (const id of ["a", "b"]) {
      const [x, y] = first.arrangement.nodes[id];
      expect(x + 1 <= x0 + 1e-6 || x - 1 >= x1 - 1e-6 || y + 0.5 <= y0 + 1e-6 || y - 0.5 >= y1 - 1e-6).toBe(true);
    }
    expect(x1 - x0).toBeCloseTo(2);
    const [sel] = candidates(g, { spacing: 0.5, anew: false, scope: "selection" });
    expect(sel.arrangement.groups.empty).toBeUndefined();
  });

  it("moves elements on crossing lines to free spots after the overlaps are gone", () => {
    // A–r–B and C–s–D cross in an X
    const edge = (id: string, src: string, dst: string) => ({ id, src, dst, label: "", via: [] as [number, number][], curved: false });
    const g = graph(
      [node("A", 0, 0), node("B", 8, 5), node("C", 0, 5), node("D", 8, 0), node("r", 4, 3, "relationship", 2, 1.4), node("s", 4, 1, "relationship", 2, 1.4)],
      { edges: [edge("e1", "A", "r"), edge("e2", "r", "B"), edge("e3", "C", "s"), edge("e4", "s", "D")] },
    );
    const crossings = (at: Record<string, [number, number]>) => {
      const side = (p: number[], q: number[], r: number[]) => (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
      const lines = g.edges.map((e) => [at[e.src], at[e.dst], e.src, e.dst] as const);
      let n = 0;
      for (let i = 0; i < lines.length; i++)
        for (let j = i + 1; j < lines.length; j++) {
          const [a, b, s1, d1] = lines[i], [c, d, s2, d2] = lines[j];
          if ([s1, d1].some((x) => x === s2 || x === d2)) continue;
          if (side(a, b, c) * side(a, b, d) < 0 && side(c, d, a) * side(c, d, b) < 0) n++;
        }
      return n;
    };
    // positions in mm, y down, as the steps use them; nothing overlaps
    const at = new Map(g.nodes.map((n) => [n.id, [n.cx * 10, -n.cy * 10] as [number, number]]));
    const cm = () => Object.fromEntries([...at].map(([id, [x, y]]) => [id, [x / 10, -y / 10] as [number, number]]));
    expect(crossings(cm())).toBeGreaterThan(0);
    relocate(g, at, {}, new Map(), 5);
    expect(crossings(cm())).toBe(0);
  });

  it("lays out anew from several starts", () => {
    const g = graph([node("a", 0, 0), node("b", 0, 0), node("c", 0, 0)], { edges: [{ id: "e", src: "a", dst: "b", label: "", via: [], curved: false }] });
    const names = candidates(g, { spacing: 0.5, anew: true }).map((c) => c.name);
    expect(names.filter((n) => n.startsWith("anew")).length).toBeGreaterThanOrEqual(4);
  });
});
