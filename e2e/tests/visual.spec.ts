import { test, expect } from '@playwright/test';

// Helper to wait for egui canvas to render
async function waitForCanvas(page: any) {
  // Wait for the canvas element to exist
  await page.waitForSelector('canvas');
  // Give egui time to render initial frame
  await page.waitForTimeout(1000);
}

test.describe('Visual Regression Tests', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForCanvas(page);
  });

  test.describe('Default State', () => {
    test('homepage loads correctly', async ({ page }) => {
      await expect(page).toHaveScreenshot('homepage.png');
    });
  });

  test.describe('Tabs', () => {
    test('verify safe api tab (default)', async ({ page }) => {
      // This is the default tab
      await expect(page).toHaveScreenshot('tab-verify-safe-api.png');
    });

    test('message tab', async ({ page }) => {
      // Click somewhere in the Message tab area
      // Since egui renders to canvas, we use coordinates
      // Message tab is typically the second tab
      await page.mouse.click(200, 50);
      await page.waitForTimeout(500);
      await expect(page).toHaveScreenshot('tab-message.png');
    });

    test('eip-712 tab', async ({ page }) => {
      // EIP-712 tab is typically the third tab
      await page.mouse.click(300, 50);
      await page.waitForTimeout(500);
      await expect(page).toHaveScreenshot('tab-eip712.png');
    });

    test('offline tab', async ({ page }) => {
      // Offline tab is typically the fourth tab
      await page.mouse.click(400, 50);
      await page.waitForTimeout(500);
      await expect(page).toHaveScreenshot('tab-offline.png');
    });
  });

  test.describe('Sidebar', () => {
    test('sidebar default state', async ({ page }) => {
      // Take screenshot focusing on sidebar area (left side)
      await expect(page).toHaveScreenshot('sidebar-default.png', {
        clip: { x: 0, y: 0, width: 300, height: 800 }
      });
    });
  });

  test.describe('Responsive', () => {
    test('narrow viewport', async ({ page }) => {
      await page.setViewportSize({ width: 800, height: 600 });
      await page.waitForTimeout(500);
      await expect(page).toHaveScreenshot('viewport-narrow.png');
    });

    test('wide viewport', async ({ page }) => {
      await page.setViewportSize({ width: 1920, height: 1080 });
      await page.waitForTimeout(500);
      await expect(page).toHaveScreenshot('viewport-wide.png');
    });
  });

});

// Note: These tests use coordinate-based clicking since egui renders to canvas.
// If tab positions change significantly, these coordinates may need updating.
//
// To find correct coordinates:
// 1. Run `bun run test:headed` to see the browser
// 2. Use browser dev tools to find element positions
// 3. Update coordinates as needed
//
// Alternative: Use keyboard navigation if egui supports it
