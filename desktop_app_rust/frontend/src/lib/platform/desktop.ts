// The desktop platform: what the page asks of the Rust side (files, dialogs, settings, pdflatex) and
// of the window (title, theme, closing, dropped files). The browser's is web.ts; backend.ts chooses.
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager";
import { open, save } from "@tauri-apps/plugin-dialog";
import { answered, type ExportJob, type Kind, type Opened } from "./common";

const KIND_FILTERS: Record<Kind, { name: string; extensions: string[] }> = {
  er: { name: "ER diagrams", extensions: ["erd"] },
  gsm: { name: "GSM diagrams", extensions: ["gsmd"] },
};
const ALL_FILES = { name: "All files", extensions: ["*"] };
/** Open: every kind of diagram. */
const OPEN_FILTERS = [{ name: "Diagram files", extensions: ["erd", "gsmd"] }, KIND_FILTERS.er, KIND_FILTERS.gsm, ALL_FILES];

export const readDiagram = (path: string) => invoke<Opened>("read_diagram", { path });
export const writeDiagram = (path: string, text: string) => invoke<string>("write_diagram", { path, text });
export const writeTextFile = (path: string, text: string) => invoke<string>("write_text", { path, text });
export const latexMissing = () => invoke<string>("latex_missing");
/** The diagram compiled by pdflatex; throws its errors (one per line). */
export const latexCompile = (text: string, kind: Kind, preamble: string, language: string, grid: boolean) =>
  invoke<ArrayBuffer>("latex_compile", { text, kind, preamble, language, grid });
/** The editor rendering as SVG (the core's), PDF (svg2pdf) or PNG (resvg). */
export const exportEditor = (job: ExportJob) => invoke<void>("export_editor", { ...job });
/** The LaTeX result as PDF, SVG or PNG (through pdftocairo). */
export const exportLatex = (job: ExportJob) => invoke<void>("export_latex", { ...job });
/** Bytes to a file (sent as they are, not as JSON). */
export const writeBytes = (path: string, bytes: Uint8Array) =>
  invoke<void>("write_bytes", bytes, { headers: { path: encodeURIComponent(path) } });
export const fileStamp = (path: string) => invoke<[number, number] | null>("file_stamp", { path });
export const diagramPaths = (paths: string[]) => invoke<string[]>("diagram_paths", { paths });
export const desktopPalette = () => invoke<Record<string, string> | null>("desktop_palette");
export const selfTestMode = () => invoke<boolean>("self_test_mode");
export const selfTestLine = (line: string) => invoke<void>("self_test_line", { line });
export const selfTestReport = (ok: boolean) => invoke<void>("self_test_report", { ok });
export const fileKind = (path: string) => invoke<Kind | null>("file_kind", { path });
export const absolutePath = (path: string) => invoke<string>("absolute_path", { path });
export const fileExists = (path: string) => invoke<boolean>("file_exists", { path });
export const startupPaths = () => invoke<string[]>("startup_paths");
export const getSettings = () => invoke<Record<string, string>>("get_settings");
export const setSetting = (key: string, value: string | null) => invoke<void>("set_setting", { key, value });
export const sinceStart = () => invoke<number>("since_start");
export const ready = (marks: string) => invoke<number>("ready", { marks });

/** Open: one file or several (each opens in a tab of its own). */
export async function openDialog(start: string): Promise<string[]> {
  const options = { title: "Open Diagram", multiple: true, directory: false, defaultPath: start || undefined, filters: OPEN_FILTERS };
  const test = answered(options);
  const r = test ? test.answer : await open(options);
  return r === null ? [] : typeof r === "string" ? [r] : r;
}

/** Save As: only the diagram's own kind (and all files), so that a dialog that adds the selected
 * filter's extension by itself (KDE's) can't give a GSM diagram `.erd`. */
export async function saveDialog(start: string, kind: Kind): Promise<string | null> {
  const options = { title: "Save Diagram", defaultPath: start || undefined, filters: [KIND_FILTERS[kind], ALL_FILES] };
  const test = answered(options);
  return test ? (test.answer as string | null) : await save(options);
}

/** A folder to save files into. */
export async function folderDialog(title: string, start: string): Promise<string | null> {
  const options = { title, multiple: false, directory: true, defaultPath: start || undefined };
  const test = answered(options);
  const r = test ? test.answer : await open(options);
  return typeof r === "string" ? r : null;
}

/** A save dialog for other files: `name` and `ext` (".sql") make its filter. */
export async function saveFileDialog(title: string, start: string, name: string, ext: string, answer: string | null = null): Promise<string | null> {
  if (answer !== null) return answer;
  const options = { title, defaultPath: start || undefined, filters: [{ name, extensions: [ext.replace(/^\./, "")] }] };
  const test = answered(options);
  return test ? (test.answer as string | null) : await save(options);
}

// The clipboard: the system's, or one inside the app (tests).
let privateClip: string | null = null;
let isPrivate: Promise<boolean> | null = null;
const usePrivate = () => (isPrivate ??= invoke<boolean>("private_clipboard"));

export async function copyText(text: string): Promise<void> {
  if (await usePrivate()) privateClip = text;
  else await writeText(text);
}

export async function pasteText(): Promise<string> {
  if (await usePrivate()) return privateClip ?? "";
  try {
    return await readText();
  } catch {
    return "";
  }
}

// -- the platform and the window ---------------------------------------------------------

/** Which platform this is. */
export const IS_WEB: boolean = false;

/** The program's version. */
export const appVersion = () => getVersion();

/** Whether the system's colours are dark now, and a call whenever that changes. */
export async function watchTheme(changed: (dark: boolean) => void): Promise<boolean> {
  const win = getCurrentWindow();
  await win.onThemeChanged(({ payload }) => changed(payload === "dark"));
  return (await win.theme()) === "dark";
}

/** The window's title. */
export const setTitle = (title: string) => void getCurrentWindow().setTitle(title);

/** Closing the window asks `confirm` first (it saves or asks about unsaved diagrams); `dirty` is for
 * platforms that can only ask synchronously. */
export async function onClose(confirm: () => Promise<boolean>, dirty: () => boolean): Promise<void> {
  void dirty;
  const win = getCurrentWindow();
  await win.onCloseRequested(async (event) => {
    event.preventDefault();
    if (await confirm()) await win.destroy();
  });
}

/** Files dropped on the window: their paths. */
export async function onDrop(open: (paths: string[]) => Promise<void>): Promise<void> {
  await getCurrentWindow().onDragDropEvent(async ({ payload }) => {
    if (payload.type === "drop") await open(payload.paths);
  });
}

/** Quit: closing the window (which asks about unsaved diagrams). */
export const quit = () => getCurrentWindow().close();

/** A web address, in the system's browser (only the repository's and the website's are allowed). */
export async function openUrl(url: string): Promise<void> {
  const { openUrl: open } = await import("@tauri-apps/plugin-opener");
  await open(url);
}

/** The website's offline cache; the desktop app has the program installed. */
export async function startOffline(askReload: () => Promise<boolean>, ready: () => void): Promise<void> {
  void askReload;
  void ready;
}

/** The website's note for phones and small windows; not for the desktop app. */
export function smallScreen(): boolean {
  return false;
}
