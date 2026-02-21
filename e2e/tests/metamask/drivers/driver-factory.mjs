import { createDappwrightDriver } from "./dappwright-driver.mjs";
import { createMixedDriver } from "./mixed-driver.mjs";
import { createSynpressDriver } from "./synpress-driver.mjs";

export const SUPPORTED_DRIVER_MODES = ["synpress", "dappwright", "mixed"];
export const RELEASE_GATE_DRIVER = "synpress";

export function resolveDriverMode(rawMode = process.env.PRD05A_DRIVER_MODE ?? "synpress") {
  const mode = String(rawMode).toLowerCase();
  if (!SUPPORTED_DRIVER_MODES.includes(mode)) {
    throw new Error(
      `unsupported-driver-mode:${mode}:expected-${SUPPORTED_DRIVER_MODES.join("|")}`,
    );
  }
  return mode;
}

export async function createWalletDriver({ context, metamask, driverMode }) {
  const mode = resolveDriverMode(driverMode);
  const synpressDriver = createSynpressDriver(metamask);

  if (mode === "synpress") {
    return synpressDriver;
  }

  const dappwrightDriver = createDappwrightDriver({
    context,
    metamaskFallback: metamask,
  });

  if (mode === "dappwright") {
    return dappwrightDriver;
  }

  return createMixedDriver(dappwrightDriver, synpressDriver);
}

export function releaseDriverPolicy() {
  return {
    releaseGateDriver: RELEASE_GATE_DRIVER,
    promotionCriteria:
      "dappwright promotion requires >=95% pass in 20-run CI soak and zero HARNESS_FAIL in 2 consecutive daily runs.",
    fallbackPolicy:
      "if dappwright run fails bootstrap/connect/network probes, downgrade gate driver to synpress for release jobs.",
  };
}

