// The browser platform: the same functions as desktop.ts, done with what a web page has.
//
// Files: a page sees no paths, so a file it may read or write gets a made-up one (`web:3/name.erd`,
// see `files`). Opening uses the browser's own file access where it has one (Chrome, Edge), else an
// upload (browser-fs-access does both). Saving writes back into the same file where the browser
// allows it, and is a download elsewhere. The settings live in the browser's local storage; the
// LaTeX preview and the LaTeX result need pdflatex, which a browser doesn't have.
import { fileOpen } from "browser-fs-access";
import { export_svg, kind_of_file } from "../core";
import { answered, baseName, LATEX_IN_BROWSER, type ExportJob, type Kind, type Opened } from "./common";

// -- files ------------------------------------------------------------------------------

interface WebFile {
  name: string;
  /** what was read (an upload), or nothing yet (a new download) */
  file?: File;
  /** the browser's handle, where it can write back */
  handle?: FileSystemFileHandle;
  /** what was last written */
  text?: string;
}

const files = new Map<string, WebFile>();
let counter = 0;

/** A made-up path for a file the page may use. */
function register(f: WebFile): string {
  const path = `web:${++counter}/${f.name}`;
  files.set(path, f);
  return path;
}

/** For the browser tests: a file "on disk" with this text, and its path. */
export function addFile(name: string, text: string): string {
  return register({ name, text });
}

/** For the browser tests: what was written to a path (a download's content too). */
export function writtenText(path: string): string | null {
  return files.get(path)?.text ?? null;
}

const canSave = typeof window !== "undefined" && "showSaveFilePicker" in window;

const DIAGRAM_WORDS = ["\\entity", "\\relationship", "\\isa", "\\canbe", "\\partof", "\\link", "\\attributes",
  "\\abstract", "\\representable", "\\subtype", "\\aggregation", "\\grouping", "\\attribute", "\\multiattribute",
  "\\component", "\\generalization", "\\specialization"];

function extension(name: string): string | undefined {
  const m = /\.[^.]+$/.exec(name);
  return m ? m[0].toLowerCase() : undefined;
}

async function textOf(path: string): Promise<string> {
  const f = files.get(path);
  if (!f) throw new Error(`Cannot open ${baseName(path)}: it is no longer there.`);
  if (f.text !== undefined) return f.text;
  if (f.file) return await f.file.text();
  if (f.handle) return await (await f.handle.getFile()).text();
  throw new Error(`Cannot open ${baseName(path)}.`);
}

export async function readDiagram(path: string): Promise<Opened> {
  const text = (await textOf(path)).replace(/\r\n?/g, "\n");
  const name = baseName(path);
  const ext = extension(name);
  if (ext !== ".erd" && ext !== ".gsmd" && !DIAGRAM_WORDS.some((w) => text.includes(w))) {
    throw new Error(`${name} is not a diagram: it has no .erd or .gsmd extension and no diagram commands inside.`);
  }
  return { path, text, kind: kind_of_file(ext, text) as Kind };
}

/** Bytes into a file: written back where the browser allows it, else downloaded. */
async function write(path: string, data: Uint8Array | string, type: string): Promise<string> {
  const f = files.get(path) ?? { name: baseName(path) };
  files.set(path, f);
  const blob = new Blob([data as BlobPart], { type });
  if (f.handle) {
    const w = await f.handle.createWritable();
    await w.write(blob);
    await w.close();
  } else {
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = f.name;
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(a.href), 60_000);
  }
  f.text = typeof data === "string" ? data : undefined;
  f.file = undefined;
  return path;
}

export const writeDiagram = (path: string, text: string) => write(path, text, "text/plain");
export const writeTextFile = (path: string, text: string) => write(path, text, "text/plain");
export async function writeBytes(path: string, bytes: Uint8Array): Promise<void> {
  await write(path, bytes, "application/octet-stream");
}

export const fileStamp = async (path: string): Promise<[number, number] | null> => (void path, null);
export const diagramPaths = async (paths: string[]) => paths.filter((p) => files.has(p));
export const fileKind = async (path: string): Promise<Kind | null> => {
  try {
    return (await readDiagram(path)).kind;
  } catch {
    return null;
  }
};
export const absolutePath = async (path: string) => path;
export const fileExists = async (path: string) => {
  const f = files.get(path);
  return !!f && (f.text !== undefined || !!f.file || !!f.handle);
};
export const startupPaths = async (): Promise<string[]> => [];

