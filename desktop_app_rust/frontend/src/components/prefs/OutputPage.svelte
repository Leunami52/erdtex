<script lang="ts" module>
  export const DEFAULT_PREAMBLE = "\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}";
</script>

<script lang="ts">
  import { untrack } from "svelte";
  // The LaTeX preview's preamble, what an export and the relational schema start with.
  import type { App } from "../../lib/app.svelte";
  let { app }: { app: App } = $props();
  const app0 = untrack(() => app);
  const s = app0.settings;
  let preamble = $state(s["preview preamble"] ?? DEFAULT_PREAMBLE);
  let source = $state(s["export/source"] ?? "latex");
  let format = $state(s["export/format"] ?? "pdf");
  let dpi = $state(Number(s["export/dpi"] ?? 300));
  let grid = $state(s["export/grid"] === "true");
  let transparent = $state(s["export/transparent"] === "true");
  let rmFormat = $state(s["rm/format"] ?? "text");
  let rel = $state(s["rm/rel_naming"] ?? "entities");
  let fk = $state(s["rm/fk_naming"] ?? "plain");
  let marks = $state(s["rm/marks"] ?? "underscore");
  let types = $state((s["rm/text_types"] ?? "true") === "true");
  let quote = $state(s["rm/quote_all"] === "true");
  let drop = $state(s["rm/drop"] === "true");

  export function problem(): string | null {
    return null;
  }

  export async function apply(): Promise<void> {
    const b = (v: boolean) => (v ? "true" : "false");
    const pairs: [string, string][] = [
      ["preview preamble", preamble.trim()],
      ["export/source", source],
      ["export/format", format],
      ["export/dpi", String(Math.max(36, Math.min(2400, Math.round(dpi) || 300)))],
      ["export/grid", b(grid)],
      ["export/transparent", b(transparent)],
      ["rm/format", rmFormat],
      ["rm/rel_naming", rel],
      ["rm/fk_naming", fk],
      ["rm/marks", marks],
      ["rm/text_types", b(types)],
      ["rm/quote_all", b(quote)],
      ["rm/drop", b(drop)],
    ];
    for (const [k, v] of pairs) await app.setSetting(k, v);
  }
</script>

<h4>LaTeX preview and export</h4>
<p>Preamble lines, for example the font packages of your document. The diagram packages are loaded automatically.</p>
<textarea rows="4" style="width: 100%" bind:value={preamble} data-testid="pref-preamble"></textarea>
<button onclick={() => (preamble = DEFAULT_PREAMBLE)}>Default preamble</button>
<div class="form">
  <h4 class="span">Export starts with</h4>
  <label for="ex-source">Source</label>
  <select id="ex-source" bind:value={source}><option value="latex">the LaTeX result</option><option value="editor">the editor rendering</option></select>
  <label for="ex-format">Format</label>
  <select id="ex-format" bind:value={format} data-testid="pref-ex-format"><option value="pdf">PDF</option><option value="png">PNG</option><option value="svg">SVG</option></select>
  <label for="ex-dpi">PNG resolution (dpi)</label>
  <input id="ex-dpi" type="number" min="36" max="2400" bind:value={dpi} data-testid="pref-ex-dpi" />
  <label class="field"><input type="checkbox" bind:checked={grid} /> Include the grid</label>
  <label class="field"><input type="checkbox" bind:checked={transparent} /> Transparent background</label>
  <h4 class="span">Relational schema starts with</h4>
  <label for="rm-format">Format</label>
  <select id="rm-format" bind:value={rmFormat} data-testid="pref-rm-format">
    <option value="text">plain text</option><option value="latex">LaTeX</option><option value="postgresql">SQL: PostgreSQL</option>
    <option value="oracle">SQL: Oracle</option><option value="sqlite">SQL: SQLite</option>
  </select>
  <label for="rm-rel">Relationship relations</label>
  <select id="rm-rel" bind:value={rel}><option value="entities">Entity_relationship_Entity</option><option value="relationship">relationship name</option></select>
  <label for="rm-fk">Foreign keys</label>
  <select id="rm-fk" bind:value={fk}><option value="plain">key name</option><option value="relationship">relationship + key name</option></select>
  <label for="rm-marks">Marks in plain text</label>
  <select id="rm-marks" bind:value={marks}><option value="underscore">_key_ &nbsp;/foreign/</option><option value="tags">key (PK) &nbsp;foreign (FK)</option></select>
  <label class="field"><input type="checkbox" bind:checked={types} /> Data types in plain text</label>
  <label class="field"><input type="checkbox" bind:checked={quote} /> Quote all names in SQL</label>
  <label class="field"><input type="checkbox" bind:checked={drop} data-testid="pref-rm-drop" /> DROP TABLE statements first</label>
</div>
