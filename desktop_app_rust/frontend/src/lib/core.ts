// The core, compiled to WebAssembly, and the Latin Modern fonts it measures with: the page
// registers the same font files (one copy, from the core), so the canvas shows what LaTeX sets.
import init, { font_data, font_names } from "../wasm/core.js";

export async function startCore(): Promise<void> {
  await init();
  await Promise.all(
    font_names().map((name) => {
      // a copy in a plain ArrayBuffer, as FontFace wants it
      const face = new FontFace(name, new Uint8Array(font_data(name) ?? []));
      document.fonts.add(face);
      return face.load();
    }),
  );
}

export * from "../wasm/core.js";
