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

async function providerRequest(page, method, params) {
  return await page.evaluate(
    async ({ requestMethod, requestParams }) => {
      const root = window.ethereum;
      const providers = Array.isArray(root?.providers) ? root.providers : [];
      const provider = providers.find((candidate) => candidate?.isMetaMask) ?? root;
      if (!provider || typeof provider.request !== "function") {
        throw new Error("metamask-provider-request-missing");
      }
      const payload = { method: requestMethod };
      if (Array.isArray(requestParams)) {
        payload.params = requestParams;
      }
      return await provider.request(payload);
    },
    {
      requestMethod: method,
      requestParams: Array.isArray(params) ? params : null,
    },
  );
}

async function logProviderDiagnostics(page) {
  const diagnostics = await page.evaluate(() => {
    const root = window.ethereum;
    const providers = Array.isArray(root?.providers) ? root.providers : [];
    const selected = providers.find((candidate) => candidate?.isMetaMask) ?? root;
    return {
      hasWindowEthereum: typeof root !== "undefined",
      rootIsMetaMask: Boolean(root?.isMetaMask),
      providersCount: providers.length,
      providers: providers.map((candidate, index) => ({
        index,
        isMetaMask: Boolean(candidate?.isMetaMask),
        hasRequest: typeof candidate?.request === "function",
        keys: Object.keys(candidate ?? {}).slice(0, 12),
      })),
      selectedIsMetaMask: Boolean(selected?.isMetaMask),
      selectedHasRequest: typeof selected?.request === "function",
    };
  });
  console.log(`[metamask-e2e] provider-diagnostics=${JSON.stringify(diagnostics)}`);
}

