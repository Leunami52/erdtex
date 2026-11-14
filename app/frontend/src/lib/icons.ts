// The tools' icons, drawn in the packages' style (as the Python app's palette.py draws them
// with QPainter), 40 × 32.  Strokes drawn straight on the panel take its text colour in dark
// mode; filled shapes keep black outlines on their light fill.

const GRAY = "rgb(235,235,235)";

interface Ink {
  line: string;
  red: string;
  blue: string;
}

const ink = (dark: boolean): Ink =>
  dark ? { line: "#e6e6e6", red: "rgb(255,120,120)", blue: "rgb(130,160,255)" } : { line: "#000", red: "rgb(178,0,0)", blue: "rgb(60,90,220)" };

const pts = (p: number[][]) => p.map(([x, y]) => `${x},${y}`).join(" ");

function arrow(i: Ink, x1: number, y1: number, x2: number, y2: number, heads = 1, w = 1.3, dash = false, double = false): string {
  const ang = Math.atan2(y2 - y1, x2 - x1);
  const L = 6;
  const [c, s] = [Math.cos(ang), Math.sin(ang)];
  const end = [x2 - heads * L * c, y2 - heads * L * s];
  let out = "";
  if (double) {
    // two thin parallel lines, transparent between them
    const [nx, ny] = [-s * 1.6, c * 1.6];
    for (const k of [1, -1]) {
      out += `<line x1="${x1 + k * nx}" y1="${y1 + k * ny}" x2="${end[0] + k * nx}" y2="${end[1] + k * ny}" stroke="${i.line}" stroke-width="1.1"/>`;
    }
  } else {
    out += `<line x1="${x1}" y1="${y1}" x2="${end[0]}" y2="${end[1]}" stroke="${i.line}" stroke-width="${w}"${dash ? ' stroke-dasharray="3.25 2.6"' : ""}/>`;
  }
  for (let k = 0; k < heads; k++) {
    const [tx, ty] = [x2 - k * L * c, y2 - k * L * s];
    const [bx, by] = [tx - L * c, ty - L * s];
    const [nx, ny] = [-s * 3, c * 3];
    out += `<polygon points="${pts([[tx, ty], [bx + nx, by + ny], [bx - nx, by - ny]])}" fill="${i.line}"/>`;
  }
  return out;
}

function diamond(cx: number, cy: number, w: number, h: number, double = false): string {
  let out = `<polygon points="${pts([[cx, cy - h], [cx + w, cy], [cx, cy + h], [cx - w, cy]])}" fill="#fff" stroke="#000" stroke-width="1.3"/>`;
  if (double) {
    const k = 0.72;
    out += `<polygon points="${pts([[cx, cy - h * k], [cx + w * k, cy], [cx, cy + h * k], [cx - w * k, cy]])}" fill="none" stroke="#000" stroke-width="1.3"/>`;
  }
  return out;
}

function hexagon(cx: number, cy: number, w: number, h: number): string {
  const s = h * 0.5;
  return `<polygon points="${pts([[cx + w, cy], [cx + w - s, cy - h], [cx - w + s, cy - h], [cx - w, cy], [cx - w + s, cy + h], [cx + w - s, cy + h]])}" fill="#fff" stroke="#000" stroke-width="1.3"/>`;
}

function label(text: string, color: string, size = 9, italic = false): string {
  return `<text x="20" y="16" dominant-baseline="central" text-anchor="middle" font-family="sans-serif" font-weight="bold" font-size="${size}"${italic ? ' font-style="italic"' : ""} fill="${color}">${text}</text>`;
}

