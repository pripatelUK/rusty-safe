import { defineConfig, devices } from "@playwright/test";

const baseUrl = process.env.PRD05A_E2E_BASE_URL ?? "http://localhost:7272";
const shouldStartWebServer = process.env.PRD05A_E2E_SKIP_WEBSERVER !== "1";

export default defineConfig({
  testDir: "./tests/metamask",
  timeout: 180 * 1000,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: [
    ["list"],
    ["html", { open: "never", outputFolder: "playwright-report-metamask" }],
  ],
  use: {
    baseURL: baseUrl,
    headless: false,
    locale: "en-US",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
      },
    },
  ],
  webServer: shouldStartWebServer
    ? {
        command: "cd ../crates/rusty-safe && NO_COLOR=true trunk serve --port 7272",
        url: baseUrl,
        // Force a clean build/runtime each run; stale reused servers have caused false E2E failures.
        reuseExistingServer: false,
        timeout: 180 * 1000,
      }
    : undefined,
});
