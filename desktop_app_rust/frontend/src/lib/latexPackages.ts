// The LaTeX packages of this version (erdiagram.sty, gsmdiagram.sty) and their manuals, built into
// the program: Help → Save LaTeX Packages… saves them into a folder, so that a document can use the
// diagrams the program writes. Loaded only when needed (the manuals are about 1 MB).
import * as be from "./backend";
import type { App } from "./app.svelte";

import erdiagram from "../../../../latex_packages/erdiagram.sty?raw";
import gsmdiagram from "../../../../latex_packages/gsmdiagram.sty?raw";

/** A package's version, from its `\ProvidesPackage{…}[date vX.Y.Z …]` line. */
export function packageVersion(text: string): string {
  return /\\ProvidesPackage\{[^}]*\}\[[^\]]*?\bv(\d+(?:\.\d+)*)/.exec(text)?.[1] ?? "";
}

/** The built-in packages' versions, for About. */
export const PACKAGES = [
  { name: "erdiagram", version: packageVersion(erdiagram) },
  { name: "gsmdiagram", version: packageVersion(gsmdiagram) },
];

interface PackageFile {
  name: string;
  text?: string;
  bytes?: Uint8Array;
}

/** The files saved: both packages and both manuals. */
async function files(): Promise<PackageFile[]> {
  const [er, gsm] = await Promise.all([
    import("../../../../latex_packages/manuals/erdiagram-manual.pdf?url"),
    import("../../../../latex_packages/manuals/gsmdiagram-manual.pdf?url"),
  ]);
  const bytes = async (url: string) => new Uint8Array(await (await fetch(url)).arrayBuffer());
  return [
    { name: "erdiagram.sty", text: erdiagram },
    { name: "gsmdiagram.sty", text: gsmdiagram },
    { name: "erdiagram-manual.pdf", bytes: await bytes(er.default) },
    { name: "gsmdiagram-manual.pdf", bytes: await bytes(gsm.default) },
  ];
}

/** Help → Save LaTeX Packages…: into a folder the user chooses; files already there are replaced
 * only when the user agrees. */
export async function savePackages(app: App): Promise<void> {
  if (be.IS_WEB) return saveZip(app);
  const t = app.current;
  const start = app.settings["packages/dir"] ?? (t.path ? be.dirName(t.path) : "");
  const dir = await be.folderDialog("Save LaTeX Packages Into", start);
  if (!dir) return;
  const all = await files();
  const sep = dir.includes("\\") && !dir.includes("/") ? "\\" : "/";
  const path = (name: string) => `${dir.replace(/[\\/]+$/, "")}${sep}${name}`;
  const present: string[] = [];
  for (const f of all) if (await be.fileExists(path(f.name))) present.push(f.name);
  if (present.length) {
    const list = present.join(", ");
    const answer = await app.ask(`${list} ${present.length > 1 ? "are" : "is"} already in that folder. Replace ${present.length > 1 ? "them" : "it"}?`, ["Replace", "Cancel"], "Save LaTeX Packages");
    if (answer !== "Replace") return;
  }
  try {
    for (const f of all) {
      if (f.text !== undefined) await be.writeTextFile(path(f.name), f.text);
      else if (f.bytes) await be.writeBytes(path(f.name), f.bytes);
    }
    await app.setSetting("packages/dir", dir);
    app.message(`Saved erdiagram.sty, gsmdiagram.sty and their manuals in ${dir}`, 6000);
  } catch (e) {
    await app.alert(String(e));
  }
}

/** In the browser, which can't write into a folder: one zip with all four files. */
async function saveZip(app: App): Promise<void> {
  const path = await be.saveFileDialog("Save LaTeX Packages", "latex-packages.zip", "ZIP archive", ".zip");
  if (!path) return;
  const { zipSync, strToU8 } = await import("fflate");
  const entries: Record<string, Uint8Array> = {};
  for (const f of await files()) entries[f.name] = f.bytes ?? strToU8(f.text ?? "");
  try {
    await be.writeBytes(path, zipSync(entries));
    app.message("Saved erdiagram.sty, gsmdiagram.sty and their manuals as latex-packages.zip", 6000);
  } catch (e) {
    await app.alert(String(e));
  }
}