/** The inside of a tool's icon (SVG elements in a 40 × 32 box). */
export function toolIcon(name: string, dark: boolean): string {
  const i = ink(dark);
  switch (name) {
    case "select":
      return `<path d="M14 5 L14 26 L19 21 L23 29 L26 27 L22 20 L28 20 Z" fill="#fff" stroke="#000" stroke-width="1.2"/>`;
    case "entity":
    case "weak entity":
      return (
        `<rect x="4" y="8" width="32" height="16" fill="${GRAY}" stroke="#000" stroke-width="1.3"/>` +
        (name === "weak entity" ? `<rect x="7" y="11" width="26" height="10" fill="none" stroke="#000" stroke-width="1.3"/>` : "")
      );
    case "relationship":
    case "weak relationship":
    case "existence dependency":
    case "identification dependency":
    case "part of":
      return (
        diamond(20, 16, 15, 11, ["weak relationship", "existence dependency", "identification dependency"].includes(name)) +
        (({ "existence dependency": label("E", "rgb(178,0,0)"), "identification dependency": label("Id", "rgb(178,0,0)"), "part of": label("p", "#000") } as Record<string, string>)[name] ?? "")
      );
    case "is a":
    case "can be":
      return hexagon(20, 16, 17, 9) + label(name === "is a" ? "IS" : "CB", "#000") + arrow(i, 20, 5, 20, 0.5, 1, 1.0);
    case "chen attribute":
      return `<ellipse cx="20" cy="16" rx="15" ry="8" fill="#fff" stroke="#000" stroke-width="1.3"/>`;
    case "attribute tree": {
      let out = `<rect x="2" y="11" width="14" height="10" fill="${GRAY}" stroke="#000" stroke-width="1.3"/>`;
      out += `<g stroke="${i.line}" stroke-width="1.3"><line x1="16" y1="16" x2="20" y2="16"/><line x1="20" y1="7" x2="20" y2="25"/>`;
      for (const y of [7, 16, 25]) out += `<line x1="20" y1="${y}" x2="24" y2="${y}"/><line x1="26" y1="${y}" x2="37" y2="${y}"/>`;
      return out + "</g>";
    }
    case "point":
      return (
        `<circle cx="20" cy="16" r="2.5" fill="${i.line}"/>` +
        `<g stroke="${i.line}" stroke-width="1"><line x1="4" y1="16" x2="16" y2="16"/><line x1="24" y1="16" x2="36" y2="8"/><line x1="24" y1="16" x2="36" y2="24"/></g>`
      );
    case "note":
      return label("Abc", i.red, 12, true);
    case "subdomain":
      return `<g fill="none" stroke="${i.blue}" stroke-width="1.6"><rect x="4" y="5" width="32" height="23"/><polygon points="4,5 20,5 20,9 17,12 4,12"/></g>`;
    case "link":
      return `<line x1="5" y1="26" x2="35" y2="6" stroke="${i.line}" stroke-width="1.3"/>`;
    case "abstract type":
      return `<polygon points="20,5 36,25 4,25" fill="${GRAY}" stroke="#000" stroke-width="1.3"/>`;
    case "representable type":
      return `<rect x="4" y="8" width="32" height="16" rx="5" fill="${GRAY}" stroke="#000" stroke-width="1.3"/>`;
    case "subtype":
      return `<circle cx="20" cy="16" r="12" fill="${GRAY}" stroke="#000" stroke-width="1.3"/>`;
    case "aggregation":
    case "grouping": {
      const k = 10 / Math.SQRT2;
      return (
        `<circle cx="20" cy="16" r="10" fill="#fff" stroke="#000" stroke-width="1.3"/>` +
        `<g stroke="#000" stroke-width="1.3"><line x1="${20 - k}" y1="${16 - k}" x2="${20 + k}" y2="${16 + k}"/><line x1="${20 + k}" y1="${16 - k}" x2="${20 - k}" y2="${16 + k}"/>` +
        (name === "grouping" ? `<line x1="10" y1="16" x2="30" y2="16"/>` : "") +
        "</g>"
      );
    }
    case "attribute":
      return arrow(i, 5, 26, 35, 6);
    case "multi-valued attribute":
      return arrow(i, 5, 26, 35, 6, 2);
    case "component":
      return arrow(i, 5, 26, 35, 6, 1, 1.3, true);
    case "generalization":
      return arrow(i, 5, 26, 35, 6, 1, 3);
    case "specialization":
      return arrow(i, 5, 26, 35, 6, 1, 1.3, false, true);
    default:
      return "";
  }
}
