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

  async connectToDapp() {
    return await tryAction("connectToDapp", () => this._metamask.connectToDapp());
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

