import { assertWalletDriverContract } from "./wallet-driver.mjs";

const DEFAULT_SEED = "test test test test test test test test test test test junk";

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
  constructor({ page, ethereumWalletMock }) {
    this.page = page;
    this.ethereumWalletMock = ethereumWalletMock;
    assertWalletDriverContract(this, "wallet-mock-driver");
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

  async bootstrapWallet(seedPhrase = DEFAULT_SEED) {
    await this.ethereumWalletMock.importWallet(seedPhrase);
  }

  async connectToDapp() {
    return await this.providerRequest("eth_requestAccounts");
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
