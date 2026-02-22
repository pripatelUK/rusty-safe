import path from "node:path";

import fs from "fs-extra";
import { test as base, chromium } from "@playwright/test";
import { CACHE_DIR_NAME, createTempContextDir, removeTempContextDir } from "@synthetixio/synpress-cache";
import { createPool } from "@viem/anvil";
import { MetaMask, getExtensionId } from "@synthetixio/synpress-metamask/playwright";

import metamaskSetup from "../../wallet-setup/metamask.anvil.setup.mjs";
import { bootstrapMetaMaskRuntime } from "./metamask-bootstrap.mjs";
import { createWalletDriver, resolveDriverMode } from "./drivers/driver-factory.mjs";

let sharedMetaMaskPage;
let sharedExtensionId;

async function resolveMetaMaskHomePage(context, extensionId) {
  const homeUrl = `chrome-extension://${extensionId}/home.html`;
  const extensionOrigin = `chrome-extension://${extensionId}/`;
  const extensionPages = context
    .pages()
    .filter((candidate) => !candidate.isClosed() && candidate.url().startsWith(extensionOrigin));
  const homePage = extensionPages.find((candidate) => candidate.url().startsWith(homeUrl));
  if (homePage) {
    return homePage;
  }
  const existingPage = extensionPages[0];
  if (existingPage) {
    await existingPage.goto(homeUrl).catch(() => {});
    return existingPage;
  }
  const createdPage = await context.newPage();
  await createdPage.goto(homeUrl).catch(() => {});
  return createdPage;
}

async function initializeMetaMaskContext(context, extensionId, walletSetup, walletPassword) {
  const homeUrl = `chrome-extension://${extensionId}/home.html`;
  let page = await resolveMetaMaskHomePage(context, extensionId);
  const bootstrap = await bootstrapMetaMaskRuntime({
    context,
    page,
    extensionId,
    walletSetup,
    walletPassword,
    maxAttempts: 4,
  });
  console.log(`[metamask-fixture] bootstrap=${JSON.stringify(bootstrap)}`);
  page = await resolveMetaMaskHomePage(context, extensionId);
  // Do not aggressively close extension tabs here.
  // MetaMask may keep multiple home surfaces that are involved in routing request approvals.
  page = context.pages().find((candidate) => candidate.url().startsWith(homeUrl)) ?? page;
  await page.bringToFront().catch(() => {});
  return page;
}

export const test = base.extend({
  _contextPath: async ({ browserName }, use, testInfo) => {
    const contextPath = await createTempContextDir(browserName, testInfo.testId);
    await use(contextPath);
    const error = await removeTempContextDir(contextPath);
    if (error) {
      console.error(error);
    }
  },

  context: async ({ _contextPath }, use) => {
    const { walletPassword, hash } = metamaskSetup;
    const cacheDirPath = path.join(process.cwd(), CACHE_DIR_NAME, hash);
    if (!(await fs.pathExists(cacheDirPath))) {
      throw new Error(`Cache for ${hash} does not exist. Create it first.`);
    }

    await fs.copy(cacheDirPath, _contextPath);

    const metamaskPath = path.join(process.cwd(), CACHE_DIR_NAME, "metamask-chrome-13.13.1");
    if (!(await fs.pathExists(metamaskPath))) {
      throw new Error(`MetaMask extension path missing at ${metamaskPath}. Run wallet setup first.`);
    }

    const browserArgs = [`--disable-extensions-except=${metamaskPath}`, "--lang=en-US"];

    const context = await chromium.launchPersistentContext(_contextPath, {
      headless: false,
      args: browserArgs,
      locale: "en-US",
      slowMo: 0,
    });

    sharedExtensionId = await getExtensionId(context, "MetaMask");
    sharedMetaMaskPage = await initializeMetaMaskContext(
      context,
      sharedExtensionId,
      metamaskSetup,
      walletPassword,
    );

    await use(context);
    await context.close();
  },

  metamaskPage: async ({ context: _unused }, use) => {
    await use(sharedMetaMaskPage);
  },

  extensionId: async ({ context: _unused }, use) => {
    await use(sharedExtensionId);
  },

  metamask: async ({ context, extensionId }, use) => {
    const metamask = new MetaMask(
      context,
      sharedMetaMaskPage,
      metamaskSetup.walletPassword,
      extensionId,
    );
    await use(metamask);
  },

  driverMode: async ({ context: _unused }, use) => {
    await use(resolveDriverMode());
  },

  walletDriver: async ({ metamask, driverMode }, use) => {
    const driver = await createWalletDriver({
      context: sharedMetaMaskPage.context(),
      metamask,
      driverMode,
    });
    await driver.bootstrapWallet();
    await use(driver);
  },

  page: async ({ page }, use) => {
    await page.goto("/");
    await use(page);
  },

  createAnvilNode: async ({ context: _unused }, use) => {
    const pool = createPool();
    await use(async (options) => {
      const nodeId = Array.from(pool.instances()).length;
      const anvil = await pool.start(nodeId, options);
      const rpcUrl = `http://${anvil.host}:${anvil.port}`;
      const chainId = options?.chainId ?? 31337;
      return { anvil, rpcUrl, chainId };
    });
    await pool.empty();
  },

  connectToAnvil: async ({ walletDriver, createAnvilNode, page }, use) => {
    await use(async () => {
      const { rpcUrl, chainId } = await createAnvilNode({ chainId: 1338 });
      const chainIdHex = `0x${chainId.toString(16)}`;
      await page.waitForFunction(() => typeof window.ethereum !== "undefined", null, {
        timeout: 30000,
      });
      const addNetworkPromise = page.evaluate(
        async ({ rpcUrlValue, chainIdHexValue }) =>
          await (
            (Array.isArray(window.ethereum?.providers)
              ? window.ethereum.providers.find((candidate) => candidate?.isMetaMask)
              : null) ?? window.ethereum
          ).request({
            method: "wallet_addEthereumChain",
            params: [
              {
                chainId: chainIdHexValue,
                chainName: "Anvil",
                nativeCurrency: {
                  name: "Ether",
                  symbol: "ETH",
                  decimals: 18,
                },
                rpcUrls: [rpcUrlValue],
                blockExplorerUrls: ["https://etherscan.io/"],
              },
            ],
          }),
        {
          rpcUrlValue: rpcUrl,
          chainIdHexValue: chainIdHex,
        },
      );
      const addNetworkOutcomePromise = addNetworkPromise
        .then(() => ({ ok: true, error: null }))
        .catch((error) => ({ ok: false, error: String(error?.message ?? error) }));

      await walletDriver.approveNetworkChange();

      const addNetworkOutcome = await Promise.race([
        addNetworkOutcomePromise,
        new Promise((resolve) =>
          setTimeout(() => resolve({ ok: false, error: "wallet_addEthereumChain-timeout" }), 30000),
        ),
      ]);

      if (!addNetworkOutcome.ok) {
        throw new Error(`wallet_addEthereumChain failed: ${addNetworkOutcome.error}`);
      }
    });
  },
});
