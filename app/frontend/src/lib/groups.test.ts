// The layout of the groups kept for the next start (keepLayout, restoreLayout): tabs without a file
// left out, files no longer there left out, and the splits around them made valid again.
import { describe, expect, it } from "vitest";
import { keepLayout, restoreLayout, type SavedLayout } from "./groups";

const leaf = (views: string[], size: number, activeView = views[0]) => ({ type: "leaf" as const, data: { views, activeView, id: views.join("") }, size });
const branch = (data: unknown[], size: number) => ({ type: "branch" as const, data, size });
const layout = (root: unknown, orientation = "HORIZONTAL"): SavedLayout =>
  ({ grid: { root, width: 1000, height: 600, orientation }, panels: {}, activeGroup: "x" }) as SavedLayout;
const files: Record<string, string | null> = { "1": "/a.erd", "2": "/b.erd", "3": null, "4": "/d.erd", "5": "/e.erd" };
const panel = (id: string) => ({ id, contentComponent: "canvas", tabComponent: "tab" });

describe("keepLayout", () => {
  it("keeps files instead of tabs, and leaves out tabs without a file", () => {
    const kept = keepLayout(layout(branch([leaf(["1", "2"], 600, "2"), leaf(["3"], 400)], 600)), (t) => files[t] ?? null);
    expect(kept?.grid.root).toEqual(branch([{ ...leaf(["/a.erd", "/b.erd"], 1000, "/b.erd"), data: { views: ["/a.erd", "/b.erd"], activeView: "/b.erd", id: "12" } }], 600));
    expect(kept?.panels).toBeUndefined();
  });
  it("gives null when no tab has a file", () => {
    expect(keepLayout(layout(branch([leaf(["3"], 1000)], 600)), (t) => files[t] ?? null)).toBeNull();
  });
});

describe("restoreLayout", () => {
  const open: Record<string, string> = { "/a.erd": "7", "/b.erd": "8", "/d.erd": "9" };
  const tabOf = (f: string) => open[f] ?? null;

  it("gives each file its tab, and describes the tabs", () => {
    const r = restoreLayout(layout(branch([leaf(["/a.erd"], 500), leaf(["/b.erd", "/d.erd"], 500, "/d.erd")], 600)), tabOf, panel);
    expect(r?.grid.root.data).toEqual([
      { ...leaf(["7"], 500), data: { views: ["7"], activeView: "7", id: "/a.erd" } },
      { ...leaf(["8", "9"], 500), data: { views: ["8", "9"], activeView: "9", id: "/b.erd/d.erd" } },
    ]);
    expect(Object.keys(r?.panels ?? {}).sort()).toEqual(["7", "8", "9"]);
  });

  it("drops a group whose files are gone; the others take its space", () => {
    const r = restoreLayout(layout(branch([leaf(["/a.erd"], 300), leaf(["/gone.erd"], 300), leaf(["/b.erd"], 400)], 600)), tabOf, panel);
    const parts = r?.grid.root.data as { size: number }[];
    expect(parts.map((p) => Math.round(p.size))).toEqual([429, 571]);
  });

  it("puts a group in the place of a split left with only it", () => {
    const r = restoreLayout(layout(branch([leaf(["/a.erd"], 500), branch([leaf(["/b.erd"], 300), leaf(["/gone.erd"], 300)], 500)], 600)), tabOf, panel);
    const parts = r?.grid.root.data as { type: string; size: number; data: { views: string[] } }[];
    expect(parts.map((p) => [p.type, p.size, p.data.views])).toEqual([["leaf", 500, ["7"]], ["leaf", 500, ["8"]]]);
  });

  it("joins the parts of a split left with one split to the parent, which runs the same way", () => {
    const inner = branch([leaf(["/b.erd"], 300), leaf(["/d.erd"], 300)], 600);
    const r = restoreLayout(layout(branch([leaf(["/a.erd"], 400), branch([inner, leaf(["/gone.erd"], 0)], 600)], 600)), tabOf, panel);
    const parts = r?.grid.root.data as { data: { views: string[] } }[];
    expect(parts.map((p) => p.data.views)).toEqual([["7"], ["8"], ["9"]]);
    expect(r?.grid.orientation).toBe("HORIZONTAL");
  });

  it("makes a split the root when it is all that is left, running the other way", () => {
    const r = restoreLayout(layout(branch([branch([leaf(["/a.erd"], 300), leaf(["/b.erd"], 300)], 700), leaf(["/gone.erd"], 300)], 600)), tabOf, panel);
    expect(r?.grid.orientation).toBe("VERTICAL");
    expect((r?.grid.root.data as { data: { views: string[] } }[]).map((p) => p.data.views)).toEqual([["7"], ["8"]]);
  });

  it("shows another tab of a group whose shown file is gone, and gives null when none is open", () => {
    const r = restoreLayout(layout(branch([leaf(["/a.erd", "/gone.erd"], 1000, "/gone.erd")], 600)), tabOf, panel);
    expect((r?.grid.root.data as { data: { activeView: string } }[])[0].data.activeView).toBe("7");
    expect(restoreLayout(layout(branch([leaf(["/gone.erd"], 1000)], 600)), tabOf, panel)).toBeNull();
  });
});
