import { describe, expect, it } from "vitest";
import { assemble, loadBooks, render, search, split } from "./rulebook";

describe("the rule books", () => {
  it("reads a page's fields and text", () => {
    const { fields, body } = split('+++\nid = "ER-1"\napplies = ["entity type"]\n+++\n**Rule.** Text.\n');
    expect(fields.id).toBe("ER-1");
    expect(fields.applies).toEqual(["entity type"]);
    expect(body).toBe("**Rule.** Text.\n");
  });

  it("orders books and rules, by their order and number", () => {
    const books = assemble({
      "x/rulebook/b/index.md": '+++\nbook = "B"\nprefix = "REL"\norder = 2\n+++\n',
      "x/rulebook/a/index.md": '+++\nbook = "A"\nprefix = "ER"\norder = 1\n+++\nIntro',
      "x/rulebook/a/ER-10.md": '+++\nid = "ER-10"\ntitle = "Ten"\nkind = "hint"\n+++\n',
      "x/rulebook/a/ER-2.md": '+++\nid = "ER-2"\ntitle = "Two"\nkind = "error"\n+++\n',
    });
    expect(books.map((b) => b.name)).toEqual(["A", "B"]);
    expect(books[0].rules.map((r) => r.id)).toEqual(["ER-2", "ER-10"]);
    expect(search(books, "ten").map((r) => r.id)).toEqual(["ER-10"]);
  });

  it("makes examples into figures, a conversion's two side by side, and rule ids into links", () => {
    const body = "See ER-12 and REL-3.\n\n```erd violates ER-1\n\\entity{a}(0,0){A}\n```\n\n```gsmd converts\nx\n```\n\n```erd result\ny\n```\n";
    const { html, examples } = render(body);
    expect(examples.map((e) => [e.lang, e.role])).toEqual([["erd", "violates"], ["gsmd", "converts"], ["erd", "result"]]);
    expect(html).toContain('data-rule="ER-12"');
    expect(html).toContain('data-rule="REL-3"');
    expect(html).toContain('class="example violates"');
    expect(html.match(/class="example pair"/g)?.length).toBe(1);
    expect(html).toContain("✗ Breaks the rule");
  });

  it("loads the real books, each rule with its examples", async () => {
    const books = await loadBooks();
    expect(books.map((b) => b.prefix)).toEqual(["ER", "GSM", "DRAW", "G2E", "REL", "NOT"]);
    const all = books.flatMap((b) => b.rules);
    expect(all.length).toBeGreaterThanOrEqual(6);
    for (const r of all) {
      expect(r.title, r.id).not.toBe("");
      const { examples } = render(r.body);
      // a hint may be information alone, with one example; checks and conversions have at least two
      if (r.kind !== "meaning") expect(examples.length, r.id).toBeGreaterThanOrEqual(r.kind === "hint" ? 1 : 2);
    }
  });
});