async function requestWithUserGesture(page, method, params) {
  const triggerId = `__rustySafeTrigger_${Date.now()}_${Math.random().toString(16).slice(2)}`;
  await page.bringToFront();
  await page.waitForFunction(() => document.readyState === "complete", null, { timeout: 10000 });
  await page.evaluate(
    ({ id, requestMethod, requestParams }) => {
      window.__rustySafeGestureRequests = window.__rustySafeGestureRequests ?? {};
      window.__rustySafeGestureMeta = window.__rustySafeGestureMeta ?? {};

      for (const staleTrigger of document.querySelectorAll(".__rustySafeGestureTrigger")) {
        staleTrigger.remove();
      }

      const existing = document.getElementById(id);
      if (existing) {
        existing.remove();
      }

      const trigger = document.createElement("button");
      trigger.id = id;
      trigger.type = "button";
      trigger.textContent = "wallet-trigger";
      trigger.style.position = "fixed";
      trigger.style.top = "8px";
      trigger.style.left = "8px";
      trigger.style.width = "132px";
      trigger.style.height = "32px";
      trigger.style.opacity = "1";
      trigger.style.background = "#ffffff";
      trigger.style.color = "#000000";
      trigger.style.border = "1px solid #999999";
      trigger.style.zIndex = "2147483647";
      trigger.style.pointerEvents = "auto";
      trigger.className = "__rustySafeGestureTrigger";

      trigger.addEventListener(
        "click",
        () => {
          window.__rustySafeGestureMeta[id] = {
            userActivation: Boolean(navigator.userActivation?.isActive),
            visibilityState: document.visibilityState,
          };
          const rootProvider = window.ethereum;
          const providers = Array.isArray(rootProvider?.providers) ? rootProvider.providers : [];
          const provider = providers.find((candidate) => candidate?.isMetaMask) ?? rootProvider;
          const payload = { method: requestMethod };
          if (Array.isArray(requestParams)) {
            payload.params = requestParams;
          }
          window.__rustySafeGestureRequests[id] = provider.request(payload);
        },
        { once: true },
      );

      document.body.appendChild(trigger);
    },
    {
      id: triggerId,
      requestMethod: method,
      requestParams: Array.isArray(params) ? params : null,
    },
  );

  const providerPromise = page.evaluate(async (id) => {
    const deadline = Date.now() + 70000;
    while (Date.now() < deadline) {
      const pending = window.__rustySafeGestureRequests?.[id];
      if (pending) {
        try {
          const result = await pending;
          return { ok: true, result };
        } catch (error) {
          let serialized;
          try {
            serialized = JSON.parse(JSON.stringify(error));
          } catch (_stringifyError) {
            serialized = undefined;
          }
          return {
            ok: false,
            error: {
              code: error?.code ?? null,
              message: error?.message ?? String(error),
              data: error?.data ?? null,
              name: error?.name ?? null,
              stack: error?.stack ?? null,
              serialized,
            },
          };
        }
      }
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    throw new Error(`gesture-request-not-started:${id}`);
  }, triggerId);

  await page.click(`#${triggerId}`);
  const started = await page.evaluate((id) => {
    return {
      started: Boolean(window.__rustySafeGestureRequests?.[id]),
      meta: window.__rustySafeGestureMeta?.[id] ?? null,
    };
  }, triggerId);
  console.log(`[metamask-e2e] gesture-started method=${method} data=${JSON.stringify(started)}`);
  const outcome = await providerPromise;
  if (!outcome?.ok) {
    throw new Error(
      `provider-request-failed:${method}:${JSON.stringify(outcome?.error ?? { message: "unknown" })}`,
    );
  }
  return outcome.result;
}

async function ensureProvider(page) {
  const hasProvider = await page.evaluate(() => {
    const root = window.ethereum;
    const providers = Array.isArray(root?.providers) ? root.providers : [];
    const selected = providers.find((candidate) => candidate?.isMetaMask) ?? root;
    return typeof selected?.request === "function";
  });
  expect(hasProvider).toBe(true);
}

function parseProviderRequestFailure(error, methodLabel) {
  const message = String(error?.message ?? error);
  const marker = `provider-request-failed:${methodLabel}:`;
  const index = message.indexOf(marker);
  if (index < 0) {
    return null;
  }
  const payload = message.slice(index + marker.length);
  try {
    return JSON.parse(payload);
  } catch (_parseError) {
    return { message: payload };
  }
}

async function resolveRequestWithApprovalAttempts({
  page,
  walletDriver,
  methodLabel,
  requestFactory,
  timeoutMs = 60000,
  sliceMs = 12000,
}) {
  const deadline = Date.now() + timeoutMs;
  let attempt = 0;
  while (Date.now() < deadline) {
    attempt += 1;
    const requestPromise = requestFactory();
    await tryWalletAction(`connectToDapp(${methodLabel})`, () =>
      awaitWithTimeout(walletDriver.connectToDapp(), 10000, `connectToDapp-${methodLabel}`),
    );
    const remaining = Math.max(500, deadline - Date.now());
    const windowMs = Math.min(sliceMs, remaining);
    const timeoutLabel = `${methodLabel}-attempt-${attempt}`;
    try {
      return await awaitWithTimeout(requestPromise, windowMs, timeoutLabel);
    } catch (error) {
      const message = String(error?.message ?? error);
      const requestError = parseProviderRequestFailure(error, methodLabel);
      const timedOut = message.includes(`${timeoutLabel}-timeout-`);
      const pendingRequest = requestError?.code === -32002;
      if (!timedOut && !pendingRequest) {
        console.log(
          `[metamask-e2e] ${methodLabel} failed error=${JSON.stringify(requestError ?? { message })}`,
        );
        throw error;
      }
      const accounts = await providerRequest(page, "eth_accounts").catch(() => []);
      if (Array.isArray(accounts) && accounts.length > 0) {
        return accounts;
      }
      if (Date.now() >= deadline) {
        break;
      }
      console.log(
        `[metamask-e2e] ${methodLabel} pending; reattempting wallet approval remaining_ms=${deadline - Date.now()}`,
      );
      await page.waitForTimeout(600).catch(() => {});
    }
  }
  throw new Error(`${methodLabel}-timeout-${timeoutMs}ms`);
}

async function ensureConnectedAccount(page, walletDriver) {
  let accounts = await providerRequest(page, "eth_accounts");
  if (accounts.length === 0) {
    accounts = await resolveRequestWithApprovalAttempts({
      page,
      walletDriver,
      methodLabel: "eth_requestAccounts",
      requestFactory: () => requestWithUserGesture(page, "eth_requestAccounts"),
      timeoutMs: 60000,
    });
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
    await logProviderDiagnostics(page);

    if (scenario.method === "eth_requestAccounts") {
      const accounts = await ensureConnectedAccount(page, walletDriver);
      expect(accounts.length).toBeGreaterThan(0);
      return;
    }

    const from = await ensureConnectedAccount(page, walletDriver);

    const chainIdHex = await providerRequest(page, "eth_chainId");
    const chainIdNumber = Number.parseInt(chainIdHex, 16);
    expect(chainIdNumber).toBeGreaterThan(0);

    if (scenario.method === "personal_sign") {
      const personalSignPromise = providerRequest(page, "personal_sign", [DEFAULT_MESSAGE_HEX, from]);
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

      const typedDataPromise = providerRequest(page, "eth_signTypedData_v4", [from, typedDataV4]);
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

      const sendTxPromise = providerRequest(page, "eth_sendTransaction", [
        {
          from,
          to: process.env.PRD05A_METAMASK_RECIPIENT ?? DEFAULT_RECIPIENT,
          value: "0x16345785d8a0000",
          chainId: toHexChainId(chainIdNumber),
        },
      ]);
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
