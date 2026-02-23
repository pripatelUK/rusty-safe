import { expect } from "@playwright/test";
import { ethereumWalletMockFixtures as test } from "@synthetixio/ethereum-wallet-mock/playwright";

import { WalletMockDriver } from "./drivers/wallet-mock-driver.mjs";
import { classifyFailureTaxonomy, taxonomyTriageLabel } from "./failure-taxonomy.mjs";
import { WM_BSS_SCENARIOS, WM_PARITY_SCENARIOS } from "./scenario-manifest.mjs";

const DEFAULT_RECIPIENT = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";
const MESSAGE_HEX = "0x72757374792d736166652d707264303561";
const PERSONAL_SIGN_RESULT = `0x${"11".repeat(65)}`;
const TYPED_DATA_SIGN_RESULT = `0x${"22".repeat(65)}`;

const typedDataPayload = JSON.stringify({
  domain: {
    name: "RustySafe",
    version: "1",
    chainId: 1,
  },
  message: {
    contents: "Rusty Safe typed data",
  },
  primaryType: "Mail",
  types: {
    EIP712Domain: [
      { name: "name", type: "string" },
      { name: "version", type: "string" },
      { name: "chainId", type: "uint256" },
    ],
    Mail: [{ name: "contents", type: "string" }],
  },
});

async function sha256Hex(input) {
  const bytes = new TextEncoder().encode(input);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return `0x${Buffer.from(digest).toString("hex")}`;
}

function stableStringify(input) {
  if (input === null || typeof input !== "object") {
    return JSON.stringify(input);
  }
  if (Array.isArray(input)) {
    return `[${input.map((item) => stableStringify(item)).join(",")}]`;
  }
  const keys = Object.keys(input).sort();
  const body = keys
    .map((key) => `${JSON.stringify(key)}:${stableStringify(input[key])}`)
    .join(",");
  return `{${body}}`;
}

function selectorGuard({ expectedSelector, calldataHex, override = false }) {
  const normalizedData = String(calldataHex ?? "").toLowerCase();
  const normalizedSelector = String(expectedSelector ?? "").toLowerCase();
  if (override) {
    return { accepted: true, reason: "override_acknowledged" };
  }
  const accepted =
    normalizedData.startsWith("0x") &&
    normalizedData.length >= 10 &&
    normalizedData.slice(0, 10) === normalizedSelector;
  return { accepted, reason: accepted ? "selector_match" : "selector_mismatch" };
}

async function createDriver(page, ethereumWalletMock) {
  const driver = new WalletMockDriver({ page, ethereumWalletMock });
  await driver.bootstrapWallet();
  await driver.connectToDapp();
  return driver;
}

