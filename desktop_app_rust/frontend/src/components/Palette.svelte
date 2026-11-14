<script lang="ts">
  // The tools of the active diagram's kind, in sections, with the Python app's icons.  The
  // tooltip names the tool and its shortcut.
  import type { App } from "../lib/app.svelte";
  import { palette, tool_tip } from "../lib/core";
  import { toolIcon } from "../lib/icons";
  import { display } from "../lib/keys";

  let { app, keysOf }: { app: App; keysOf: (id: string) => string } = $props();

  const sections = $derived.by(() => {
    const flat = palette(app.current.kind);
    const out: { title: string; tools: string[] }[] = [];
    for (let i = 0; i + 1 < flat.length; i += 2) {
      const [title, tool] = [flat[i], flat[i + 1]];
      if (out.at(-1)?.title !== title) out.push({ title, tools: [] });
      out.at(-1)?.tools.push(tool);
    }
    return out;
  });

  const tip = (tool: string) => {
    const keys = display(keysOf(`${app.current.kind}.${tool}`));
    return keys ? `${tool_tip(tool)} (${keys})` : tool_tip(tool);
  };
</script>

<aside class="palette" data-testid="palette">
  {#each sections as s}
    <div class="title">{s.title}</div>
    <div class="grid">
      {#each s.tools as tool}
        <button
          class:current={app.current.tool === tool}
          title={tip(tool)}
          aria-label={tool}
          data-tool={tool}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => (app.setTool(tool), app.focusCanvas())}
        >
          <svg width="40" height="32" viewBox="0 0 40 32">{@html toolIcon(tool, app.panelsDark)}</svg>
        </button>
      {/each}
    </div>
  {/each}
</aside>

<style>
  .palette {
    box-sizing: border-box;
    height: 100%;
    padding: 4px;
    background: var(--window);
    color: var(--text);
    overflow-y: auto;
    user-select: none;
  }
  .title {
    font-weight: bold;
    margin: 6px 2px 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* as many columns as the pane is wide for, down to one */
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, 46px);
    gap: 2px;
  }
  button {
    width: 46px;
    height: 38px;
    padding: 2px;
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
  }
  button:hover {
    background: var(--button);
  }
  button.current {
    background: var(--button);
    border-color: var(--highlight);
  }
</style>
