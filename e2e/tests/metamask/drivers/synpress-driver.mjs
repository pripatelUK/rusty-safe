import { assertWalletDriverContract } from "./wallet-driver.mjs";

async function tryAction(label, action) {
  try {
    await action();
    return true;
  } catch (error) {
    console.log(`[synpress-driver] ${label} unavailable: ${String(error?.message ?? error)}`);
    return false;
  }
}

export class SynpressDriver {
  constructor(metamask) {
    this.name = "synpress";
    this.releaseGateEligible = true;
    this._metamask = metamask;
    assertWalletDriverContract(this, "synpress-driver");
  }

  async bootstrapWallet() {
    // Bootstrap is handled in fixture setup via bootstrapMetaMaskRuntime.
    return { supported: true, delegatedTo: "fixture-bootstrap" };
  }

  async _approveWithDappwrightFallback(label) {
    try {
      const { getWallet } = await import("@tenkeylabs/dappwright");
      const wallet = await getWallet("metamask", this._metamask.context);
      let lastError = null;
      for (let attempt = 0; attempt < 3; attempt += 1) {
        try {
          await wallet.approve();
          console.log(
            `[synpress-driver] ${label} fallback approved via dappwright (attempt=${attempt + 1})`,
          );
          return true;
        } catch (error) {
          lastError = error;
          await new Promise((resolve) => setTimeout(resolve, 1000));
        }
      }
      console.log(
        `[synpress-driver] ${label} dappwright fallback unavailable: ${String(lastError?.message ?? lastError)}`,
      );
      return false;
    } catch (error) {
      console.log(
        `[synpress-driver] ${label} dappwright loader unavailable: ${String(error?.message ?? error)}`,
      );
      return false;
    }
  }

  async connectToDapp() {
    try {
      await this._metamask.connectToDapp();
      return true;
    } catch (error) {
      console.log(`[synpress-driver] connectToDapp unavailable: ${String(error?.message ?? error)}`);
      const pageUrls = this._metamask.context
        .pages()
        .filter((page) => !page.isClosed())
        .map((page) => page.url());
      console.log(`[synpress-driver] connectToDapp context-pages=${JSON.stringify(pageUrls)}`);
      return await this._approveWithDappwrightFallback("connectToDapp");
    }
  }

  async approveSignature() {
    return await tryAction("confirmSignature", () => this._metamask.confirmSignature());
  }

  async approveTransaction() {
    return await tryAction("confirmTransaction", () => this._metamask.confirmTransaction());
  }

  async approveNetworkChange() {
    const approvedAddNetwork = await tryAction("approveNewNetwork", () =>
      this._metamask.approveNewNetwork(),
    );
    if (approvedAddNetwork) {
      await tryAction("approveSwitchNetwork", () => this._metamask.approveSwitchNetwork());
    }
    return approvedAddNetwork;
  }

  async recoverFromCrashOrOnboarding() {
    // Synpress handles restart/setup indirectly through the fixture bootstrap.
    return { supported: false, delegatedTo: "fixture-bootstrap" };
  }

  async collectWalletDiagnostics() {
    return {
      driver: this.name,
      release_gate_eligible: this.releaseGateEligible,
      source: "synpress",
      capabilities: {
        connect: true,
        sign: true,
        send_transaction: true,
        network_change: true,
      },
    };
  }
}

export function createSynpressDriver(metamask) {
  return new SynpressDriver(metamask);
}
