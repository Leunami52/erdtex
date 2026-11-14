import { describe, expect, it } from "vitest";
import { CM, PX_PER_PT, centreOn, fit, gridSteps, multiples, position, toDiagram, toScreen, zoomOf, zoomTo } from "./view";

describe("the view", () => {
  it("maps diagram points (y up) to the screen and back", () => {
    const v = { k: 2, tx: 100, ty: 50 };
    expect(toScreen(v, 10, 5)).toEqual([120, 40]);
    expect(toDiagram(v, 120, 40)).toEqual([10, 5]);
  });

  it("zooms around a point, which stays where it is", () => {
    const v = { k: PX_PER_PT, tx: 30, ty: 40 };
    const before = toDiagram(v, 200, 150);
    const z = zoomTo(v, 2.5, 200, 150);
    expect(zoomOf(z)).toBeCloseTo(2.5);
    const after = toDiagram(z, 200, 150);
    expect(after[0]).toBeCloseTo(before[0]);
    expect(after[1]).toBeCloseTo(before[1]);
    expect(zoomOf(zoomTo(v, 100, 0, 0))).toBe(20); // 5 % to 2000 %
    expect(zoomOf(zoomTo(v, 0, 0, 0))).toBe(0.05);
  });

  it("fits a diagram with 19 pt around it, and shows the origin at 100 % when empty", () => {
    const v = fit([0, 0, 162, 62], 400, 200);
    expect(v.k).toBeCloseTo(2); // (62 + 38) pt in 200 px
    const [cx, cy] = toDiagram(v, 200, 100);
    expect(cx).toBeCloseTo(81);
    expect(cy).toBeCloseTo(31);
    expect(fit(null, 400, 200)).toEqual(centreOn({ k: PX_PER_PT, tx: 0, ty: 0 }, 0, 0, 400, 200));
  });

  it("chooses the grid as the Python app does", () => {
    expect(gridSteps(0.3)).toEqual({ step: 5 * CM, minor: 0, label: 5 * (5 * CM) }); // 8.5 px per cm
    expect(gridSteps(0.8)).toEqual({ step: CM, minor: 0, label: 2 * CM }); // 22.8 px per cm
    expect(gridSteps(3)).toEqual({ step: CM, minor: CM / 2, label: CM }); // 85 px per cm
    expect(multiples(-1, 2.5, 1)).toEqual([-1, 0, 1, 2]);
    expect(position(-0.001, CM)).toBe("(0.00, 1.00) cm");
  });
});
