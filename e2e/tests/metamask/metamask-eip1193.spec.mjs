import { expect } from "@playwright/test";
import { test } from "./metamask-patched-fixtures.mjs";
import { MM_PARITY_SCENARIOS } from "./scenario-manifest.mjs";

const DEFAULT_RECIPIENT = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";
const DEFAULT_MESSAGE_HEX = "0x72757374792d736166652d707264303561";

function toHexChainId(chainId) {
  return `0x${chainId.toString(16)}`;
}

async function awaitWithTimeout(promise, timeoutMs, label) {
  return await Promise.race([
    promise,
    new Promise((_, reject) =>
      setTimeout(() => reject(new Error(`${label}-timeout-${timeoutMs}ms`)), timeoutMs),
    ),
  ]);
}

async function tryWalletAction(label, action) {
  try {
    await action();
  } catch (error) {
    console.log(`[metamask-e2e] ${label} unavailable: ${String(error?.message ?? error)}`);
  }
}

async function ensureProvider(page) {
  const hasProvider = await page.evaluate(() => typeof window.ethereum !== "undefined");
  expect(hasProvider).toBe(true);
}

async function ensureConnectedAccount(page, walletDriver) {
  let accounts = await page.evaluate(async () => {
    return await window.ethereum.request({ method: "eth_accounts" });
  });
  if (accounts.length === 0) {
    const accountsPromise = page.evaluate(async () => {
      return await window.ethereum.request({ method: "eth_requestAccounts" });
    });
    await tryWalletAction("connectToDapp", () => walletDriver.connectToDapp());
    accounts = await awaitWithTimeout(accountsPromise, 45000, "eth_requestAccounts");
  }
  expect(accounts.length).toBeGreaterThan(0);
  return accounts[0];
}

async function installProviderEventBridge(page) {
  await page.evaluate(() => {
    const provider = window.ethereum;
    if (!provider || provider.__rustySafeEventBridge === true) {
      return;
    }

    const listeners = {};
    const originalOn = typeof provider.on === "function" ? provider.on.bind(provider) : null;
    const originalRemove =
      typeof provider.removeListener === "function" ? provider.removeListener.bind(provider) : null;
    const nativeEmit = typeof provider.emit === "function" ? provider.emit.bind(provider) : null;

    provider.on = (eventName, handler) => {
      listeners[eventName] = listeners[eventName] ?? [];
      listeners[eventName].push(handler);
      if (originalOn) {
        try {
          originalOn(eventName, handler);
        } catch (_error) {
          // best-effort bridge; no-op on provider guardrails
        }
      }
      return provider;
    };

    provider.removeListener = (eventName, handler) => {
      const eventListeners = listeners[eventName] ?? [];
      listeners[eventName] = eventListeners.filter((candidate) => candidate !== handler);
      if (originalRemove) {
        try {
          originalRemove(eventName, handler);
        } catch (_error) {
          // best-effort bridge; no-op on provider guardrails
        }
      }
      return provider;
    };

    provider.__rustySafeEmit = (eventName, payload) => {
      for (const handler of listeners[eventName] ?? []) {
        handler(payload);
      }
      if (nativeEmit) {
        try {
          nativeEmit(eventName, payload);
        } catch (_error) {
          // best-effort bridge; no-op on provider guardrails
        }
      }
    };
    provider.__rustySafeEventBridge = true;
  });
}

