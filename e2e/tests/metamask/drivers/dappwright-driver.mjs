import { assertWalletDriverContract } from "./wallet-driver.mjs";

async function tryAction(label, action) {
  try {
    await action();
    return true;
  } catch (error) {
    console.log(`[dappwright-driver] ${label} unavailable: ${String(error?.message ?? error)}`);
    return false;
  }
}

export class DappwrightDriver {
  constructor({ context, metamaskFallback, walletLoader } = {}) {
    this.name = "dappwright";
    this.releaseGateEligible = false;
    this._context = context;
    this._metamaskFallback = metamaskFallback;
    this._walletLoader = walletLoader;
    this._wallet = null;
    this._walletSource = "uninitialized";
    assertWalletDriverContract(this, "dappwright-driver");
  }

  async _resolveWallet() {
    if (this._wallet) {
      return this._wallet;
    }

    if (typeof this._walletLoader === "function") {
      this._wallet = await this._walletLoader();
      this._walletSource = "custom-loader";
      return this._wallet;
    }

    if (!this._context) {
      return null;
    }

    try {
      const { getWallet } = await import("@tenkeylabs/dappwright");
      this._wallet = await getWallet("metamask", this._context);
      this._walletSource = "dappwright";
      return this._wallet;
    } catch (error) {
      console.log(`[dappwright-driver] wallet-load failed: ${String(error?.message ?? error)}`);
      this._wallet = null;
      this._walletSource = "fallback";
      return null;
    }
  }

  async bootstrapWallet() {
    const wallet = await this._resolveWallet();
    if (wallet) {
      return { supported: true, delegatedTo: this._walletSource };
    }
    return { supported: false, delegatedTo: "fallback" };
  }

  async connectToDapp() {
    const wallet = await this._resolveWallet();
    if (wallet && (await tryAction("approve(connect)", () => wallet.approve()))) {
      return true;
    }
    if (this._metamaskFallback) {
      return await tryAction("synpress.connectToDapp", () => this._metamaskFallback.connectToDapp());
    }
    return false;
  }

  async approveSignature() {
    const wallet = await this._resolveWallet();
    if (wallet && (await tryAction("sign(signature)", () => wallet.sign()))) {
      return true;
    }
    if (this._metamaskFallback) {
      return await tryAction("synpress.confirmSignature", () =>
        this._metamaskFallback.confirmSignature(),
      );
    }
    return false;
  }

  async approveTransaction() {
    const wallet = await this._resolveWallet();
    if (wallet && (await tryAction("confirmTransaction", () => wallet.confirmTransaction()))) {
      return true;
    }
    if (this._metamaskFallback) {
      return await tryAction("synpress.confirmTransaction", () =>
        this._metamaskFallback.confirmTransaction(),
      );
    }
    return false;
  }

  async approveNetworkChange() {
    const wallet = await this._resolveWallet();
    if (wallet) {
      const approved = await tryAction("approve(add-network)", () => wallet.approve());
      if (approved) {
        await tryAction("confirmNetworkSwitch", () => wallet.confirmNetworkSwitch());
      }
      return approved;
    }

    if (this._metamaskFallback) {
      const approvedAddNetwork = await tryAction("synpress.approveNewNetwork", () =>
        this._metamaskFallback.approveNewNetwork(),
      );
      if (approvedAddNetwork) {
        await tryAction("synpress.approveSwitchNetwork", () =>
          this._metamaskFallback.approveSwitchNetwork(),
        );
      }
      return approvedAddNetwork;
    }
    return false;
  }

  async recoverFromCrashOrOnboarding() {
    return { supported: false, delegatedTo: "fixture-bootstrap" };
  }

  async collectWalletDiagnostics() {
    return {
      driver: this.name,
      release_gate_eligible: this.releaseGateEligible,
      wallet_source: this._walletSource,
      capabilities: {
        connect: true,
        sign: true,
        send_transaction: true,
        network_change: true,
      },
    };
  }
}

export function createDappwrightDriver(options) {
  return new DappwrightDriver(options);
}

