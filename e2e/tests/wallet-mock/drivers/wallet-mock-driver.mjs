import { assertWalletDriverContract } from "./wallet-driver.mjs";

const DEFAULT_SEED = "test test test test test test test test test test test junk";
const DEFAULT_APP_BASE_URL = process.env.PRD05A_E2E_BASE_URL ?? "http://localhost:7272";

function weiToDecimalString(value) {
  if (typeof value === "number") {
    return String(value);
  }
  if (typeof value === "bigint") {
    return value.toString(10);
  }
  const raw = String(value ?? "0").trim();
  if (raw.startsWith("0x") || raw.startsWith("0X")) {
    return BigInt(raw).toString(10);
  }
  return raw;
}

export class WalletMockDriver {
  constructor({ page, ethereumWalletMock, appBaseUrl = DEFAULT_APP_BASE_URL }) {
    this.page = page;
    this.ethereumWalletMock = ethereumWalletMock;
    this.appBaseUrl = appBaseUrl.replace(/\/$/, "");
    assertWalletDriverContract(this, "wallet-mock-driver");
  }

  resolveAppUrl(path = "/") {
    if (/^https?:\/\//i.test(path)) {
      return path;
    }
    const normalizedPath = path.startsWith("/") ? path : `/${path}`;
    return `${this.appBaseUrl}${normalizedPath}`;
  }

  async openApp(path = "/") {
    await this.page.addInitScript(() => {
      window.__RUSTY_SAFE_RUNTIME_PROFILE = "development";
    });
    await this.page.goto(this.resolveAppUrl(path), {
      waitUntil: "domcontentloaded",
    });
    await this.page.waitForFunction(() => window.__rustySafeE2EAppReady === true, null, {
      timeout: 30000,
    });
    await this.ensureRustySafeE2EBridge();
  }

  async providerRequest(method, params) {
    return await this.page.evaluate(
      async ({ requestMethod, requestParams }) => {
        const payload = { method: requestMethod };
        if (Array.isArray(requestParams)) {
          payload.params = requestParams;
        }
        return await window.ethereum.request(payload);
      },
      { requestMethod: method, requestParams: params ?? null },
    );
  }

  async ensureEmitterBridge() {
    await this.page.evaluate(() => {
      const rootProvider = window.ethereum;
      if (!rootProvider) {
        return;
      }

      const candidates = Array.isArray(rootProvider.providers)
        ? [rootProvider, ...rootProvider.providers]
        : [rootProvider];

      for (const provider of candidates) {
        if (!provider || provider.__rustySafeEmitBridgeReady) {
          continue;
        }

        const listeners = {};
        const originalOn = typeof provider.on === "function" ? provider.on.bind(provider) : null;
        const originalRemove =
          typeof provider.removeListener === "function"
            ? provider.removeListener.bind(provider)
            : null;

        provider.on = (eventName, handler) => {
          listeners[eventName] = listeners[eventName] ?? [];
          listeners[eventName].push(handler);
          if (originalOn) {
            try {
              originalOn(eventName, handler);
            } catch (_error) {
              // no-op: wallet mock may not support native listener plumbing for all events
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
              // no-op
            }
          }
          return provider;
        };

        provider.__rustySafeEmit = (eventName, payload) => {
          for (const handler of listeners[eventName] ?? []) {
            handler(payload);
          }
        };
        provider.__rustySafeEmitBridgeReady = true;
      }
    });
  }

  async ensureRustySafeE2EBridge() {
    await this.page.evaluate(() => {
      if (!Array.isArray(window.__rustySafeE2EQueue)) {
        window.__rustySafeE2EQueue = [];
      }
      if (!window.__rustySafeE2EResults || typeof window.__rustySafeE2EResults !== "object") {
        window.__rustySafeE2EResults = {};
      }
      window.__rustySafeE2EBridgeReady = true;
    });
  }

  async bootstrapWallet(seedPhrase = DEFAULT_SEED) {
    await this.ethereumWalletMock.importWallet(seedPhrase);
  }

  async connectToDapp() {
    return await this.providerRequest("eth_requestAccounts");
  }