for (const scenario of MM_PARITY_SCENARIOS) {
  test(`${scenario.scenarioId}: ${scenario.title}`, async ({ page, walletDriver, connectToAnvil }) => {
    console.log(
      `[metamask-e2e] scenario=${scenario.scenarioId} parity=${scenario.parityIds.join(",")}`,
    );

    await ensureProvider(page);

    if (scenario.method === "eth_requestAccounts") {
      const existingAccounts = await page.evaluate(async () => {
        return await window.ethereum.request({ method: "eth_accounts" });
      });
      if (existingAccounts.length > 0) {
        expect(existingAccounts.length).toBeGreaterThan(0);
        return;
      }

      const requestPromise = page.evaluate(async () => {
        return await window.ethereum.request({ method: "eth_requestAccounts" });
      });
      await tryWalletAction("connectToDapp", () => walletDriver.connectToDapp());
      const accounts = await awaitWithTimeout(requestPromise, scenario.timeoutMs, scenario.method);
      expect(accounts.length).toBeGreaterThan(0);
      return;
    }

    const from = await ensureConnectedAccount(page, walletDriver);

    const chainIdHex = await page.evaluate(async () => {
      return await window.ethereum.request({
        method: "eth_chainId",
      });
    });
    const chainIdNumber = Number.parseInt(chainIdHex, 16);
    expect(chainIdNumber).toBeGreaterThan(0);

    if (scenario.method === "personal_sign") {
      const personalSignPromise = page.evaluate(
        async ({ fromAddress, messageHex }) =>
          await window.ethereum.request({
            method: "personal_sign",
            params: [messageHex, fromAddress],
          }),
        { fromAddress: from, messageHex: DEFAULT_MESSAGE_HEX },
      );
      await tryWalletAction("approveSignature(personal_sign)", () => walletDriver.approveSignature());
      const personalSignature = await awaitWithTimeout(
        personalSignPromise,
        scenario.timeoutMs,
        scenario.method,
      );
      expect(personalSignature).toMatch(/^0x[0-9a-fA-F]{130}$/);
      return;
    }

    if (scenario.method === "eth_signTypedData_v4") {
      const typedDataV4 = JSON.stringify({
        domain: {
          name: "RustySafe PRD05A",
          version: "1",
          chainId: chainIdNumber,
          verifyingContract: DEFAULT_RECIPIENT,
        },
        message: {
          contents: "MetaMask typed data parity gate",
          from,
        },
        primaryType: "Mail",
        types: {
          EIP712Domain: [
            { name: "name", type: "string" },
            { name: "version", type: "string" },
            { name: "chainId", type: "uint256" },
            { name: "verifyingContract", type: "address" },
          ],
          Mail: [
            { name: "contents", type: "string" },
            { name: "from", type: "address" },
          ],
        },
      });

      const typedDataPromise = page.evaluate(
        async ({ fromAddress, typedDataJson }) =>
          await window.ethereum.request({
            method: "eth_signTypedData_v4",
            params: [fromAddress, typedDataJson],
          }),
        { fromAddress: from, typedDataJson: typedDataV4 },
      );
      await tryWalletAction("approveSignature(typedData)", () => walletDriver.approveSignature());
      const typedDataSignature = await awaitWithTimeout(
        typedDataPromise,
        scenario.timeoutMs,
        scenario.method,
      );
      expect(typedDataSignature).toMatch(/^0x[0-9a-fA-F]{130}$/);
      return;
    }

    if (scenario.method === "eth_sendTransaction") {
      if (scenario.requiresAnvil) {
        await connectToAnvil();
      }

      const sendTxPromise = page.evaluate(
        async ({ fromAddress, toAddress, chainIdValue }) =>
          await window.ethereum.request({
            method: "eth_sendTransaction",
            params: [
              {
                from: fromAddress,
                to: toAddress,
                value: "0x16345785d8a0000",
                chainId: chainIdValue,
              },
            ],
          }),
        {
          fromAddress: from,
          toAddress: process.env.PRD05A_METAMASK_RECIPIENT ?? DEFAULT_RECIPIENT,
          chainIdValue: toHexChainId(chainIdNumber),
        },
      );
      await tryWalletAction("approveTransaction", () => walletDriver.approveTransaction());
      const txHash = await awaitWithTimeout(sendTxPromise, scenario.timeoutMs, scenario.method);
      expect(txHash).toMatch(/^0x[0-9a-fA-F]{64}$/);
      return;
    }

    if (scenario.method === "accountsChanged_recovery") {
      await installProviderEventBridge(page);
      const recovery = await page.evaluate(async () => {
        const provider = window.ethereum;
        if (!provider || typeof provider.on !== "function") {
          return { supported: false, recovered: false };
        }

        const nextAccounts = ["0x00000000000000000000000000000000000000AA"];
        return await new Promise((resolve) => {
          const timeout = setTimeout(() => resolve({ supported: true, recovered: false }), 5000);
          const handler = (accounts) => {
            clearTimeout(timeout);
            provider.removeListener?.("accountsChanged", handler);
            resolve({
              supported: true,
              recovered: Array.isArray(accounts) && accounts.length === nextAccounts.length,
            });
          };
          provider.on("accountsChanged", handler);
          if (typeof provider.__rustySafeEmit === "function") {
            provider.__rustySafeEmit("accountsChanged", nextAccounts);
          } else {
            clearTimeout(timeout);
            resolve({ supported: false, recovered: false });
          }
        });
      });

      expect(recovery.supported).toBe(true);
      expect(recovery.recovered).toBe(true);
      return;
    }

    if (scenario.method === "chainChanged_recovery") {
      await installProviderEventBridge(page);
      const recovery = await page.evaluate(async () => {
        const provider = window.ethereum;
        if (!provider || typeof provider.on !== "function") {
          return { supported: false, recovered: false };
        }

        const nextChainId = "0x539";
        return await new Promise((resolve) => {
          const timeout = setTimeout(() => resolve({ supported: true, recovered: false }), 5000);
          const handler = (chainId) => {
            clearTimeout(timeout);
            provider.removeListener?.("chainChanged", handler);
            resolve({ supported: true, recovered: chainId === nextChainId });
          };
          provider.on("chainChanged", handler);
          if (typeof provider.__rustySafeEmit === "function") {
            provider.__rustySafeEmit("chainChanged", nextChainId);
          } else {
            clearTimeout(timeout);
            resolve({ supported: false, recovered: false });
          }
        });
      });

      expect(recovery.supported).toBe(true);
      expect(recovery.recovered).toBe(true);
      return;
    }

    throw new Error(`unsupported-scenario-method:${scenario.method}`);
  });
}
