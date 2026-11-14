// The view of a canvas: scale and offset, as the Python app's QGraphicsView.  A point (x, y)
// of the diagram, in TeX points with y up, is at screen pixel (x·k + tx, −y·k + ty).

/** CSS pixels per TeX point at 100 %: 96 px per inch, 72.27 pt per inch. */
export const PX_PER_PT = 96 / 72.27;
/** One centimetre in TeX points. */
export const CM = 72.27 / 2.54;

/** Grid snapping offered (toolbar, Preferences): text, cm. */
export const SNAPS: [string, number][] = [["off", 0], ["0.1 cm", 0.1], ["0.25 cm", 0.25], ["0.5 cm", 0.5], ["1 cm", 1]];
export const MIN_ZOOM = 0.05;
export const MAX_ZOOM = 20;

export interface View {
  /** pixels per point */
  k: number;
  tx: number;
  ty: number;
}

export type Box = [number, number, number, number];

export function zoomOf(v: View): number {
  return v.k / PX_PER_PT;
}

export function clampZoom(zoom: number): number {
  return Math.max(MIN_ZOOM, Math.min(zoom, MAX_ZOOM));
}

export function toDiagram(v: View, sx: number, sy: number): [number, number] {
  return [(sx - v.tx) / v.k, (v.ty - sy) / v.k];
}

export function toScreen(v: View, x: number, y: number): [number, number] {
  return [x * v.k + v.tx, -y * v.k + v.ty];
}

/** The zoom set to `zoom`, keeping the diagram point under (sx, sy) where it is. */
export function zoomTo(v: View, zoom: number, sx: number, sy: number): View {
  const [x, y] = toDiagram(v, sx, sy);
  const k = clampZoom(zoom) * PX_PER_PT;
  return { k, tx: sx - x * k, ty: sy + y * k };
}

/** Diagram point (x, y) in the middle of a view of size w × h. */
export function centreOn(v: View, x: number, y: number, w: number, h: number): View {
  return { k: v.k, tx: w / 2 - x * v.k, ty: h / 2 + y * v.k };
}

/** The whole diagram in view, with 15 pt around it (and the 4 pt the Python app's canvas items
 * reach beyond their drawing); 100 % for an empty one. */
export function fit(bbox: Box | null, w: number, h: number): View {
  if (!bbox || bbox[2] <= bbox[0] || bbox[3] <= bbox[1] || w <= 0 || h <= 0) {
    return centreOn({ k: PX_PER_PT, tx: 0, ty: 0 }, 0, 0, w, h);
  }
  const m = 15 + 4;
  const [x0, y0, x1, y1] = [bbox[0] - m, bbox[1] - m, bbox[2] + m, bbox[3] + m];
  const k = clampZoom(Math.min(w / (x1 - x0), h / (y1 - y0)) / PX_PER_PT) * PX_PER_PT;
  return centreOn({ k, tx: 0, ty: 0 }, (x0 + x1) / 2, (y0 + y1) / 2, w, h);
}

export interface Grid {
  /** major lines, in points */
  step: number;
  /** minor lines in between, in points; 0 for none */
  minor: number;
  /** distance of the numbers on the rulers, in points */
  label: number;
}

/** 1 cm lines; every 5 cm when that's too dense, half centimetres when there's room. */
export function gridSteps(k: number): Grid {
  const px = CM * k;
  const step = px < 12 ? 5 * CM : CM;
  const minor = px > 60 ? CM / 2 : 0;
  const label = px >= 30 ? step : px >= 15 ? 2 * step : 5 * step;
  return { step, minor, label };
}

/** The multiples of `step` from `from` to `to` (diagram coordinates). */
export function multiples(from: number, to: number, step: number): number[] {
  const out: number[] = [];
  if (step <= 0 || !(to > from)) return out;
  for (let i = Math.floor(from / step); i * step < to && out.length < 4000; i++) out.push(i * step);
  return out;
}

/** "(x, y) cm" for the status bar. */
export function position(x: number, y: number): string {
  const f = (v: number) => (v / CM + 0).toFixed(2).replace(/^-0\.00$/, "0.00");
  return `(${f(x)}, ${f(y)}) cm`;
}
