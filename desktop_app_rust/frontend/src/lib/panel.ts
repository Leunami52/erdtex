// The properties panel's data, as the core describes it (crates/core/src/panel.rs, written
// as JSON by crates/core-wasm/src/json.rs), and what its text fields share.

export interface Item {
  value: string;
  text: string;
  tip: string;
}

export interface Swatch {
  value: string;
  name: string;
  /** #rrggbb, or null for none */
  rgb: string | null;
  tip: string;
}

export interface AttrRow {
  depth: number;
  name: string;
  key: boolean;
  unique: boolean;
  type: string;
  guess: string;
  composite: boolean;
  kind: "normal" | "multi-valued" | "derived";
  formula: string;
}

export interface CardField {
  notation: "n" | "minmax";
  choices: Item[];
  current: string | null;
  lo: string;
  hi: string;
  note: string | null;
  sentence: string;
  deviation: [string, string] | null;
}

export type Field =
  | { type: "text"; text: string; button: string | null; tip: string }
  | { type: "problem"; severity: "error" | "warning" | "hint"; message: string; fix: string | null }
  | { type: "check"; text: string; value: boolean; enabled: boolean; tip: string }
  | { type: "choice"; items: Item[]; current: string; editable: boolean; tip: string }
  | { type: "line"; value: string; placeholder: string; tip: string }
  | { type: "label"; value: string; tip: string }
  | { type: "number"; value: number; min: number; max: number; step: number; decimals: number; suffix: string }
  | { type: "position"; x: number; y: number; step: number }
  | { type: "button"; text: string; tip: string }
  | { type: "color"; swatches: Swatch[]; current: string; tinted: boolean }
  | { type: "datatype"; value: string; guess: string }
  | ({ type: "card" } & CardField)
  | { type: "attributes"; rows: AttrRow[] }
  | { type: "lineage"; rows: LineageRow[] }
  | { type: "code"; text: string };

/** A row of the list of all attributes of an entity or object type (`panel::LineageRow`). */
export interface LineageRow {
  depth: number;
  name: string;
  key: boolean;
  weakKey: boolean;
  unique: boolean;
  kind: "normal" | "multi-valued" | "derived";
  type: string;
  /** inherited: the element it comes from; empty: its own */
  from: string;
  /** where it comes from, every chain (the tooltip) */
  trace: string;
  /** where to edit it: `12` or `12:3` (an attribute list's row) */
  target: string;
}

export type Row = { id: string; label: string } & Field;

export interface Form {
  title: string;
  focus: string | null;
  rows: Row[];
}

export const SEVERITY_COLORS = { error: "rgb(215,35,35)", warning: "rgb(232,135,0)", hint: "rgb(45,125,220)" };

/**
 * A text field commits when it loses the focus or Enter is pressed, and only if its text
 * changed: `data-orig` holds the text it was given.  Ctrl+Z goes to the field while it holds
 * typing of its own, and to the diagram otherwise (see App.svelte).
 */
export function changed(el: HTMLInputElement | HTMLTextAreaElement): boolean {
  if (el.value === el.dataset.orig) return false;
  el.dataset.orig = el.value;
  return true;
}

/** Whether a field holds typing that hasn't been committed. */
export function dirty(el: Element | null): boolean {
  return (
    (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) &&
    el.dataset.orig !== undefined &&
    !el.readOnly &&
    el.value !== el.dataset.orig
  );
}

/** Takes back what was typed into a field since it was last committed. */
export function revert(el: HTMLInputElement | HTMLTextAreaElement): void {
  el.value = el.dataset.orig ?? "";
  el.dispatchEvent(new Event("input", { bubbles: true }));
  el.select();
}

/** Svelte action: a click into the field selects its text (typing replaces it), as a table's
 * editor does; the mouse button's release would otherwise put the cursor at the click. */
export function selectOnFocus(el: HTMLInputElement) {
  let fresh = false;
  const focus = () => {
    el.select();
    fresh = true;
  };
  const up = (e: MouseEvent) => {
    if (fresh) e.preventDefault();
    fresh = false;
  };
  el.addEventListener("focus", focus);
  el.addEventListener("mouseup", up);
  return {
    destroy() {
      el.removeEventListener("focus", focus);
      el.removeEventListener("mouseup", up);
    },
  };
}

/** Svelte action: keeps `data-orig` (and the shown text, when not being typed in) in step with
 * the value the panel was last given. */
export function orig(el: HTMLInputElement | HTMLTextAreaElement, value: string) {
  const set = (v: string) => {
    if (document.activeElement !== el || el.value === el.dataset.orig) el.value = v;
    el.dataset.orig = v;
  };
  set(value);
  return { update: set };
}