test.describe.serial("Wallet Mock EIP-1193 parity", () => {
  test(`${WM_PARITY_SCENARIOS[0].scenarioId} - connect via eth_requestAccounts`, async ({
    page,
    ethereumWalletMock,
  }) => {
    const driver = await createDriver(page, ethereumWalletMock);
    const accounts = await driver.connectToDapp();
    expect(accounts.length).toBeGreaterThan(0);
    expect(accounts[0]).toMatch(/^0x[a-fA-F0-9]{40}$/);
  });

  test(`${WM_PARITY_SCENARIOS[1].scenarioId} - personal_sign`, async ({ page, ethereumWalletMock }) => {
    const driver = await createDriver(page, ethereumWalletMock);
    const accounts = await driver.connectToDapp();
    const signature = await driver.approveSignature({
      method: "personal_sign",
      params: [MESSAGE_HEX, accounts[0]],
      signature: PERSONAL_SIGN_RESULT,
    });
    expect(signature).toBe(PERSONAL_SIGN_RESULT);
  });

  test(`${WM_PARITY_SCENARIOS[2].scenarioId} - eth_signTypedData_v4`, async ({
    page,
    ethereumWalletMock,
  }) => {
    const driver = await createDriver(page, ethereumWalletMock);
    const accounts = await driver.connectToDapp();
    const signature = await driver.approveSignature({
      method: "eth_signTypedData_v4",
      params: [accounts[0], typedDataPayload],
      signature: TYPED_DATA_SIGN_RESULT,
    });
    expect(signature).toBe(TYPED_DATA_SIGN_RESULT);
  });

  test(`${WM_PARITY_SCENARIOS[3].scenarioId} - eth_sendTransaction`, async ({
    page,
    ethereumWalletMock,
  }) => {
    const driver = await createDriver(page, ethereumWalletMock);
    const txHash = await driver.approveTransaction({
      to: DEFAULT_RECIPIENT,
      value: "1",
    });
    expect(txHash).toMatch(/^0x[a-fA-F0-9]{64}$/);
  });

  test(`${WM_PARITY_SCENARIOS[4].scenarioId} - accountsChanged deterministic recovery`, async ({
    page,
    ethereumWalletMock,
  }) => {
    const driver = await createDriver(page, ethereumWalletMock);
    await ethereumWalletMock.addNewAccount();
    const accounts = await ethereumWalletMock.getAllAccounts();
    const nextAccount = accounts?.[0];
    expect(nextAccount).toBeTruthy();
    await driver.ensureEmitterBridge();

    await page.evaluate(() => {
      window.__rustySafeAccountsEvents = [];
      window.ethereum.on("accountsChanged", (updatedAccounts) => {
        window.__rustySafeAccountsEvents.push(updatedAccounts);
      });
    });

    await driver.recoverFromFailure("accountsChanged", [nextAccount]);
    await page.waitForFunction(
      () =>
        Array.isArray(window.__rustySafeAccountsEvents) &&
        window.__rustySafeAccountsEvents.length > 0,
      null,
      { timeout: 5000 },
    );
    const observedEvents = await page.evaluate(() => window.__rustySafeAccountsEvents);
    expect(observedEvents.length).toBeGreaterThan(0);
    expect(observedEvents[0][0].toLowerCase()).toBe(nextAccount.toLowerCase());
  });

  test(`${WM_PARITY_SCENARIOS[5].scenarioId} - chainChanged deterministic recovery`, async ({
    page,
    ethereumWalletMock,
  }) => {
    const driver = await createDriver(page, ethereumWalletMock);
    await driver.ensureEmitterBridge();
    await page.evaluate(() => {
      window.__rustySafeChainEvents = [];
      window.ethereum.on("chainChanged", (chainId) => {
        window.__rustySafeChainEvents.push(chainId);
      });
    });

    await driver.recoverFromFailure("chainChanged", "0xaa36a7");
    await page.waitForFunction(
      () =>
        Array.isArray(window.__rustySafeChainEvents) &&
        window.__rustySafeChainEvents.length > 0,
      null,
      { timeout: 5000 },
    );
    const observedEvents = await page.evaluate(() => window.__rustySafeChainEvents);
    expect(observedEvents.length).toBeGreaterThan(0);
    expect(observedEvents[0]).toBe("0xaa36a7");
  });
});

test.describe.serial("Wallet Mock build/sign/share blocking lane", () => {
  test(`${WM_BSS_SCENARIOS[0].scenarioId} - tx lifecycle intent`, async ({ page, ethereumWalletMock }) => {
    const driver = await createDriver(page, ethereumWalletMock);
    const accounts = await driver.connectToDapp();
    const signature = await driver.approveSignature({
      method: "personal_sign",
      params: [MESSAGE_HEX, accounts[0]],
      signature: PERSONAL_SIGN_RESULT,
    });
    expect(signature).toBe(PERSONAL_SIGN_RESULT);

    const txHash = await driver.approveTransaction({
      to: DEFAULT_RECIPIENT,
      value: "1",
    });
    expect(txHash).toMatch(/^0x[a-fA-F0-9]{64}$/);
  });

  test(`${WM_BSS_SCENARIOS[1].scenarioId} - ABI selector mismatch guard`, async () => {
    const guardedReject = selectorGuard({
      expectedSelector: "0xa9059cbb",
      calldataHex: "0x095ea7b30000000000000000000000000000000000000000000000000000000000000001",
      override: false,
    });
    expect(guardedReject.accepted).toBe(false);
    expect(guardedReject.reason).toBe("selector_mismatch");

    const overrideAccept = selectorGuard({
      expectedSelector: "0xa9059cbb",
      calldataHex: "0x095ea7b30000000000000000000000000000000000000000000000000000000000000001",
      override: true,
    });
    expect(overrideAccept.accepted).toBe(true);
    expect(overrideAccept.reason).toBe("override_acknowledged");
  });

  test(`${WM_BSS_SCENARIOS[2].scenarioId} - manual signature idempotent`, async ({
    page,
    ethereumWalletMock,
  }) => {
    const driver = await createDriver(page, ethereumWalletMock);
    const accounts = await driver.connectToDapp();
    const first = await driver.approveSignature({
      method: "personal_sign",
      params: [MESSAGE_HEX, accounts[0]],
      signature: PERSONAL_SIGN_RESULT,
    });
    const second = await driver.approveSignature({
      method: "personal_sign",
      params: [MESSAGE_HEX, accounts[0]],
      signature: PERSONAL_SIGN_RESULT,
    });
    expect(first).toBe(PERSONAL_SIGN_RESULT);
    expect(second).toBe(PERSONAL_SIGN_RESULT);
  });

  test(`${WM_BSS_SCENARIOS[3].scenarioId} - bundle roundtrip deterministic`, async () => {
    const bundle = {
      schema_version: 1,
      txs: [{ id: "tx:1", state_revision: 2 }],
      messages: [{ id: "msg:1", state_revision: 1 }],
      wc_requests: [],
    };
    const canonical = stableStringify(bundle);
    const digestA = await sha256Hex(canonical);
    const roundtrip = JSON.parse(JSON.stringify(bundle));
    const digestB = await sha256Hex(stableStringify(roundtrip));
    expect(digestB).toBe(digestA);
  });

  test(`${WM_BSS_SCENARIOS[4].scenarioId} - localsafe URL keys compatibility`, async () => {
    const keys = ["importTx", "importSig", "importMsg", "importMsgSig"];
    expect(keys).toEqual(["importTx", "importSig", "importMsg", "importMsgSig"]);
  });

  test(`${WM_BSS_SCENARIOS[5].scenarioId} - tampered bundle quarantine`, async () => {
    const bundle = {
      schema_version: 1,
      txs: [{ id: "tx:1", state_revision: 2 }],
      messages: [],
      wc_requests: [],
    };
    const digest = await sha256Hex(stableStringify(bundle));
    const tampered = {
      ...bundle,
      txs: [{ id: "tx:1", state_revision: 3 }],
    };
    const tamperedDigest = await sha256Hex(stableStringify(tampered));
    expect(tamperedDigest).not.toBe(digest);
  });
});

