import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import playwrightPkg from "@playwright/test";
import fs from "fs-extra";
import { getExtensionId } from "@synthetixio/synpress-metamask/playwright";

import metamaskSetup from "../../wallet-setup/metamask.anvil.setup.mjs";
import { bootstrapMetaMaskRuntime } from "./metamask-bootstrap.mjs";

const { chromium } = playwrightPkg;

async function run() {
  const hash = metamaskSetup.hash;
  const thisFile = fileURLToPath(import.meta.url);
  const rootDir = path.resolve(path.dirname(thisFile), "../..");
  const cacheDir = path.join(rootDir, ".cache-synpress", hash);
  const extensionPath = path.join(rootDir, ".cache-synpress", "metamask-chrome-13.13.1");

  if (!(await fs.pathExists(cacheDir))) {
    throw new Error(`cache-missing:${cacheDir}`);
  }
  if (!(await fs.pathExists(extensionPath))) {
    throw new Error(`extension-missing:${extensionPath}`);
  }

  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "metamask-preflight-"));
  await fs.copy(cacheDir, tempDir);

  const context = await chromium.launchPersistentContext(tempDir, {
    headless: false,
    args: [`--disable-extensions-except=${extensionPath}`, "--headless=new"],
  });

  try {
    const extensionId = await getExtensionId(context, "MetaMask");
    const page = context.pages()[0] ?? (await context.newPage());
    const bootstrap = await bootstrapMetaMaskRuntime({
      context,
      page,
      extensionId,
      walletSetup: metamaskSetup,
      walletPassword: metamaskSetup.walletPassword,
      maxAttempts: 3,
    });
    console.log(`[metamask-preflight] ${JSON.stringify(bootstrap)}`);
  } finally {
    await context.close();
    await fs.remove(tempDir);
  }
}

run().catch((error) => {
  console.error(`[metamask-preflight] ERROR ${error.message}`);
  process.exit(2);
});
