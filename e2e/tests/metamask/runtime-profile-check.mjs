import os from "node:os";
import path from "node:path";

import playwrightPkg from "@playwright/test";
import fs from "fs-extra";

const { chromium } = playwrightPkg;

async function run() {
  const expectedLocalePrefix = (process.env.PRD05A_EXPECTED_LOCALE_PREFIX ?? "en").toLowerCase();
  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "prd05a-runtime-profile-"));

  const context = await chromium.launchPersistentContext(tempDir, {
    headless: false,
    locale: "en-US",
    args: ["--lang=en-US"],
  });

  try {
    const page = context.pages()[0] ?? (await context.newPage());
    await page.goto("about:blank");
    const profile = await page.evaluate(() => {
      return {
        navigatorLanguage: navigator.language,
        navigatorLanguages: navigator.languages,
        intlLocale: Intl.DateTimeFormat().resolvedOptions().locale,
      };
    });

    const observed = String(profile.navigatorLanguage ?? profile.intlLocale ?? "unknown").toLowerCase();
    console.log(`[runtime-profile] ${JSON.stringify(profile)}`);
    if (!observed.startsWith(expectedLocalePrefix)) {
      throw new Error(
        `runtime-profile-locale-mismatch:${observed}:expected-prefix-${expectedLocalePrefix}`,
      );
    }
  } finally {
    await context.close();
    await fs.remove(tempDir);
  }
}

run().catch((error) => {
  console.error(`[runtime-profile] ERROR ${error.message}`);
  process.exit(2);
});
