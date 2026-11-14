// The sample diagrams (samples/): File → Open Example. Loaded only when opened.
import type { Kind } from "./backend";
import er from "../../../samples/festival.erd?raw";
import gsm from "../../../samples/festival.gsmd?raw";

/** The sample of each kind: its text and the file name it suggests for Save As. */
export const EXAMPLES: Record<Kind, { text: string; name: string }> = {
  er: { text: er, name: "festival.erd" },
  gsm: { text: gsm, name: "festival.gsmd" },
};
