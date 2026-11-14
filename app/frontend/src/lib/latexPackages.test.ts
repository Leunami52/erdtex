import { describe, expect, it } from "vitest";
import { PACKAGES, packageVersion } from "./latexPackages";

describe("the built-in LaTeX packages", () => {
  it("reads a package's version from its \\ProvidesPackage line", () => {
    expect(packageVersion("\\ProvidesPackage{erdiagram}[2026/11/07 v3.0.2 ER diagrams]")).toBe("3.0.2");
    expect(packageVersion("\\ProvidesPackage{x}[2026/01/01 v10.1 X]")).toBe("10.1");
    expect(packageVersion("no such line")).toBe("");
  });

  it("knows both packages' versions", () => {
    expect(PACKAGES.map((p) => p.name)).toEqual(["erdiagram", "gsmdiagram"]);
    for (const p of PACKAGES) expect(p.version).toMatch(/^\d+\.\d+\.\d+$/);
  });
});
