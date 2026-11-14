// The website in a real browser (Chromium, Firefox, WebKit): served as GitHub Pages serves it, under a
// sub-path. What only the website does: files without paths, settings and open diagrams kept in the
// browser, exports made in the page, the LaTeX preview greyed out, its own keys and Help entries, and
// working offline after the first visit.
import { readFileSync } from "node:fs";
import { expect, test, type Page } from "@playwright/test";

type Tab = { name: string; path: string | null; modified: boolean; kind: string };
type State = { tabs: Tab[]; current: number; snap: number; status: string; question: string | null };

const hook = <T>(page: Page, fn: string, ...args: unknown[]) =>
  page.evaluate(([f, a]) => (window as unknown as Record<string, Record<string, (...x: unknown[]) => unknown>>).__erdtex[f as string](...(a as unknown[])), [fn, args] as const) as Promise<T>;
const state = (page: Page) => hook<State>(page, "state");

async function start(page: Page) {
  await page.goto("./");
  await page.waitForFunction(() => "__erdtex" in window);
}

test.beforeEach(async ({ page }) => start(page));

test("the first visit opens the two samples, and the page is titled after the diagram", async ({ page }) => {
  await expect.poll(async () => (await state(page)).tabs.map((t) => t.name)).toEqual(["festival.erd", "festival.gsmd"]);
  await expect(page).toHaveTitle(/festival\.erd — ERDTeX/);
});

test("a diagram is opened, edited, saved (downloaded) and kept after a reload", async ({ page }) => {
  const path = await hook<string>(page, "webFile", "people.erd", "\\entity{p}(0,0){Person}\n\\attributes{p}{*PID}\n");
  await hook(page, "answerDialogs", [path]);
  await hook(page, "run", "a_open");
  await expect.poll(async () => { const s = await state(page); return s.tabs[s.current].name; }).toBe("people.erd");
  expect(await hook<string>(page, "text")).toContain("{Person}");
  // an edit, saved: the browser downloads it (no handle to write back to here)
  await hook(page, "paste", "\\entity{c}(5,0){City}\n", 5, 0);
  await expect.poll(async () => { const s = await state(page); return s.tabs[s.current].modified; }).toBe(true);
  const download = page.waitForEvent("download");
  await hook(page, "run", "a_save");
  expect((await download).suggestedFilename()).toBe("people.erd");
  expect(await hook<string>(page, "written", path)).toContain("{City}");
  // after a reload the diagram comes back (as an unsaved tab of the same name)
  await page.waitForTimeout(800);
  await page.reload();
  await page.waitForFunction(() => "__erdtex" in window);
  await expect.poll(async () => (await state(page)).tabs.map((t) => t.name)).toContain("people.erd");
  expect((await hook<string[]>(page, "tabTexts")).some((t) => t.includes("{City}"))).toBe(true);
});

test("a file that isn't a diagram is refused", async ({ page }) => {
  const path = await hook<string>(page, "webFile", "notes.txt", "just some text\n");
  await hook(page, "answerDialogs", [path]);
  await page.evaluate(() => void (window as unknown as { __erdtex: { run(id: string): unknown } }).__erdtex.run("a_open")); // waits for the answer
  await expect.poll(async () => (await state(page)).question ?? "").toContain("is not a diagram");
});

for (const [format, magic] of [["png", "\x89PNG"], ["svg", "<svg"], ["pdf", "%PDF-"]] as const) {
  test(`the editor rendering is exported as ${format.toUpperCase()}`, async ({ page }) => {
    await expect.poll(async () => (await state(page)).tabs.length).toBe(2);
    await hook(page, "run", "a_export");
    await page.locator("[data-testid=export]").waitFor();
    await expect(page.locator("[data-testid=export-latex]")).toBeDisabled();
    await page.locator("[data-testid=export-format]").selectOption(format);
    await hook(page, "savePathOnce", `web:export/festival.${format}`);
    const download = page.waitForEvent("download");
    await page.locator("[data-testid=export-ok]").click();
    const d = await download;
    expect(d.suggestedFilename()).toBe(`festival.${format}`);
    const bytes = readFileSync(await d.path());
    expect(bytes.subarray(0, magic.length).toString("latin1")).toBe(magic);
    expect(bytes.length).toBeGreaterThan(2000);
  });
}

test("the LaTeX preview is greyed out and says why", async ({ page }) => {
  await page.locator(".menubar .title", { hasText: "View" }).click();
  const row = page.locator(".menubar .row-wrap", { hasText: "LaTeX Preview" });
  await expect(row.locator("button")).toBeDisabled();
  await expect(row).toHaveAttribute("title", /pdflatex, which a browser doesn't have/);
  await expect(page.locator("[data-testid=preview]")).toHaveCount(0);
});

test("the LaTeX packages are saved as one zip", async ({ page }) => {
  await hook(page, "answerDialogs", ["web:pkg/latex-packages.zip"]);
  const download = page.waitForEvent("download");
  await hook(page, "run", "a_save_packages");
  const d = await download;
  expect(d.suggestedFilename()).toBe("latex-packages.zip");
  const bytes = readFileSync(await d.path());
  expect(bytes.subarray(0, 2).toString("latin1")).toBe("PK");
  for (const name of ["erdiagram.sty", "gsmdiagram.sty", "erdiagram-manual.pdf", "gsmdiagram-manual.pdf"]) {
    expect(bytes.includes(Buffer.from(name))).toBe(true);
  }
});

test("settings are kept in the browser", async ({ page }) => {
  await page.locator("[data-testid=snap]").selectOption("0.5");
  await expect.poll(async () => (await state(page)).snap).toBe(0.5);
  await page.reload();
  await page.waitForFunction(() => "__erdtex" in window);
  await expect.poll(async () => (await state(page)).snap).toBe(0.5);
});

test("the website's keys and Help entries", async ({ page }) => {
  await expect.poll(async () => (await state(page)).tabs.length).toBe(2);
  await hook(page, "focusCanvas");
  await page.keyboard.press("Alt+N");
  await expect.poll(async () => (await state(page)).tabs.length).toBe(3);
  await page.locator(".menubar .title", { hasText: "Help" }).click();
  const labels = await page.locator(".menubar .dropdown .label").allTextContents();
  expect(labels).toContain("Download the Desktop Version");
  expect(labels).toContain("Source Code on GitHub");
  expect(labels).toContain("Privacy");
  expect(labels).not.toContain("Imprint"); // its file is still empty
  await page.locator(".menubar .row", { hasText: "Privacy" }).click();
  await expect(page.locator("[data-testid=text-page]")).toContainText("runs entirely in your browser");
  await page.locator("[data-testid=text-page-close]").click();
  await page.locator(".menubar .title", { hasText: "File" }).click();
  const file = await page.locator(".menubar .dropdown .label").allTextContents();
  expect(file).not.toContain("Quit");
  expect(file).not.toContain("Open Recent");
});

test("after the first visit it works offline", async ({ page, context }) => {
  await page.evaluate(() => navigator.serviceWorker.ready);
  await page.reload();
  await page.waitForFunction(() => "__erdtex" in window && !!navigator.serviceWorker.controller);
  await context.setOffline(true);
  await page.reload();
  await page.waitForFunction(() => "__erdtex" in window);
  await expect.poll(async () => (await state(page)).tabs.length).toBeGreaterThan(0);
  await context.setOffline(false);
});
