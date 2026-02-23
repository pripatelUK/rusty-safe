import { createRequire } from "node:module";
import fs from "node:fs";

import playwrightPkg from "@playwright/test";
import { EthereumWalletMock } from "@synthetixio/ethereum-wallet-mock/playwright";

const { chromium } = playwrightPkg;
const require = createRequire(import.meta.url);
const web3MockPath = require.resolve("@depay/web3-mock/dist/umd/index.bundle.js");

async function run() {
  const expectedLocalePrefix = (process.env.PRD05A_EXPECTED_LOCALE_PREFIX ?? "en").toLowerCase();
  const headed = process.env.PRD05A_WALLET_MOCK_HEADED === "1";
  const browser = await chromium.launch({ headless: !headed });

  const context = await browser.newContext({
    locale: "en-US",
  });
  await context.addInitScript({
    content: fs.readFileSync(web3MockPath, "utf8"),
  });

  try {
    const page = await context.newPage();
    await page.goto("about:blank");

    const wallet = new EthereumWalletMock(page);
    await wallet.importWallet("test test test test test test test test test test test junk");

    const profile = await page.evaluate(async () => {
      const accounts = await window.ethereum.request({ method: "eth_requestAccounts" });
      return {
        navigatorLanguage: navigator.language,
        intlLocale: Intl.DateTimeFormat().resolvedOptions().locale,
        accountsLength: Array.isArray(accounts) ? accounts.length : 0,
        firstAccount: Array.isArray(accounts) ? accounts[0] : null,
      };
    });

    const observedLocale = String(profile.navigatorLanguage ?? profile.intlLocale ?? "unknown").toLowerCase();
    if (!observedLocale.startsWith(expectedLocalePrefix)) {
      throw new Error(
        `wallet-mock-locale-mismatch:${observedLocale}:expected-prefix-${expectedLocalePrefix}`,
      );
    }

    if (profile.accountsLength < 1) {
      throw new Error("wallet-mock-accounts-missing");
    }

    console.log(`[wallet-mock-runtime-profile] ${JSON.stringify(profile)}`);
  } finally {
    await context.close();
    await browser.close();
  }
}

run().catch((error) => {
  console.error(`[wallet-mock-runtime-profile] ERROR ${error.message}`);
  process.exit(2);
});