// -- dialogs ------------------------------------------------------------------------------

/** Open: one file or several, through the browser's own picker or an upload. */
export async function openDialog(start: string): Promise<string[]> {
  void start;
  const test = answered({ title: "Open Diagram", multiple: true });
  if (test) return test.answer === null ? [] : typeof test.answer === "string" ? [test.answer] : test.answer;
  try {
    const picked = await fileOpen({ multiple: true, extensions: [".erd", ".gsmd"], description: "Diagram files" });
    return picked.map((file) => register({ name: file.name, file, handle: file.handle }));
  } catch {
    return []; // cancelled
  }
}

/** Where to save: the browser's own picker where there is one, else a download of this name. */
async function saveTarget(name: string, description: string, ext: string): Promise<string | null> {
  if (canSave) {
    try {
      const handle = await (window as unknown as { showSaveFilePicker: (o: object) => Promise<FileSystemFileHandle> }).showSaveFilePicker({
        suggestedName: name,
        types: [{ description, accept: { "application/octet-stream": [ext] } }],
      });
      return register({ name: handle.name, handle });
    } catch {
      return null; // cancelled
    }
  }
  return register({ name });
}

export async function saveDialog(start: string, kind: Kind): Promise<string | null> {
  const ext = kind === "er" ? ".erd" : ".gsmd";
  const test = answered({ title: "Save Diagram", kind });
  if (test) return test.answer as string | null;
  const name = baseName(start || `diagram${ext}`);
  return saveTarget(name, kind === "er" ? "ER diagrams" : "GSM diagrams", ext);
}

export async function saveFileDialog(title: string, start: string, name: string, ext: string, answer: string | null = null): Promise<string | null> {
  if (answer !== null) return answer;
  const test = answered({ title, name, ext });
  if (test) return test.answer as string | null;
  return saveTarget(baseName(start), name, ext);
}

/** A page can't choose a folder to write into; the LaTeX packages come as one zip instead. */
export async function folderDialog(title: string, start: string): Promise<string | null> {
  void title;
  void start;
  return null;
}

// -- settings and the like ------------------------------------------------------------------

const SETTINGS = "erdtex/settings";

function loadSettings(): Record<string, string> {
  try {
    const v = JSON.parse(localStorage.getItem(SETTINGS) ?? "{}") as unknown;
    return v && typeof v === "object" ? (v as Record<string, string>) : {};
  } catch {
    return {};
  }
}

export const getSettings = async () => loadSettings();
export async function setSetting(key: string, value: string | null): Promise<void> {
  const s = loadSettings();
  if (value === null) delete s[key];
  else s[key] = value;
  try {
    localStorage.setItem(SETTINGS, JSON.stringify(s));
  } catch {
    // full or not allowed: the setting lasts until the page is closed
  }
}
export const desktopPalette = async (): Promise<Record<string, string> | null> => null;
export const selfTestMode = async () => false;
export const selfTestLine = async (line: string): Promise<void> => void line;
export const selfTestReport = async (ok: boolean): Promise<void> => void ok;
export const sinceStart = async () => performance.now() / 1000;
export const ready = async (marks: string) => (void marks, 0);

// -- LaTeX and exports ------------------------------------------------------------------------

export const latexMissing = async () => LATEX_IN_BROWSER;
export async function latexCompile(text: string, kind: Kind, preamble: string, language: string, grid: boolean): Promise<ArrayBuffer> {
  void [text, kind, preamble, language, grid];
  throw new Error(LATEX_IN_BROWSER);
}
export async function exportLatex(job: ExportJob): Promise<void> {
  void job;
  throw new Error(LATEX_IN_BROWSER);
}

