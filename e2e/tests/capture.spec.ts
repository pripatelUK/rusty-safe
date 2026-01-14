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

test('capture with transaction data', async ({ page }) => {
  await page.goto('/');
  await waitForCanvasRendered(page);

  const canvas = page.locator('canvas');

  // Enter Safe Address
  await canvas.click({ position: { x: 127, y: 210 } });
  await page.waitForTimeout(500);
  await page.keyboard.press('Control+a');
  await page.keyboard.type('0x595362A906a0B4e90AEe1430b1f80FDDe2bE80de', { delay: 3 });
  await page.waitForTimeout(500);

  // Click Fetch Details
  await canvas.click({ position: { x: 70, y: 290 } });
  await page.waitForTimeout(4000);

  // Enter Nonce
  await canvas.click({ position: { x: 430, y: 147 } });
  await page.waitForTimeout(300);
  await page.keyboard.press('Control+a');
  await page.keyboard.type('5');
  await page.waitForTimeout(500);

  // Click Fetch & Verify
  await canvas.click({ position: { x: 352, y: 222 } });
  await page.waitForTimeout(6000);

  // Scroll to see all verification results
  await canvas.click({ position: { x: 640, y: 400 } });
  await page.mouse.wheel(0, 250);
  await page.waitForTimeout(500);

  await page.screenshot({ path: 'screenshots/full-with-data.png', fullPage: true });
  console.log('Transaction data screenshot captured');
});
