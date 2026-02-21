import path from "node:path";

import fs from "fs-extra";
import { test as base, chromium } from "@playwright/test";
import { CACHE_DIR_NAME, createTempContextDir, removeTempContextDir } from "@synthetixio/synpress-cache";
import { createPool } from "@viem/anvil";
import { MetaMask, getExtensionId } from "@synthetixio/synpress-metamask/playwright";

import metamaskSetup from "../../wallet-setup/metamask.anvil.setup.mjs";
import { bootstrapMetaMaskRuntime } from "./metamask-bootstrap.mjs";
import { createSynpressDriver } from "./drivers/synpress-driver.mjs";

let sharedMetaMaskPage;
let sharedExtensionId;

function resolveDriverMode() {
  const mode = String(process.env.PRD05A_DRIVER_MODE ?? "synpress").toLowerCase();
  if (mode !== "synpress") {
    throw new Error(`unsupported-driver-mode-for-e1:${mode}:expected-synpress`);
  }
  return mode;
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
    sharedMetaMaskPage = context.pages()[0] ?? (await context.newPage());

    await bootstrapMetaMaskRuntime({
      context,
      page: sharedMetaMaskPage,
      extensionId: sharedExtensionId,
      walletSetup: metamaskSetup,
      walletPassword,
      maxAttempts: 3,
    });

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
    if (driverMode !== "synpress") {
      throw new Error(`wallet-driver-mode-not-supported:${driverMode}`);
    }
    const driver = createSynpressDriver(metamask);
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
          await window.ethereum.request({
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
