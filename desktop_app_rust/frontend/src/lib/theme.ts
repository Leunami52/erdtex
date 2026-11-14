// Display modes: light, dark, intermediate (dark panels, light editing window), or as the
// system says.  The panels follow a palette; on the dark canvas, the core recolours the diagram
// while drawing it (never in files, LaTeX or exports).  The values are the Python app's.

export const MODES = {
  system: "System",
  light: "Light",
  dark: "Dark",
  intermediate: "Intermediate: dark panels, light editing window",
} as const;
export type Mode = keyof typeof MODES;

const DARK_PANELS = {
  window: "#2b2b2b", text: "#e6e6e6", base: "#1f1f1f", alternate: "#262626", button: "#3a3a3a",
  highlight: "#2f7fd6", link: "#6aaeff", placeholder: "#8c8c8c", disabled: "#7a7a7a",
  light: "#4a4a4a", mid: "#333333", dark: "#191919",
};
const LIGHT_PANELS = {
  window: "#efefef", text: "#1a1a1a", base: "#ffffff", alternate: "#f6f6f6", button: "#e6e6e6",
  highlight: "#2f7fd6", link: "#1a5fb4", placeholder: "#7f7f7f", disabled: "#9a9a9a",
  light: "#ffffff", mid: "#c8c8c8", dark: "#a0a0a0",
};

/** The canvas: its background, the grid's lines, the axes and the ruler numbers. */
export interface CanvasColors {
  background: string;
  minor: string;
  major: string;
  axes: string;
  ruler: string;
}

const DARK_CANVAS: CanvasColors = {
  background: "rgb(28,28,28)", minor: "rgb(36,36,40)", major: "rgb(50,50,56)", axes: "rgb(84,84,96)",
  ruler: "rgb(135,135,150)",
};
const LIGHT_CANVAS: CanvasColors = {
  background: "rgb(255,255,255)", minor: "rgb(244,244,248)", major: "rgb(228,228,236)",
  axes: "rgb(200,200,215)", ruler: "rgb(150,150,170)",
};

export function asMode(value: string | undefined): Mode {
  return value && value in MODES ? (value as Mode) : "system";
}

/** (panels dark, canvas dark) for a mode, given what the system uses. */
export function resolve(mode: Mode, systemDark: boolean): [boolean, boolean] {
  switch (mode) {
    case "light":
      return [false, false];
    case "dark":
      return [true, true];
    case "intermediate":
      return [true, false];
    default:
      return [systemDark, systemDark];
  }
}

export function canvasColors(dark: boolean): CanvasColors {
  return dark ? DARK_CANVAS : LIGHT_CANVAS;
}

/** Whether a palette's window colour is dark (as Qt's lightness < 128). */
export function isDarkPalette(p: Record<string, string>): boolean {
  const m = /^#(..)(..)(..)$/.exec(p.window ?? "");
  if (!m) return false;
  const [r, g, b] = m.slice(1).map((h) => parseInt(h, 16));
  return (Math.max(r, g, b) + Math.min(r, g, b)) / 2 < 128;
}

/** The palette of the panels as CSS variables on the page: the desktop's own (a KDE colour
 * scheme, in "System" mode, as the Python app), or the app's light or dark one. */
export function applyPanels(dark: boolean, desktop: Record<string, string> | null = null): void {
  const p = desktop ?? (dark ? DARK_PANELS : LIGHT_PANELS);
  const root = document.documentElement.style;
  for (const [k, v] of Object.entries(p)) root.setProperty(`--${k}`, v);
  root.setProperty("color-scheme", dark ? "dark" : "light");
}
