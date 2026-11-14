<script lang="ts">
  import { untrack } from "svelte";
  // What new diagrams of a kind start with: the main choices, then every style key by group.
  // An empty field means the packages' default, shown in grey.
  import type { App } from "../../lib/app.svelte";
  import { check_default, color_rgb, defaults_page } from "../../lib/core";

  interface Toggle {
    key: string;
    title: string;
    setting: string;
    choices: [string, string][];
    value: string;
  }
  interface Key {
    key: string;
    setting: string;
    default: string;
    type: string;
    value: string;
  }

  let { app, kind }: { app: App; kind: "er" | "gsm" } = $props();
  const app0 = untrack(() => app);
  const kind0 = untrack(() => kind);
  const title = kind0 === "er" ? "New ER diagrams" : "New GSM diagrams";
  const names = Object.keys(app0.settings);
  const data = JSON.parse(defaults_page(kind0, names, names.map((n) => app0.settings[n]))) as {
    toggles: Toggle[];
    groups: { title: string; keys: Key[] }[];
  };
  const toggles = $state(Object.fromEntries(data.toggles.map((t) => [t.key, t.value])));
  const fields = $state(Object.fromEntries(data.groups.flatMap((g) => g.keys.map((k) => [k.key, k.value]))));
  const keys = data.groups.flatMap((g) => g.keys);
  const value = (k: Key) => (k.type === "text" ? fields[k.key] : fields[k.key].trim());

  export function problem(): string | null {
    for (const k of keys) {
      const wrong = check_default(k.key, fields[k.key]);
      if (wrong) return `${title}: ${wrong}`;
    }
    return null;
  }

  function values(): [string, string][] {
    return [...data.toggles.map((t): [string, string] => [t.key, toggles[t.key]]), ...keys.map((k): [string, string] => [k.key, value(k)])];
  }

  export async function apply(): Promise<void> {
    const defaults = Object.fromEntries([
      ...data.toggles.map((t) => [t.key, t.choices[0]?.[0] ?? ""]),
      ...keys.map((k) => [k.key, k.default]),
    ]);
    const settingOf = Object.fromEntries([...data.toggles, ...keys].map((x) => [x.key, x.setting]));
    for (const [k, v] of values()) {
      await app.setSetting(settingOf[k], !v.trim() || v === defaults[k] ? null : v);
    }
  }

  async function useForOpen() {
    const wrong = problem();
    if (wrong) {
      await app.alert(wrong);
      return;
    }
    const pairs = values();
    app.edit((d) =>
      d.use_defaults(
        pairs.map((p) => p[0]),
        pairs.map((p) => p[1]),
      ),
    );
  }

  const swatch = (k: Key) => color_rgb(fields[k.key] || k.default, false);
</script>

<div data-testid="defaults-{kind}">
  <p>New diagrams start with these values, and existing ones keep what they have. An empty field means the packages' default, shown in grey.</p>
  <div class="form">
    <h4 class="span">Main choices</h4>
    {#each data.toggles as t}
      <label for="t-{kind}-{t.key}">{t.title}</label>
      <select id="t-{kind}-{t.key}" bind:value={toggles[t.key]} data-key={t.key}>
        {#each t.choices as [v, text]}<option value={v}>{text}</option>{/each}
      </select>
    {/each}
    {#each data.groups as g}
      <h4 class="span">{g.title}</h4>
      {#each g.keys as k}
        <label for="k-{kind}-{k.key}">{k.key}</label>
        <span class="key">
          {#if k.type === "color"}
            <span class="chip" style:background={swatch(k) || "transparent"}></span>
          {/if}
          <input id="k-{kind}-{k.key}" bind:value={fields[k.key]} placeholder={k.default || "(empty)"} data-key={k.key} spellcheck="false" />
        </span>
      {/each}
    {/each}
  </div>
  <p style="margin-top: 10px">
    <button
      disabled={app.current.kind !== kind}
      title="Copies these values into the open diagram, as one step that can be undone. It keeps its elements and everything else."
      onclick={useForOpen}
      data-testid="use-{kind}">Use for the open diagram</button
    >
  </p>
</div>

<style>
  .key {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .key input {
    flex: 1;
    min-width: 0;
  }
  .chip {
    width: 14px;
    height: 14px;
    border: 1px solid #888;
    flex: none;
  }
</style>
