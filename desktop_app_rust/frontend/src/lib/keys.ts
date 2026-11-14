// Keyboard shortcuts: what a key press is, in Qt's portable form (Meta+Ctrl+Alt+Shift+Key),
// and which command it belongs to.  The table and the reading of key texts come from the core.
import { canonical_keys, shortcut_table } from "../wasm/core.js";
import { IS_WEB } from "./backend";

/** The website's defaults where a browser keeps the desktop's key for itself (new window, close tab,
 * switch tabs, quit, inspect): changeable in Preferences as any other. Quit and Reload have none. */
export const WEB_KEYS: Record<string, string> = {
  a_new_er: "Alt+N",
  a_new_gsm: "Alt+Shift+N",
  a_close_tab: "Alt+W",
  a_next_tab: "Alt+PgDown",
  a_prev_tab: "Alt+PgUp",
  a_copy_input: "Alt+Shift+C",
  a_quit: "",
  a_reload: "",
};

export interface Command {
  id: string;
  title: string;
  group: string;
  keys: string;
}

export function commands(): Command[] {
  const t = shortcut_table();
  const out: Command[] = [];
  for (let i = 0; i + 3 < t.length; i += 4) {
    const keys = IS_WEB && t[i] in WEB_KEYS ? WEB_KEYS[t[i]] : t[i + 3];
    out.push({ id: t[i], title: t[i + 1], group: t[i + 2], keys });
  }
  return out;
}

const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);

const NAMED: Record<string, string> = {
  " ": "Space", Escape: "Esc", Delete: "Del", Backspace: "Backspace", Enter: "Return", Tab: "Tab",
  ArrowLeft: "Left", ArrowRight: "Right", ArrowUp: "Up", ArrowDown: "Down", PageUp: "PgUp",
  PageDown: "PgDown", Insert: "Ins", Home: "Home", End: "End", Pause: "Pause", ContextMenu: "Menu",
};

function keyName(e: KeyboardEvent): string | null {
  if (e.code === "NumpadEnter") return "Enter";
  if (NAMED[e.key]) return NAMED[e.key];
  if (/^F\d{1,2}$/.test(e.key)) return e.key;
  if ([...e.key].length === 1) return e.key.toUpperCase();
  return null;
}

/** The physical key, for layouts where Shift+digit isn't a digit (! " § $ … on German keys). */
function physical(e: KeyboardEvent): string | null {
  const m = /^(?:Key([A-Z])|Digit(\d)|Numpad(\d))$/.exec(e.code);
  return m ? (m[1] ?? m[2] ?? m[3]) : null;
}

/** The combinations a key press can stand for, the most specific first.  `loose`: as if Shift
 * and Alt weren't held (nudging takes them as the step). */
export function combos(e: KeyboardEvent, loose = false): string[] {
  if (["Control", "Shift", "Alt", "Meta", "AltGraph"].includes(e.key)) return [];
  // Qt on macOS: "Ctrl" is Cmd and "Meta" is Control
  const ctrl = isMac ? e.metaKey : e.ctrlKey;
  const meta = isMac ? e.ctrlKey : e.metaKey;
  const alt = e.altKey && !loose;
  const mods = (shift: boolean) =>
    (meta ? "Meta+" : "") + (ctrl ? "Ctrl+" : "") + (alt ? "Alt+" : "") + (shift && !loose ? "Shift+" : "");
  const out: string[] = [];
  const key = keyName(e);
  if (key) {
    out.push(mods(e.shiftKey) + key);
    // a character that needs Shift: Ctrl++ is also pressed as Ctrl+Shift+=
    if (e.shiftKey && key.length === 1 && !/[A-Z0-9]/.test(key)) out.push(mods(false) + key);
  }
  const phys = physical(e);
  if (phys && phys !== key) out.push(mods(e.shiftKey) + phys);
  return out;
}

/** The settings' name of a command's keys, where changed keys are kept. */
export const settingOf = (id: string) => `shortcuts/${id}`;

/** Bindings: combinations -> the commands they stand for.  Tools have a set per kind of
 * diagram, so the same keys may stand for an ER and a GSM tool. */
export class Bindings {
  private byCombo = new Map<string, string[]>();
  readonly keys = new Map<string, string>();

  constructor(table: Command[], changed: Record<string, string> = {}) {
    for (const c of table) {
      const keys = changed[settingOf(c.id)] ?? c.keys;
      this.keys.set(c.id, keys);
      for (const part of keys.split(";")) {
        const k = canonical_keys(part.trim());
        if (!k) continue;
        const ids = this.byCombo.get(k) ?? [];
        ids.push(c.id);
        this.byCombo.set(k, ids);
      }
    }
  }

  /** Whether a key press is one of the combinations of a command. */
  matches(id: string, e: KeyboardEvent): boolean {
    return combos(e).some((c) => this.byCombo.get(c)?.includes(id));
  }

  /** The command for a key press, among those that apply to a diagram of `kind`. */
  commandFor(e: KeyboardEvent, kind: string): string | null {
    const applies = (id: string) => !/^(er|gsm)\./.test(id) || id.startsWith(kind + ".");
    for (const c of combos(e)) {
      const id = this.byCombo.get(c)?.find(applies);
      if (id) return id;
    }
    // nudging with Shift (1 cm) or Alt (0.05 cm)
    if (e.shiftKey || e.altKey) {
      for (const c of combos(e, true)) {
        const id = this.byCombo.get(c)?.find((i) => i.startsWith("nudge"));
        if (id) return id;
      }
    }
    return null;
  }
}

/** As the platform writes a combination (Cmd on macOS). */
export function display(keys: string): string {
  const first = keys.split(";")[0]?.trim() ?? "";
  return isMac ? first.replace(/Ctrl\+/g, "⌘").replace(/Meta\+/g, "⌃") : first;
}
