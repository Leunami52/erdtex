// PDFs from pdflatex, drawn with pdf.js: the LaTeX preview, and the LaTeX result as PNG where
// pdftocairo is missing.  pdf.js is loaded the first time it is needed, so that it doesn't slow
// down the start.
type PdfJs = typeof import("pdfjs-dist");
let lib: Promise<PdfJs> | null = null;

function pdfjs(): Promise<PdfJs> {
  lib ??= (async () => {
    const [m, worker] = await Promise.all([import("pdfjs-dist"), import("pdfjs-dist/build/pdf.worker.min.mjs?url")]);
    m.GlobalWorkerOptions.workerSrc = worker.default;
    return m;
  })();
  return lib;
}

/** The first page drawn at `pxPerPt` device pixels per PDF point, white behind it unless
 * `transparent`; at most `maxPixels` on its longer side. */
export async function renderPage(data: ArrayBuffer, pxPerPt: number, transparent = false, maxPixels = 8000): Promise<HTMLCanvasElement> {
  const m = await pdfjs();
  const task = m.getDocument({ data: new Uint8Array(data) });
  const doc = await task.promise;
  try {
    const page = await doc.getPage(1);
    const unit = page.getViewport({ scale: 1 });
    const scale = Math.min(pxPerPt, maxPixels / Math.max(unit.width, unit.height, 1));
    const viewport = page.getViewport({ scale });
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.ceil(viewport.width));
    canvas.height = Math.max(1, Math.ceil(viewport.height));
    await page.render({ canvas, viewport, background: transparent ? "rgba(0,0,0,0)" : "#ffffff" }).promise;
    return canvas;
  } finally {
    await task.destroy();
  }
}

/** A canvas as PNG bytes. */
export function pngBytes(canvas: HTMLCanvasElement): Promise<Uint8Array> {
  return new Promise((resolve, reject) =>
    canvas.toBlob(async (b) => (b ? resolve(new Uint8Array(await b.arrayBuffer())) : reject(new Error("cannot make a PNG"))), "image/png"),
  );
}
