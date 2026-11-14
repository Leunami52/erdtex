<script lang="ts" module>
  export interface Item {
    label: string;
    action?: () => void;
    /** the shortcut shown next to it */
    keys?: string;
    disabled?: boolean;
    /** a tooltip, also on a greyed-out entry (why it can't be used) */
    tip?: string;
    checked?: boolean;
    /** the command whose icon it shows (where it isn't a toggle) */
    icon?: string;
    items?: Item[];
  }
  export type Entry = Item | "-";
  export interface Menu {
    label: string;
    items: Entry[];
  }
</script>

<script lang="ts">
  // The menu bar: click a menu to open it; while one is open, pointing at another opens that.
  // Esc or a click elsewhere closes it.
  import { actionIcon } from "../lib/actionIcons";

  let { menus }: { menus: Menu[] } = $props();
  let open = $state<number | null>(null);
  let sub = $state<number | null>(null);

  function run(item: Item) {
    if (item.disabled || item.items) return;
    open = null;
    sub = null;
    item.action?.();
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (open !== null && e.key === "Escape") {
      open = null;
      e.stopPropagation();
    }
  }}
  onpointerdown={(e) => {
    if (open !== null && !(e.target as Element).closest?.(".menubar")) open = null;
  }}
/>

<nav class="menubar" data-testid="menubar">
  {#each menus as menu, i}
    <div class="menu">
      <button
        class="title"
        class:open={open === i}
        onclick={() => ((open = open === i ? null : i), (sub = null))}
        onpointerenter={() => open !== null && ((open = i), (sub = null))}>{menu.label}</button
      >
      {#if open === i}
        <div class="dropdown" role="menu">
          {#each menu.items as item, j}
            {#if item === "-"}
              <hr />
            {:else}
              <div class="row-wrap" role="none" title={item.tip} onpointerenter={() => (sub = item.items ? j : null)}>
                <button class="row" role="menuitem" disabled={item.disabled} onclick={() => run(item)}>
                  <span class="check">{#if item.checked}✓{:else if item.icon}{@html actionIcon(item.icon)}{/if}</span>
                  <span class="label">{item.label}</span>
                  <span class="keys">{item.items ? "▸" : (item.keys ?? "")}</span>
                </button>
                {#if item.items && sub === j}
                  <div class="dropdown submenu" role="menu">
                    {#each item.items as s}
                      <button class="row" role="menuitem" disabled={s.disabled} onclick={() => run(s)}>
                        <span class="check">{s.checked ? "●" : ""}</span>
                        <span class="label">{s.label}</span>
                        <span class="keys"></span>
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</nav>

<style>
  .menubar {
    display: flex;
    background: var(--window);
    color: var(--text);
    border-bottom: 1px solid var(--mid);
    user-select: none;
  }
  .menu {
    position: relative;
  }
  button {
    font: inherit;
    color: inherit;
    background: none;
    border: none;
  }
  .title {
    padding: 4px 10px;
  }
  .title.open,
  .title:hover {
    background: var(--button);
  }
  .dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    min-width: 220px;
    background: var(--base);
    border: 1px solid var(--mid);
    box-shadow: 0 3px 10px rgb(0 0 0 / 25%);
    padding: 3px 0;
    z-index: 20;
  }
  .submenu {
    top: -4px;
    left: 100%;
  }
  .row-wrap {
    position: relative;
  }
  .row {
    display: flex;
    width: 100%;
    padding: 4px 12px 4px 6px;
    text-align: left;
    white-space: nowrap;
  }
  .row:hover:not(:disabled) {
    background: var(--highlight);
    color: #fff;
  }
  .row:disabled {
    color: var(--disabled);
  }
  .check {
    width: 18px;
    margin-right: 6px;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .label {
    flex: 1;
  }
  .keys {
    padding-left: 24px;
    opacity: 0.75;
  }
  hr {
    border: none;
    border-top: 1px solid var(--mid);
    margin: 3px 0;
  }
</style>
