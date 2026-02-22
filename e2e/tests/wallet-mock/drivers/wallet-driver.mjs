export const WALLET_DRIVER_METHODS = [
  "bootstrapWallet",
  "connectToDapp",
  "approveSignature",
  "approveTransaction",
  "approveNetworkChange",
  "recoverFromFailure",
  "collectWalletDiagnostics",
];

export function assertWalletDriverContract(driver, driverName = "wallet-driver") {
  if (!driver || typeof driver !== "object") {
    throw new Error(`${driverName}-invalid-instance`);
  }

  const missing = WALLET_DRIVER_METHODS.filter((method) => typeof driver[method] !== "function");
  if (missing.length > 0) {
    throw new Error(`${driverName}-missing-methods:${missing.join(",")}`);
  }
}
