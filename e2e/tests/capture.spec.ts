import { test } from '@playwright/test';

// Standalone capture script - saves screenshots for AI analysis
// Run with: bun run capture

async function waitForCanvasRendered(page: any) {
  // Wait for canvas element
  await page.waitForSelector('canvas');

  // Wait for WASM to initialize and render
  // The loading screen hides when app is ready
  await page.waitForFunction(() => {
    const loading = document.getElementById('loading');
    return !loading || loading.style.display === 'none';
  }, { timeout: 30000 });

  // Extra time for egui to render frames
  await page.waitForTimeout(2000);
}

test('capture current UI state', async ({ page }) => {
  await page.goto('/');
  await waitForCanvasRendered(page);

  // Full page screenshot
  await page.screenshot({
    path: 'screenshots/full.png',
    fullPage: true
  });

  // Individual tabs - using keyboard navigation
  // Tab 1: Verify Safe API (default)
  await page.screenshot({ path: 'screenshots/tab-verify.png' });

  // We can add more targeted captures as needed
  console.log('Screenshots captured successfully');
});
