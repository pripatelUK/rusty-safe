import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests',

  // Run tests sequentially for consistent screenshots
  fullyParallel: false,

  // Fail the build on CI if you accidentally left test.only in the source code
  forbidOnly: !!process.env.CI,

  // No retries for visual tests - we want deterministic results
  retries: 0,

  // Single worker for consistent rendering
  workers: 1,

  // Reporter
  reporter: [
    ['html', { open: 'never' }],
    ['list']
  ],

  use: {
    // Base URL for the dev server
    baseURL: 'http://localhost:7272',

    // Capture screenshot on failure
    screenshot: 'only-on-failure',

    // Record trace on failure for debugging
    trace: 'on-first-retry',

    // Viewport size
    viewport: { width: 1280, height: 800 },
  },

  // Configure projects for major browsers
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        // Enable GPU for WebGL/canvas rendering
        launchOptions: {
          args: [
            '--enable-webgl',
            '--use-gl=swiftshader',
            '--enable-gpu-rasterization',
          ],
        },
      },
    },
  ],

  // Expect settings for visual comparisons
  expect: {
    toHaveScreenshot: {
      // Allow small differences due to anti-aliasing
      maxDiffPixelRatio: 0.01,
      // Animation threshold
      threshold: 0.2,
    },
  },

  // Web server configuration - start trunk if not already running
  webServer: {
    command: 'cd ../crates/rusty-safe && trunk serve --port 7272',
    url: 'http://localhost:7272',
    reuseExistingServer: !process.env.CI,
    timeout: 120 * 1000, // 2 minutes for WASM build
  },
});
