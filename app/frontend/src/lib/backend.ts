// What the page asks of its platform: the desktop app (platform/desktop.ts, Tauri) or the browser
// (platform/web.ts), chosen when the page is built (vite.config.ts: `#platform`). Both offer the same
// functions (platform/same.ts checks that); what they share is in platform/common.ts.
export * from "./platform/common";
export * from "#platform";
