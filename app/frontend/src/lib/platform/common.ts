// What both platforms share: the types, the window tests' answers to dialogs, path helpers.

export type Kind = "er" | "gsm";

export interface Opened {
  path: string;
  text: string;
  kind: Kind;
}

export const EXTENSIONS: Record<Kind, string> = { er: ".erd", gsm: ".gsmd" };

/** The window tests answer dialogs instead of a person (`window.__erdtex.answerDialogs`), and see what
 * each answered dialog was asked with. */
export const dialogTest: { answers: (string | string[] | null)[]; asked: Record<string, unknown>[] } = { answers: [], asked: [] };

export function answered(asked: Record<string, unknown>): { answer: string | string[] | null } | null {
  if (!dialogTest.answers.length) return null;
  dialogTest.asked.push(asked);
  return { answer: dialogTest.answers.shift() ?? null };
}

export interface ExportJob {
  text: string;
  kind: Kind;
  language: string;
  preamble: string;
  format: string;
  grid: boolean;
  dpi: number;
  transparent: boolean;
  path: string;
}
/** What exportLatex says when the page has to draw the LaTeX result as PNG itself. */
export const NEEDS_PAGE = "pdftocairo was not found";

/** The file name of a path. */
export function baseName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** The folder of a path. */
export function dirName(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i > 0 ? path.slice(0, i) : "";
}

/** Whether a path's file name has an extension. */
export function hasSuffix(path: string): boolean {
  return /\.[^.\\/]+$/.test(baseName(path));
}

/** Why the website has no LaTeX preview and no LaTeX result. */
export const LATEX_IN_BROWSER =
  "The LaTeX preview needs pdflatex, which a browser doesn't have. It works in the desktop version (Help → Download the Desktop Version).";
