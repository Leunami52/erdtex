// The icons of the menus and the toolbar (Lucide, ISC licence), drawn in the text colour so that
// they suit the light and the dark panels.  Like the Python app's, which took them from the
// desktop's theme; these look the same on every system.
import {
  ArrowLeft, ArrowLeftToLine, BookOpen, PackageOpen, Lightbulb, Download, Globe, Github, ShieldCheck, Scale, ArrowRight, ArrowRightLeft, ArrowRightToLine, Columns2, ListPlus, Rows2, WandSparkles, ClipboardCopy, ClipboardPaste, Copy, CopyPlus, Database, FileOutput,
  FilePlus, FilePlus2, FileText, FolderOpen, Grid3x3, History, Info, Keyboard, LogOut, PanelBottom, PanelLeft,
  PanelRight, Redo2, RefreshCw, Save, SaveAll, Scan, Scissors, Search, Settings, SquareDashed, SunMoon, Trash2,
  Undo2, X, ZoomIn, ZoomOut,
} from "lucide";

/** Lucide's form: ["svg", attributes, [[tag, attributes], …]] */
type Icon = [string, Record<string, string | number>, [string, Record<string, string | number>][]];

const ICONS: Record<string, Icon> = {
  a_new_er: FilePlus, a_new_gsm: FilePlus2, a_open: FolderOpen, recent: History, example: Lightbulb, a_example_er: Lightbulb, a_example_gsm: Lightbulb, download: Download, website: Globe, source: Github, privacy: ShieldCheck, imprint: Scale, a_save: Save, a_save_as: SaveAll,
  a_reload: RefreshCw, a_close_tab: X, a_export: FileOutput, a_relational: Database, a_convert: ArrowRightLeft,
  a_copy_input: ClipboardCopy, a_quit: LogOut, a_undo: Undo2, a_redo: Redo2, a_cut: Scissors, a_copy: Copy,
  a_paste: ClipboardPaste, a_dup: CopyPlus, delete: Trash2, a_all: SquareDashed, a_zoom_in: ZoomIn,
  a_zoom_out: ZoomOut, a_zoom_100: Search, a_fit: Scan, a_grid: Grid3x3, a_types: FileText, mode: SunMoon,
  a_next_tab: ArrowRight, a_prev_tab: ArrowLeft, a_split_right: Columns2, a_split_down: Rows2,
  a_move_tab_group: ArrowRightLeft, a_add_attributes: ListPlus, a_arrange: WandSparkles, a_arrange_selection: WandSparkles, a_next_group: ArrowRightToLine, a_prev_group: ArrowLeftToLine, a_preferences: Settings, a_keys: Keyboard, a_rulebook: BookOpen, a_save_packages: PackageOpen, a_about: Info,
  tools: PanelLeft, element: PanelRight, preview: PanelRight, problems: PanelBottom,
} as unknown as Record<string, Icon>;

/** An icon as SVG markup, `size` pixels; empty for a command without one. */
export function actionIcon(id: string, size = 16): string {
  const icon = ICONS[id];
  if (!icon) return "";
  const inner = icon[2]
    .map(([tag, attrs]) => `<${tag} ${Object.entries(attrs).map(([k, v]) => `${k}="${v}"`).join(" ")}/>`)
    .join("");
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">${inner}</svg>`;
}