  async dispatchSigningCommand(method, params = {}, { timeoutMs = 15000 } = {}) {
    await this.ensureRustySafeE2EBridge();
    const id = `wm-e2e-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    await this.page.evaluate(
      ({ commandId, commandMethod, commandParams }) => {
        window.__rustySafeE2EQueue.push({
          id: commandId,
          method: commandMethod,
          params: commandParams ?? {},
        });
      },
      { commandId: id, commandMethod: method, commandParams: params },
    );
    await this.page.waitForFunction(
      (requestId) => typeof window.__rustySafeE2EResults?.[requestId] === "string",
      id,
      { timeout: timeoutMs },
    );
    const raw = await this.page.evaluate((requestId) => {
      const value = window.__rustySafeE2EResults?.[requestId];
      if (window.__rustySafeE2EResults && requestId in window.__rustySafeE2EResults) {
        delete window.__rustySafeE2EResults[requestId];
      }
      return value;
    }, id);
    const parsed = JSON.parse(raw ?? "{}");
    if (!parsed.ok) {
      throw new Error(`rusty-safe-e2e-command-failed:${method}:${parsed.error ?? "unknown"}`);
    }
    return parsed.result ?? {};
  }

  async openSigningTab({ surface = "Queue" } = {}) {
    await this.openApp("/");
    return await this.dispatchSigningCommand("open_signing_tab", { surface });
  }

  async acquireWriterLock({ holder = "wallet-mock-driver", ttlMs = 60000 } = {}) {
    return await this.dispatchSigningCommand("acquire_writer_lock", {
      holder,
      ttl_ms: ttlMs,
    });
  }

  async createRawTxDraft({
    chainId = 1,
    safeAddress,
    nonce,
    to,
    value = "0",
    data = "0x",
    threshold = 1,
    safeVersion = "1.3.0",
  }) {
    return await this.dispatchSigningCommand("create_raw_tx", {
      chain_id: chainId,
      safe_address: safeAddress,
      nonce,
      to,
      value,
      data,
      threshold,
      safe_version: safeVersion,
    });
  }

  async addTxSignature({ safeTxHash, signer, signature }) {
    return await this.dispatchSigningCommand("add_tx_signature", {
      safe_tx_hash: safeTxHash,
      signer,
      signature,
    });
  }

  async proposeTx({ safeTxHash }) {
    return await this.dispatchSigningCommand("propose_tx", {
      safe_tx_hash: safeTxHash,
    });
  }

  async confirmTx({ safeTxHash, signature }) {
    return await this.dispatchSigningCommand("confirm_tx", {
      safe_tx_hash: safeTxHash,
      signature,
    });
  }

  async executeTx({ safeTxHash }) {
    return await this.dispatchSigningCommand("execute_tx", {
      safe_tx_hash: safeTxHash,
    });
  }

  async loadTx({ safeTxHash }) {
    const result = await this.dispatchSigningCommand("load_tx", {
      safe_tx_hash: safeTxHash,
    });
    return result.tx ?? null;
  }

  async loadMessage({ messageHash }) {
    const result = await this.dispatchSigningCommand("load_message", {
      message_hash: messageHash,
    });
    return result.message ?? null;
  }

  async loadTransitionLog({ flowId }) {
    const result = await this.dispatchSigningCommand("load_transition_log", {
      flow_id: flowId,
    });
    return result.records ?? [];
  }

  async triggerExportBundle({ flowIds }) {
    return await this.dispatchSigningCommand("export_bundle", {
      flow_ids: flowIds,
    });
  }

  async triggerImportBundle({ bundle }) {
    return await this.dispatchSigningCommand("import_bundle", { bundle });
  }

  async triggerImportUrlPayload({
    key,
    payloadBase64url,
    schemaVersion = 1,
  }) {
    return await this.dispatchSigningCommand("import_url_payload", {
      key,
      payload_base64url: payloadBase64url,
      schema_version: schemaVersion,
    });
  }

  async readStatusBanner() {
    return await this.dispatchSigningCommand("get_notice");
  }

  async clearStatusBanner() {
    return await this.dispatchSigningCommand("clear_notice");
  }

  async waitForTxStatus(safeTxHash, expectedStatus, { timeoutMs = 10000 } = {}) {
    const startedAt = Date.now();
    for (;;) {
      const tx = await this.loadTx({ safeTxHash });
      if (tx && String(tx.status) === expectedStatus) {
        return tx;
      }
      if (Date.now() - startedAt > timeoutMs) {
        throw new Error(
          `wallet-mock-wait-for-status-timeout:${safeTxHash}:expected-${expectedStatus}`,
        );
      }
      await this.page.waitForTimeout(100);
    }
  }

  async approveSignature({
    method = "personal_sign",
    params,
    signature = `0x${"11".repeat(65)}`,
  }) {
    if (!Array.isArray(params) || params.length === 0) {
      throw new Error("wallet-mock-signature-params-required");
    }

    await this.page.evaluate(
      ({ signatureParams, signatureResult }) => {
        Web3Mock.mock({
          blockchain: "ethereum",
          signature: {
            params: signatureParams,
            return: signatureResult,
          },
        });
      },
      { signatureParams: params, signatureResult: signature },
    );

    return await this.providerRequest(method, params);
  }

  async approveTransaction({ to, value = "1" }) {
    return await this.ethereumWalletMock.sendTransaction(to, weiToDecimalString(value));
  }

  async approveNetworkChange({ chainId = "0x1" } = {}) {
    await this.ensureEmitterBridge();
    await this.providerRequest("wallet_switchEthereumChain", [{ chainId }]).catch(() => {
      // wallet mock does not always emulate network switching semantics; event emit handles recovery path
    });
    await this.page.evaluate(
      ({ nextChainId }) => window.ethereum?.__rustySafeEmit?.("chainChanged", nextChainId),
      { nextChainId: chainId },
    );
    return chainId;
  }

  async recoverFromFailure(kind, payload) {
    await this.ensureEmitterBridge();
    const emitEvent = async (eventName, eventPayload) => {
      await this.page.evaluate(
        ({ nextEventName, nextPayload }) => {
          const rootProvider = window.ethereum;
          if (!rootProvider) {
            return;
          }
          const candidates = Array.isArray(rootProvider.providers)
            ? [rootProvider, ...rootProvider.providers]
            : [rootProvider];
          for (const provider of candidates) {
            provider?.__rustySafeEmit?.(nextEventName, nextPayload);
          }
        },
        { nextEventName: eventName, nextPayload: eventPayload },
      );
    };
    if (kind === "accountsChanged") {
      await emitEvent("accountsChanged", payload);
      return { recovered: true, kind, payload };
    }
    if (kind === "chainChanged") {
      await emitEvent("chainChanged", payload);
      return { recovered: true, kind, payload };
    }
    return { recovered: false, kind };
  }

  async collectWalletDiagnostics() {
    return await this.page.evaluate(async () => {
      const provider = window.ethereum;
      let chainId = null;
      let accounts = [];
      try {
        chainId = await provider.request({ method: "eth_chainId" });
      } catch (_error) {
        chainId = null;
      }
      try {
        accounts = await provider.request({ method: "eth_accounts" });
      } catch (_error) {
        accounts = [];
      }
      return {
        driver: "wallet-mock",
        hasProvider: Boolean(provider),
        chainId,
        accounts,
      };
    });
  }
}
