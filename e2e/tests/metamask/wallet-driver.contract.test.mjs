import assert from "node:assert/strict";
import test from "node:test";

import { SynpressDriver } from "./drivers/synpress-driver.mjs";
import { WALLET_DRIVER_METHODS, assertWalletDriverContract } from "./drivers/wallet-driver.mjs";

function createMetaMaskStub() {
  const calls = [];
  return {
    calls,
    connectToDapp: async () => calls.push("connectToDapp"),
    confirmSignature: async () => calls.push("confirmSignature"),
    confirmTransaction: async () => calls.push("confirmTransaction"),
    approveNewNetwork: async () => calls.push("approveNewNetwork"),
    approveSwitchNetwork: async () => calls.push("approveSwitchNetwork"),
  };
}

test("WalletDriver contract rejects invalid implementation", () => {
  assert.throws(
    () => assertWalletDriverContract({ connectToDapp: async () => {} }, "bad-driver"),
    /bad-driver-missing-methods/,
  );
});

test("SynpressDriver satisfies WalletDriver contract", async () => {
  const stub = createMetaMaskStub();
  const driver = new SynpressDriver(stub);
  assertWalletDriverContract(driver, "synpress-driver");

  for (const method of WALLET_DRIVER_METHODS) {
    assert.equal(typeof driver[method], "function", `method ${method} must exist`);
  }
});

test("SynpressDriver delegates approval primitives", async () => {
  const stub = createMetaMaskStub();
  const driver = new SynpressDriver(stub);

  await driver.bootstrapWallet();
  await driver.connectToDapp();
  await driver.approveSignature();
  await driver.approveTransaction();
  await driver.approveNetworkChange();
  const diagnostics = await driver.collectWalletDiagnostics();

  assert.deepEqual(stub.calls, [
    "connectToDapp",
    "confirmSignature",
    "confirmTransaction",
    "approveNewNetwork",
    "approveSwitchNetwork",
  ]);
  assert.equal(diagnostics.driver, "synpress");
});

