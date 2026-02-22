import assert from "node:assert/strict";
import test from "node:test";

import { WalletMockDriver } from "./drivers/wallet-mock-driver.mjs";
import { WALLET_DRIVER_METHODS, assertWalletDriverContract } from "./drivers/wallet-driver.mjs";

test("WalletDriver contract rejects invalid implementation", () => {
  assert.throws(
    () => assertWalletDriverContract({ connectToDapp: async () => {} }, "bad-driver"),
    /bad-driver-missing-methods/,
  );
});

test("WalletMockDriver satisfies WalletDriver contract", () => {
  const pageStub = {
    evaluate: async () => ({}),
  };
  const walletStub = {
    importWallet: async () => {},
    sendTransaction: async () => "0xabc",
  };
  const driver = new WalletMockDriver({
    page: pageStub,
    ethereumWalletMock: walletStub,
  });

  assertWalletDriverContract(driver, "wallet-mock-driver");
  for (const method of WALLET_DRIVER_METHODS) {
    assert.equal(typeof driver[method], "function", `method ${method} must exist`);
  }
});