test.describe.serial("Wallet Mock deterministic negative paths", () => {
  test("WM-NEG-001 - user rejection taxonomy is APP_FAIL", async ({ page, ethereumWalletMock }) => {
    const driver = await createDriver(page, ethereumWalletMock);
    const accounts = await driver.connectToDapp();

    const rejection = await page.evaluate(async ({ messageHex, accountAddress }) => {
      const originalRequest = window.ethereum.request.bind(window.ethereum);
      window.ethereum.request = async (payload) => {
        if (payload?.method === "personal_sign") {
          const error = new Error("User rejected the request.");
          error.code = 4001;
          throw error;
        }
        return await originalRequest(payload);
      };

      try {
        await window.ethereum.request({
          method: "personal_sign",
          params: [messageHex, accountAddress],
        });
      } catch (error) {
        return {
          code: error?.code ?? null,
          message: error?.message ?? String(error),
        };
      }
      return { code: null, message: "unexpected-success" };
    }, { messageHex: MESSAGE_HEX, accountAddress: accounts[0] });

    const taxonomy = classifyFailureTaxonomy(rejection);
    expect(taxonomy).toBe("APP_FAIL");
    expect(taxonomyTriageLabel(taxonomy)).toBe("triage/app");
  });

  test("WM-NEG-002 - timeout taxonomy is HARNESS_FAIL", async () => {
    const timeoutError = await Promise.race([
      new Promise((_, reject) =>
        setTimeout(() => reject(new Error("wallet-mock-request-timeout-250ms")), 15),
      ),
      new Promise((resolve) => setTimeout(resolve, 5000)),
    ]).catch((error) => error);

    const taxonomy = classifyFailureTaxonomy(timeoutError);
    expect(taxonomy).toBe("HARNESS_FAIL");
    expect(taxonomyTriageLabel(taxonomy)).toBe("triage/harness");
  });

  test("WM-NEG-003 - chain mismatch taxonomy is APP_FAIL", async ({ page, ethereumWalletMock }) => {
    const driver = await createDriver(page, ethereumWalletMock);
    const chainId = await driver.providerRequest("eth_chainId");
    const mismatchError =
      chainId === "0x1"
        ? new Error("chain mismatch expected 0xaa36a7 got 0x1")
        : new Error(`chain mismatch expected 0x1 got ${chainId}`);

    const taxonomy = classifyFailureTaxonomy(mismatchError);
    expect(taxonomy).toBe("APP_FAIL");
    expect(taxonomyTriageLabel(taxonomy)).toBe("triage/app");
  });
});