/** An SVG drawn into a PNG at `dpi`, white or transparent behind it (as the desktop app's resvg). */
async function svgToPng(svg: string, dpi: number, transparent: boolean): Promise<Uint8Array> {
  const img = new Image();
  img.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(svg);
  await img.decode();
  const pt = (attr: string) => Number(new RegExp(`${attr}="([\\d.]+)pt"`).exec(svg)?.[1] ?? 0);
  const scale = dpi / 72;
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.ceil(pt("width") * scale));
  canvas.height = Math.max(1, Math.ceil(pt("height") * scale));
  const g = canvas.getContext("2d");
  if (!g) throw new Error("This browser can't draw a PNG.");
  if (!transparent) {
    g.fillStyle = "#ffffff";
    g.fillRect(0, 0, canvas.width, canvas.height);
  }
  g.drawImage(img, 0, 0, canvas.width, canvas.height);
  const blob = await new Promise<Blob | null>((ok) => canvas.toBlob(ok, "image/png"));
  if (!blob) throw new Error(`${canvas.width} × ${canvas.height} pixels is too large; please choose a lower resolution.`);
  return new Uint8Array(await blob.arrayBuffer());
}

/** The editor rendering as SVG (the core's), PNG (drawn by the page) or PDF (svg2pdf, loaded now). */
export async function exportEditor(job: ExportJob): Promise<void> {
  const svg = export_svg(job.text, job.kind, job.language, job.grid);
  if (job.format === "svg") {
    await write(job.path, svg, "image/svg+xml");
  } else if (job.format === "pdf") {
    const pdf = await import("../../wasm-pdf/svgpdf.js");
    await pdf.default();
    await write(job.path, pdf.svg_to_pdf(svg), "application/pdf");
  } else {
    await write(job.path, await svgToPng(svg, job.dpi, job.transparent), "image/png");
  }
}

// -- the clipboard ------------------------------------------------------------------------------

let ownClip = "";

export async function copyText(text: string): Promise<void> {
  ownClip = text;
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // not allowed here: copying works inside the page
  }
}

export async function pasteText(): Promise<string> {
  try {
    return await navigator.clipboard.readText();
  } catch {
    return ownClip;
  }
}

// -- the platform and the window -------------------------------------------------------------------

/** Which platform this is. */
export const IS_WEB: boolean = true;

/** The program's version. */
export const appVersion = async () => __APP_VERSION__;

/** Whether the system's colours are dark now, and a call whenever that changes. */
export async function watchTheme(changed: (dark: boolean) => void): Promise<boolean> {
  const q = window.matchMedia("(prefers-color-scheme: dark)");
  q.addEventListener("change", (e) => changed(e.matches));
  return q.matches;
}

/** The window's title: the page's. */
export const setTitle = (title: string) => void (document.title = title);

/** Leaving the page warns while a diagram has unsaved changes (a browser asks only synchronously). */
export async function onClose(confirm: () => Promise<boolean>, dirty: () => boolean): Promise<void> {
  void confirm;
  window.addEventListener("beforeunload", (e) => {
    if (dirty()) {
      e.preventDefault();
      e.returnValue = "";
    }
  });
}

/** Files dropped on the page: made-up paths for them. */
export async function onDrop(open: (paths: string[]) => Promise<void>): Promise<void> {
  window.addEventListener("dragover", (e) => e.preventDefault());
  window.addEventListener("drop", (e) => {
    e.preventDefault();
    const dropped = [...(e.dataTransfer?.files ?? [])];
    if (dropped.length) void open(dropped.map((file) => register({ name: file.name, file })));
  });
}

/** A page can't quit; the menu leaves the command out. */
export const quit = async (): Promise<void> => {};

/** A web address, in a new browser tab. */
export async function openUrl(url: string): Promise<void> {
  window.open(url, "_blank", "noopener");
}

/** The service worker (vite-plugin-pwa): the program kept in the browser after the first visit, so
 * that it starts offline and loads almost nothing again; a new version asks before it reloads. */
export async function startOffline(askReload: () => Promise<boolean>, ready: () => void): Promise<void> {
  if (!("serviceWorker" in navigator)) return;
  const { registerSW } = await import("virtual:pwa-register");
  const update = registerSW({
    onNeedRefresh: () =>
      void askReload().then((yes) => {
        if (yes) void update(true);
      }),
    onOfflineReady: ready,
  });
}

/** A phone or tablet, or a very small window: the editor wants a mouse, a keyboard and room. */
export function smallScreen(): boolean {
  const fine = window.matchMedia("(any-pointer: fine)").matches;
  return !fine || window.innerWidth < 900 || window.innerHeight < 560;
}
