// The rule books (rulebook/, see its README): the pages loaded when the window first opens, their
// fields read (TOML), their text turned into HTML (Markdown). Examples become figures the window
// draws with the core; rule ids in the text become links.
import { Marked, type Tokens } from "marked";
import { parse as parseToml } from "smol-toml";

export interface Rule {
  id: string;
  title: string;
  kind: "error" | "warning" | "hint" | "refused" | "conversion" | "meaning";
  applies: string[];
  fix: string;
  see: string[];
  withdrawn: boolean;
  /** the book's folder */
  book: string;
  body: string;
}

export interface Book {
  dir: string;
  name: string;
  prefix: string;
  order: number;
  intro: string;
  rules: Rule[];
}

/** An example of a page: a diagram (or relations) and what it shows. */
export interface Example {
  lang: "erd" | "gsmd" | "relations";
  role: string;
  text: string;
}

const RULE_ID = /^(ER|GSM|DRAW|G2E|REL|NOT)-\d+\b/;

/** What a kind of rule says about the program, as the page's badge shows it. */
export const KIND_NAMES: Record<Rule["kind"], string> = {
  error: "Error",
  warning: "Warning",
  hint: "Hint",
  refused: "Refused while drawing",
  conversion: "Conversion",
  meaning: "Meaning",
};

/** The palette's tool for an element named in `applies` (its symbol), if it has one. */
export const ELEMENT_TOOLS: Record<string, string> = {
  "entity type": "entity",
  "weak entity type": "weak entity",
  "relationship type": "relationship",
  "weak relationship type": "weak relationship",
  attribute: "attribute tree",
  "attribute list": "attribute tree",
  "Chen attribute": "chen attribute",
  IS_A: "is a",
  CAN_BE: "can be",
  PART_OF: "part of",
  line: "link",
  "junction point": "point",
  note: "note",
  subdomain: "subdomain",
  "abstract type": "abstract type",
  "representable type": "representable type",
  subtype: "subtype",
  aggregation: "aggregation",
  grouping: "grouping",
  "attribute line": "attribute",
  "multi-valued attribute line": "multi-valued attribute",
  "component line": "component",
  generalization: "generalization",
  specialization: "specialization",
};

/** The fields block (`+++ … +++`) and the text of a page. */
export function split(text: string): { fields: Record<string, unknown>; body: string } {
  const m = /^\+\+\+\n([\s\S]*?)\n\+\+\+\n([\s\S]*)$/.exec(text.replace(/\r\n/g, "\n"));
  if (!m) return { fields: {}, body: text };
  return { fields: parseToml(m[1]) as Record<string, unknown>, body: m[2] };
}

const str = (v: unknown) => (typeof v === "string" ? v : "");
const list = (v: unknown) => (Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : []);

/** The books from their pages' texts, by path (`…/rulebook/<book>/<file>.md`). */
export function assemble(files: Record<string, string>): Book[] {
  const books = new Map<string, Book>();
  const rules: Rule[] = [];
  for (const [path, text] of Object.entries(files)) {
    const parts = path.split("/");
    const dir = parts[parts.length - 2];
    const { fields, body } = split(text);
    if (parts[parts.length - 1] === "index.md") {
      books.set(dir, { dir, name: str(fields.book), prefix: str(fields.prefix), order: Number(fields.order ?? 99), intro: body, rules: [] });
    } else {
      rules.push({
        id: str(fields.id),
        title: str(fields.title),
        kind: (str(fields.kind) || "meaning") as Rule["kind"],
        applies: list(fields.applies),
        fix: str(fields.fix),
        see: list(fields.see),
        withdrawn: fields.withdrawn === true,
        book: dir,
        body,
      });
    }
  }
  const number = (id: string) => Number(id.split("-")[1] ?? 0);
  for (const r of rules.sort((a, b) => number(a.id) - number(b.id))) books.get(r.book)?.rules.push(r);
  return [...books.values()].sort((a, b) => a.order - b.order);
}

let loaded: Promise<Book[]> | null = null;

/** The books, loaded once (the first time the window opens). */
export function loadBooks(): Promise<Book[]> {
  loaded ??= (async () => {
    const loaders = import.meta.glob("../../../rulebook/*/*.md", { query: "?raw", import: "default" });
    const files: Record<string, string> = {};
    await Promise.all(Object.entries(loaders).map(async ([p, load]) => (files[p] = (await load()) as string)));
    return assemble(files);
  })();
  return loaded;
}

const escape = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

const CAPTIONS: Record<string, string> = {
  follows: "✓ Follows the rule",
  violates: "✗ Breaks the rule",
  allowed: "✓ Allowed: the last line",
  refused: "✗ Refused: the last line",
  converts: "Given",
  result: "Becomes",
};

/** A page's text as HTML, with its examples as figures (`data-example`: an index into the list it
 * returns) and rule ids as links (`data-rule`). A `converts` example and the `result` after it share
 * one figure. */
export function render(body: string, ruleKind: Rule["kind"] | "" = ""): { html: string; examples: Example[] } {
  const examples: Example[] = [];
  let pending: number | null = null;
  const marked = new Marked({
    gfm: true,
    extensions: [
      {
        name: "ruleRef",
        level: "inline",
        start: (src: string) => src.search(/\b(ER|GSM|DRAW|G2E|REL|NOT)-\d/),
        tokenizer(src: string) {
          const m = RULE_ID.exec(src);
          return m ? { type: "ruleRef", raw: m[0], id: m[0] } : undefined;
        },
        renderer: (t) => `<a href="#" class="rule-ref" data-rule="${(t as unknown as { id: string }).id}">${(t as unknown as { id: string }).id}</a>`,
      },
    ],
    renderer: {
      code({ text, lang }: Tokens.Code) {
        const [kind, role = ""] = (lang ?? "").split(/\s+/);
        if (kind !== "erd" && kind !== "gsmd" && kind !== "relations") return `<pre><code>${escape(text)}</code></pre>`;
        examples.push({ lang: kind, role, text });
        const k = examples.length - 1;
        const caption = (role: string) => (ruleKind === "hint" && role === "violates" ? "ℹ Gets the hint" : (CAPTIONS[role] ?? ""));
        const one = (n: number) => `<div class="example-part"><div class="caption">${caption(examples[n].role)}</div><div data-example="${n}"></div></div>`;
        if (role === "converts") {
          pending = k;
          return "";
        }
        if (role === "result" && pending !== null) {
          const html = `<figure class="example pair">${one(pending)}<div class="arrow">→</div>${one(k)}</figure>`;
          pending = null;
          return html;
        }
        return `<figure class="example ${role}">${one(k)}</figure>`;
      },
    },
  });
  const html = marked.parse(body, { async: false }) as string;
  return { html, examples };
}

/** The rules matching a search: in the id, the title or the text. */
export function search(books: Book[], query: string): Rule[] {
  const q = query.trim().toLowerCase();
  const all = books.flatMap((b) => b.rules);
  if (!q) return all;
  return all.filter((r) => r.id.toLowerCase().includes(q) || r.title.toLowerCase().includes(q) || r.body.toLowerCase().includes(q));
}
