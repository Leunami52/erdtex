// The check harness for arranging (ROADMAP_ARRANGE.md): the page's half. Runs the app's own layout
// (frontend/src/lib/arrange.ts) on a graph from `arrange_check graph` and prints its candidates.
//
//     node --experimental-strip-types spike/arrange_check.mjs graph.json [anew] > candidates.json
import { readFileSync } from "node:fs";
import { candidates } from "../frontend/src/lib/arrange.ts";

const [graphPath, mode] = process.argv.slice(2);
const graph = JSON.parse(readFileSync(graphPath, "utf8"));
const t0 = performance.now();
const out = candidates(graph, { spacing: Number(process.env.SPACING ?? 0.5), anew: mode === "anew" });
process.stderr.write(`${out.length} candidates in ${Math.round(performance.now() - t0)} ms\n`);
console.log(JSON.stringify(out));
