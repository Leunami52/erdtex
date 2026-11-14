// The website's tests (e2e-web/), in Chromium, Firefox and WebKit, against the built site served
// under a sub-path as GitHub Pages serves it: npm run build:web, then npx playwright test.
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "e2e-web",
  timeout: 60_000,
  fullyParallel: false,
  workers: 1,
  reporter: [["list"]],
  use: { baseURL: "http://127.0.0.1:4174/erdtex/", trace: "retain-on-failure", viewport: { width: 1400, height: 900 } },
  webServer: {
    command: "npx vite preview --mode web --port 4174 --strictPort --host 127.0.0.1 --base /erdtex/",
    url: "http://127.0.0.1:4174/erdtex/",
    reuseExistingServer: false,
    timeout: 60_000,
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"], viewport: { width: 1400, height: 900 } } },
    { name: "firefox", use: { ...devices["Desktop Firefox"], viewport: { width: 1400, height: 900 } } },
    { name: "webkit", use: { ...devices["Desktop Safari"], viewport: { width: 1400, height: 900 } } },
  ],
});
