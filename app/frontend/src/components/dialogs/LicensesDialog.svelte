<script lang="ts">
  // Help → About → Licences…: the program's licence, the LaTeX packages', the fonts', and those of
  // every library built into the program (Rust crates from tools/licenses.sh, JavaScript libraries
  // listed when the page is built). Loaded only when opened.
  import { untrack } from "svelte";
  import program from "../../../../../LICENSE?raw";
  import packages from "../../../../../latex_packages/LICENSE?raw";
  import fonts from "../../../../fonts/GUST-FONT-LICENSE.txt?raw";
  import rustDesktop from "../../../../licenses/rust-desktop.txt?raw";
  import rustCore from "../../../../licenses/rust-core.txt?raw";
  import rustPdf from "../../../../licenses/rust-pdf.txt?raw";
  import { IS_WEB } from "../../lib/backend";
  import type { App } from "../../lib/app.svelte";

  let { app, close }: { app: App; close: () => void } = $props();
  // the desktop app's crates, or the website's (the core and its PDF export)
  const rust = IS_WEB ? `${rustCore}\n\n${rustPdf}` : rustDesktop;
  untrack(() => app);

  const SECTIONS = [
    { key: "program", title: "ERDTeX (MIT)" },
    { key: "packages", title: "LaTeX packages (LPPL 1.3c)" },
    { key: "fonts", title: "Latin Modern fonts (GUST Font License)" },
    { key: "rust", title: "Libraries: Rust crates" },
    { key: "javascript", title: "Libraries: JavaScript" },
  ] as const;
  type Key = (typeof SECTIONS)[number]["key"];

  let current = $state<Key>("program");
  let javascript = $state("Loading…");
  void fetch("licenses-javascript.txt")
    .then((r) => (r.ok ? r.text() : Promise.reject(new Error(String(r.status)))))
    .then((t) => (javascript = t))
    .catch(() => (javascript = "The list of JavaScript libraries is written when the program is built; it is missing here."));

  const text = $derived({ program, packages, fonts, rust, javascript }[current]);
  let box: HTMLDivElement | undefined = $state();
  $effect(() => box?.focus());

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  }
</script>

<div class="backdrop" role="presentation">
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Licences" tabindex="-1" bind:this={box} onkeydown={onKey} data-testid="licenses">
    <div class="title">Licences</div>
    <div class="main">
      <div class="list" role="tablist">
        {#each SECTIONS as s (s.key)}
          <button role="tab" aria-selected={current === s.key} class:current={current === s.key} onclick={() => (current = s.key)} data-section={s.key}>{s.title}</button>
        {/each}
      </div>
      <pre class="text" data-testid="licenses-text">{text}</pre>
    </div>
    <div class="buttons">
      <button onclick={close} data-testid="licenses-close">Close</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 30%);
    display: grid;
    place-items: center;
    z-index: 40;
  }
  .dialog {
    width: min(1000px, calc(100vw - 80px));
    height: min(700px, calc(100vh - 80px));
    display: flex;
    flex-direction: column;
    background: var(--window);
    color: var(--text);
    border: 1px solid var(--mid);
    border-radius: 6px;
    box-shadow: 0 8px 30px rgb(0 0 0 / 35%);
    outline: none;
  }
  .title {
    padding: 8px 12px;
    font-weight: 600;
    border-bottom: 1px solid var(--mid);
  }
  .main {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 8px;
    padding: 8px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 260px;
  }
  .list button {
    text-align: left;
    border: 0;
    background: none;
    padding: 6px 8px;
    border-radius: 4px;
  }
  .list button.current {
    background: var(--highlight);
    color: #fff;
  }
  .text {
    flex: 1;
    margin: 0;
    overflow: auto;
    padding: 8px;
    background: var(--base);
    border: 1px solid var(--mid);
    border-radius: 4px;
    font-size: 12px;
    white-space: pre-wrap;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    padding: 8px 12px;
    border-top: 1px solid var(--mid);
  }
  button {
    font: inherit;
    color: var(--text);
    background: var(--button);
    border: 1px solid var(--mid);
    border-radius: 4px;
    padding: 4px 14px;
  }
</style>
