import { defineConfig } from "@playwright/test";

const baseUrl = process.env.PRD05A_E2E_BASE_URL ?? "http://localhost:7272";
const shouldStartWebServer = process.env.PRD05A_E2E_SKIP_WEBSERVER !== "1";
const suiteTimeoutMs = Number.parseInt(process.env.PRD05A_E2E_TEST_TIMEOUT_MS ?? "240000", 10);

export default defineConfig({
  testDir: "./tests/wallet-mock",
  timeout: suiteTimeoutMs,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: [
    ["list"],
    ["html", { open: "never", outputFolder: "playwright-report-wallet-mock" }],
  ],
  use: {
    baseURL: baseUrl,
    headless: process.env.PRD05A_WALLET_MOCK_HEADED === "1" ? false : true,
    locale: "en-US",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: {},
    },
  ],
  webServer: shouldStartWebServer
    ? {
        command: "cd ../crates/rusty-safe && NO_COLOR=true trunk serve --port 7272",
        url: baseUrl,
        reuseExistingServer: false,
        timeout: 300 * 1000,
      }
    : undefined,
});
