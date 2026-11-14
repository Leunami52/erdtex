<script lang="ts" module>
  export const MIN_HEIGHT = 54;
  export const MAX_LINES = 12;
</script>

<script lang="ts">
  // A label: several lines (Shift+Enter starts a new one; it becomes \\ in the file); Enter or
  // leaving the field commits.  It grows with its text up to MAX_LINES lines and scrolls beyond;
  // the grip below sizes it by hand, a double click on the grip fits it to the text again.
  import { changed, orig } from "../../lib/panel";

  let { value, tip, commit }: { value: string; tip: string; commit: (v: string) => void } = $props();
  let area: HTMLTextAreaElement;
  let manual = $state<number | null>(null);
  let start: { y: number; h: number } | null = null;

  function fit() {
    if (!area) return;
    if (manual !== null) {
      area.style.height = `${manual}px`;
      return;
    }
    const cs = getComputedStyle(area);
    const line = parseFloat(cs.lineHeight) || 16;
    const extra = parseFloat(cs.paddingTop) + parseFloat(cs.paddingBottom) + parseFloat(cs.borderTopWidth) + parseFloat(cs.borderBottomWidth);
    area.style.height = "0px";
    const want = area.scrollHeight + parseFloat(cs.borderTopWidth) + parseFloat(cs.borderBottomWidth);
    area.style.height = `${Math.min(Math.max(want, MIN_HEIGHT), MAX_LINES * line + extra)}px`;
  }

  $effect(() => {
    void value;
    void manual;
    fit();
  });

  function done() {
    if (changed(area)) commit(area.value);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      done();
    }
  }

  function grab(e: PointerEvent) {
    start = { y: e.clientY, h: area.offsetHeight };
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }

  function drag(e: PointerEvent) {
    if (start) manual = Math.round(Math.max(MIN_HEIGHT, Math.min(start.h + e.clientY - start.y, 900)));
  }
</script>

<div class="label">
  <textarea
    bind:this={area}
    use:orig={value}
    title={tip}
    data-field="label"
    spellcheck="false"
    oninput={fit}
    onkeydown={onKey}
    onblur={done}
  ></textarea>
  <div
    class="grip"
    role="separator"
    aria-orientation="horizontal"
    title="Drag to resize the field; double-click to fit the text again"
    data-testid="label-grip"
    onpointerdown={grab}
    onpointermove={drag}
    onpointerup={() => (start = null)}
    ondblclick={() => (manual = null)}
  ></div>
</div>

<style>
  .label {
    display: flex;
    flex-direction: column;
  }
  textarea {
    font: inherit;
    line-height: 1.35;
    resize: none;
    box-sizing: border-box;
    width: 100%;
    color: var(--text);
    background: var(--base);
    border: 1px solid var(--mid);
    border-radius: 3px;
    padding: 4px 5px;
  }
  .grip {
    height: 7px;
    cursor: ns-resize;
    background: linear-gradient(var(--dark), var(--dark)) center calc(50% - 1px) / 24px 1px no-repeat,
      linear-gradient(var(--dark), var(--dark)) center calc(50% + 1px) / 24px 1px no-repeat;
  }
</style>
