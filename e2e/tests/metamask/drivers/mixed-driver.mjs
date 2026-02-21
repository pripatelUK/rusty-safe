import { assertWalletDriverContract } from "./wallet-driver.mjs";

async function tryPrimaryOrFallback(primaryCall, fallbackCall) {
  const primaryOk = await primaryCall();
  if (primaryOk) {
    return true;
  }
  return await fallbackCall();
}

export class MixedDriver {
  constructor(primaryDriver, fallbackDriver) {
    this.name = "mixed";
    this.releaseGateEligible = false;
    this._primary = primaryDriver;
    this._fallback = fallbackDriver;
    assertWalletDriverContract(this, "mixed-driver");
  }

  async bootstrapWallet() {
    await this._primary.bootstrapWallet();
    await this._fallback.bootstrapWallet();
    return { supported: true, delegatedTo: "mixed" };
  }

  async connectToDapp() {
    return await tryPrimaryOrFallback(
      () => this._primary.connectToDapp(),
      () => this._fallback.connectToDapp(),
    );
  }

  async approveSignature() {
    return await tryPrimaryOrFallback(
      () => this._primary.approveSignature(),
      () => this._fallback.approveSignature(),
    );
  }

  async approveTransaction() {
    return await tryPrimaryOrFallback(
      () => this._primary.approveTransaction(),
      () => this._fallback.approveTransaction(),
    );
  }

  async approveNetworkChange() {
    return await tryPrimaryOrFallback(
      () => this._primary.approveNetworkChange(),
      () => this._fallback.approveNetworkChange(),
    );
  }

  async recoverFromCrashOrOnboarding() {
    return { supported: false, delegatedTo: "fixture-bootstrap" };
  }

  async collectWalletDiagnostics() {
    return {
      driver: this.name,
      release_gate_eligible: this.releaseGateEligible,
      primary: await this._primary.collectWalletDiagnostics(),
      fallback: await this._fallback.collectWalletDiagnostics(),
    };
  }
}

export function createMixedDriver(primaryDriver, fallbackDriver) {
  return new MixedDriver(primaryDriver, fallbackDriver);
}

