<script lang="ts">
  import { untrack } from "svelte";
  import type { App } from "../../lib/app.svelte";
  import { MODES, type Mode } from "../../lib/theme";

  let { app }: { app: App } = $props();
  const app0 = untrack(() => app);
  let mode = $state<Mode>(app0.mode);
  let language = $state(app0.language);

  export function problem(): string | null {
    return null;
  }

  export async function apply(): Promise<void> {
    await app.setMode(mode);
    if (language !== app.language) await app.setLanguage(language);
  }
</script>

<div class="form">
  <label for="pref-mode">Display mode</label>
  <select id="pref-mode" bind:value={mode} data-testid="pref-mode">
    {#each Object.entries(MODES) as [value, text]}<option {value}>{text}</option>{/each}
  </select>
  <label for="pref-language">Diagram labels</label>
  <select id="pref-language" bind:value={language} data-testid="pref-language">
    <option value="english">English: IS_A, CAN_BE, PART_OF</option>
    <option value="german">German: IST_EIN*E, KANN_SEIN, TEIL_VON</option>
  </select>
  <p class="span">
    A dark editing window recolours diagrams on screen only: files, LaTeX and exports don't change. The labels
    apply to the canvas, preview and export. In your own LaTeX document, its language decides.
  </p>
</div>
