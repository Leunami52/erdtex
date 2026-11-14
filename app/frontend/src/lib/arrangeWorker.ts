// Arranging (lib/arrange.ts) in a worker: WebCola's candidates are computed off the window's thread, so
// that it stays responsive.
import { candidates, type Graph, type Options } from "./arrange";

self.onmessage = (e: MessageEvent<{ graph: Graph; options: Options }>) => {
  try {
    postMessage({ ok: true, candidates: candidates(e.data.graph, e.data.options) });
  } catch (err) {
    postMessage({ ok: false, error: String(err) });
  }
};
