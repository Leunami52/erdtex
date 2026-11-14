<script lang="ts">
  // Help → Rule Books…: the rules built into the program, one page per rule (lib/rulebook.ts). It
  // doesn't block the window. Books and rules on the left (with a search), the page on the right: its
  // badge, what it applies to (the palette's symbols), the text, the examples drawn by the core as on
  // the canvas, and the related rules. Rule ids in the text are links; back and forward go through the
  // pages seen. The Problems panel opens a problem's rule here (`app.openRule`).
  import { tick, untrack } from "svelte";
  import type { App } from "../../lib/app.svelte";
  import { Diagram } from "../../lib/core";
  import { toolIcon } from "../../lib/icons";
  import { ELEMENT_TOOLS, KIND_NAMES, loadBooks, render, search, type Book, type Example, type Rule } from "../../lib/rulebook";

  let { app, close }: { app: App; close: () => void } = $props();

  let books = $state<Book[]>([]);
  let query = $state("");
  /** what is shown: a rule's id, or `book:<dir>` for a book's introduction */
  let history = $state<string[]>([]);
  let at = $state(-1);
  let page: HTMLDivElement | undefined = $state();
  let failed = $state("");

  void loadBooks()
    .then((b) => {
      books = b;
      const wanted = untrack(() => app.ruleBook.rule);
      go(wanted ?? (b[0] ? `book:${b[0].dir}` : ""));
    })
    .catch((e) => (failed = String(e)));

  // a rule asked for from elsewhere (the Problems panel) while the window is open
  $effect(() => {
    const wanted = app.ruleBook.rule;
    if (wanted && books.length && untrack(() => history[at]) !== wanted) untrack(() => go(wanted));
  });

  /** the books whose rules are listed (all collapsed at first) */
  let open = $state<Set<string>>(new Set());

  function toggle(dir: string) {
    const next = new Set(open);
    if (next.has(dir)) next.delete(dir);
    else next.add(dir);
    open = next;
  }

  // the book of the rule shown is open, wherever the rule was opened from
  $effect(() => {
    const dir = rule?.book;
    if (dir && !untrack(() => open).has(dir)) open = new Set([...untrack(() => open), dir]);
  });

  function go(target: string) {
    if (!target) return;
    history = [...history.slice(0, at + 1), target];
    at = history.length - 1;
    app.ruleBook.rule = target.startsWith("book:") ? null : target;
  }

  const current = $derived(history[at] ?? "");
  const rule: Rule | null = $derived(books.flatMap((b) => b.rules).find((r) => r.id === current) ?? null);
  const book: Book | null = $derived(
    current.startsWith("book:") ? (books.find((b) => `book:${b.dir}` === current) ?? null) : (books.find((b) => b.dir === rule?.book) ?? null),
  );
  const shown = $derived(rule ? render(rule.body, rule.kind) : book && current.startsWith("book:") ? render(book.intro) : { html: "", examples: [] as Example[] });
  const found = $derived(new Set(search(books, query).map((r) => r.id)));
  const titleOf = (id: string) => books.flatMap((b) => b.rules).find((r) => r.id === id)?.title ?? "";

  // the examples: drawn by the core once the page is in place
  $effect(() => {
    const examples = shown.examples;
    const dark = app.canvasDark;
    void tick().then(() => {
      if (!page) return;
      for (const el of page.querySelectorAll<HTMLElement>("[data-example]")) {
        const ex = examples[Number(el.dataset.example)];
        if (!ex) continue;
        if (ex.lang === "relations") {
          el.innerHTML = `<pre class="relations">${ex.text.replace(/&/g, "&amp;").replace(/</g, "&lt;")}</pre>`;
          continue;
        }
        el.innerHTML = drawn(ex, dark);
      }
    });
  });

  function drawn(ex: Example, dark: boolean): string {
    let d: Diagram | null = null;
    try {
      d = new Diagram(ex.text, ex.lang === "gsmd" ? "gsm" : "er");
      d.set_language(app.language);
      const b = d.bounds();
      const [defs, body] = d.drawing(dark, 2);
      if (b.length !== 4) return "";
      const pad = 8;
      const [x0, y0, x1, y1] = [b[0] - pad, b[1] - pad, b[2] + pad, b[3] + pad];
      const w = x1 - x0, h = y1 - y0;
      const scale = Math.min(1.3, 420 / w);
      return `<svg width="${w * scale}" height="${h * scale}" viewBox="0 0 ${w} ${h}"><defs>${defs}</defs><g transform="matrix(1 0 0 -1 ${-x0} ${y1})">${body}</g></svg>`;
    } catch (e) {
      return `<pre class="bad">${String(e)}</pre>`;
    } finally {
      d?.free();
    }
  }

  // links in the page
  function onPageClick(e: MouseEvent) {
    const a = (e.target as Element).closest<HTMLElement>("[data-rule]");
    if (!a) return;
    e.preventDefault();
    go(a.dataset.rule ?? "");
  }

  // the window can be moved by its title
  let pos = $state({ x: 120, y: 70 });
  let grab: { x: number; y: number } | null = null;
  function down(e: PointerEvent) {
    if ((e.target as Element).closest("button")) return;
    grab = { x: e.clientX - pos.x, y: e.clientY - pos.y };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (grab) pos = { x: Math.max(0, e.clientX - grab.x), y: Math.max(0, e.clientY - grab.y) };
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="window"
  role="dialog"
  aria-label="Rule Books"
  tabindex="-1"
  style:left="{pos.x}px"
  style:top="{pos.y}px"
  onkeydown={(e) => e.key === "Escape" && (e.stopPropagation(), close())}
  data-testid="rulebook"
>
  <div class="title" role="presentation" onpointerdown={down} onpointermove={move} onpointerup={() => (grab = null)}>
    <button class="nav" disabled={at <= 0} onclick={() => (at -= 1)} title="Back" aria-label="Back">‹</button>
    <button class="nav" disabled={at >= history.length - 1} onclick={() => (at += 1)} title="Forward" aria-label="Forward">›</button>
    <span>Rule Books</span>
    <button class="x" onclick={close} aria-label="Close">×</button>
  </div>
  <div class="main">
    <nav class="left">
      <input class="search" type="search" placeholder="Search the rules" bind:value={query} data-testid="rulebook-search" />
      <div class="list">
        {#each books as b (b.dir)}
          {@const rules = b.rules.filter((r) => found.has(r.id))}
          {#if !query || rules.length}
            {@const shown = !!query || open.has(b.dir)}
            <button
              class="book"
              class:current={current === `book:${b.dir}`}
              aria-expanded={shown}
              onclick={() => {
                // closed: open it at its introduction; open: its introduction, and closed if that is shown
                if (!open.has(b.dir) || current === `book:${b.dir}`) toggle(b.dir);
                if (current !== `book:${b.dir}`) go(`book:${b.dir}`);
              }}
              data-book={b.dir}><span class="chevron">{shown ? "▾" : "▸"}</span>{b.name}</button
            >
            {#each shown ? rules : [] as r (r.id)}
              <button class="rule" class:current={current === r.id} class:withdrawn={r.withdrawn} onclick={() => go(r.id)} data-rule-item={r.id}>
                <span class="id">{r.id}</span>
                {r.title}
              </button>
            {/each}
          {/if}
        {/each}
        {#if query && !found.size}<div class="none">No rule matches.</div>{/if}
      </div>
    </nav>
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="page" bind:this={page} onclick={onPageClick} data-testid="rulebook-page">
      {#if failed}
        <p class="bad">The rule books couldn't be loaded: {failed}</p>
      {:else if rule}
        <div class="crumb">{book?.name ?? ""}</div>
        <h2><span class="id">{rule.id}</span> {rule.title}</h2>
        <div class="facts">
          <span class="badge {rule.kind}">{KIND_NAMES[rule.kind]}</span>
          {#if rule.withdrawn}<span class="badge withdrawn">Withdrawn: no longer applies</span>{/if}
          <span class="applies">
            Applies to:
            {#each rule.applies as a}
              <span class="element" title={a}>
                {#if ELEMENT_TOOLS[a]}<svg width="30" height="24" viewBox="0 0 40 32">{@html toolIcon(ELEMENT_TOOLS[a], app.panelsDark)}</svg>{/if}
                {a}
              </span>
            {/each}
          </span>
          {#if rule.fix}<span class="fix">Fix offered: {rule.fix}</span>{/if}
        </div>
        <div class="text">{@html shown.html}</div>
        {#if rule.see.length}
          <p class="see">
            <b>See also.</b>
            {#each rule.see as s, i}{#if i},{/if} <button class="link" data-rule={s}>{s}</button> {titleOf(s)}{/each}
          </p>
        {/if}
      {:else if book}
        <h2>{book.name}</h2>
        <div class="text">{@html shown.html}</div>
        <ul class="contents">
          {#each book.rules as r (r.id)}
            <li><button class="link" data-rule={r.id}>{r.id}</button> {r.title}</li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
</div>

<style>
  .window {
    position: fixed;
    z-index: 30;
    width: min(1100px, calc(100vw - 100px));
    height: min(780px, calc(100vh - 80px));
    display: flex;
    flex-direction: column;
    background: var(--window);
    color: var(--text);
    border: 1px solid var(--mid);
    border-radius: 6px;
    box-shadow: 0 8px 30px rgb(0 0 0 / 35%);
    resize: both;
    overflow: hidden;
    outline: none;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 10px;
    font-weight: 600;
    border-bottom: 1px solid var(--mid);
    cursor: move;
    user-select: none;
  }
  .title span {
    flex: 1;
    margin-left: 6px;
  }
  button {
    font: inherit;
    color: var(--text);
  }
  .x,
  .nav {
    border: 0;
    background: none;
    font-size: 18px;
    line-height: 1;
    padding: 0 6px;
    cursor: pointer;
  }
  .nav:disabled {
    color: var(--disabled);
    cursor: default;
  }
  .main {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .left {
    width: 300px;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--mid);
  }
  .search {
    margin: 8px;
    font: inherit;
    color: var(--text);
    background: var(--base);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 3px 6px;
  }
  .list {
    overflow: auto;
    flex: 1;
    padding-bottom: 8px;
  }
  .book,
  .rule {
    display: block;
    width: 100%;
    text-align: left;
    border: 0;
    background: none;
    padding: 3px 10px;
    cursor: pointer;
  }
  .book {
    font-weight: 600;
    margin-top: 6px;
  }
  .rule {
    padding-left: 26px;
  }
  .chevron {
    display: inline-block;
    width: 14px;
    color: var(--disabled);
  }
  .current {
    background: color-mix(in srgb, var(--highlight) 22%, transparent);
  }
  .withdrawn {
    text-decoration: line-through;
    color: var(--disabled);
  }
  .id {
    font-family: monospace;
    color: var(--disabled);
    margin-right: 4px;
  }
  .page {
    flex: 1;
    overflow: auto;
    padding: 10px 18px 24px;
    line-height: 1.45;
  }
  .crumb {
    color: var(--disabled);
  }
  h2 {
    font-size: 17px;
    margin: 4px 0 8px;
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
    align-items: center;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--alternate);
  }
  .badge {
    border-radius: 3px;
    padding: 1px 7px;
    color: #fff;
    background: rgb(110, 110, 110);
  }
  .badge.error {
    background: rgb(215, 35, 35);
  }
  .badge.warning {
    background: rgb(232, 135, 0);
  }
  .badge.hint {
    background: rgb(45, 125, 220);
  }
  .badge.refused {
    background: rgb(150, 40, 150);
  }
  .badge.conversion {
    background: rgb(30, 130, 90);
  }
  .applies {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: center;
  }
  .element {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  .fix {
    color: var(--disabled);
  }
  .text :global(table) {
    border-collapse: collapse;
    margin: 8px 0;
  }
  .text :global(th),
  .text :global(td) {
    border: 1px solid var(--mid);
    padding: 3px 8px;
    text-align: left;
    vertical-align: top;
  }
  .text :global(th) {
    background: var(--button);
    font-weight: 600;
  }
  .text :global(code) {
    font-family: monospace;
    background: var(--alternate);
    padding: 0 3px;
    border-radius: 2px;
  }
  .text :global(figure.example) {
    margin: 10px 0 14px;
    display: flex;
    gap: 14px;
    align-items: center;
    flex-wrap: wrap;
  }
  .text :global(.example-part) {
    border: 1px solid var(--mid);
    border-radius: 4px;
    padding: 4px 8px 6px;
    background: var(--base);
  }
  .text :global(.caption) {
    font-size: 12px;
    color: var(--disabled);
    margin-bottom: 2px;
  }
  .text :global(figure.violates .caption),
  .text :global(figure.refused .caption) {
    color: rgb(215, 35, 35);
  }
  .text :global(figure.follows .caption),
  .text :global(figure.allowed .caption) {
    color: rgb(30, 140, 70);
  }
  .text :global(.arrow) {
    font-size: 22px;
    color: var(--disabled);
  }
  .text :global(pre.relations) {
    margin: 0;
    font-family: monospace;
  }
  .page :global(a),
  .link {
    color: var(--highlight);
  }
  .link {
    border: 0;
    background: none;
    padding: 0;
    text-decoration: underline;
    cursor: pointer;
    font-family: monospace;
  }
  .see {
    margin-top: 16px;
    padding-top: 8px;
    border-top: 1px solid var(--alternate);
  }
  .bad,
  .none {
    color: rgb(215, 35, 35);
    padding: 8px;
  }
  .none {
    color: var(--disabled);
  }
</style>
