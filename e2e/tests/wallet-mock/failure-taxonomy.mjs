export function classifyFailureTaxonomy(errorLike) {
  const code = Number(errorLike?.code ?? NaN);
  const message = String(errorLike?.message ?? errorLike ?? "").toLowerCase();

  if (message.includes("node major version must be 20")) {
    return "ENV_BLOCKER";
  }

  if (message.includes("timeout")) {
    return "HARNESS_FAIL";
  }

  if (code === 4001 || message.includes("user rejected")) {
    return "APP_FAIL";
  }

  if (message.includes("metamask had trouble starting") || message.includes("background connection unresponsive")) {
    return "WALLET_FAIL";
  }

  if (message.includes("chain mismatch")) {
    return "APP_FAIL";
  }

  return "APP_FAIL";
}

export function taxonomyTriageLabel(taxonomy) {
  switch (taxonomy) {
    case "ENV_BLOCKER":
      return "triage/env";
    case "HARNESS_FAIL":
      return "triage/harness";
    case "WALLET_FAIL":
      return "triage/wallet";
    default:
      return "triage/app";
  }
}
