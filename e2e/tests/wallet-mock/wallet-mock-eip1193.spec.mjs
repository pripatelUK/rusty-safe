import { expect } from "@playwright/test";
import { ethereumWalletMockFixtures as test } from "@synthetixio/ethereum-wallet-mock/playwright";

import { WalletMockDriver } from "./drivers/wallet-mock-driver.mjs";
import { classifyFailureTaxonomy, taxonomyTriageLabel } from "./failure-taxonomy.mjs";
import { WM_PARITY_SCENARIOS } from "./scenario-manifest.mjs";

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
    await driver.ensureEmitterBridge();
    await ethereumWalletMock.addNewAccount();
    const accounts = await ethereumWalletMock.getAllAccounts();
    const nextAccount = accounts?.[0];
    expect(nextAccount).toBeTruthy();

    await page.evaluate(() => {
      window.__rustySafeAccountsEvents = [];
      window.ethereum.on("accountsChanged", (updatedAccounts) => {
        window.__rustySafeAccountsEvents.push(updatedAccounts);
      });
    });

    await driver.recoverFromFailure("accountsChanged", [nextAccount]);
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
    const observedEvents = await page.evaluate(() => window.__rustySafeChainEvents);
    expect(observedEvents.length).toBeGreaterThan(0);
    expect(observedEvents[0]).toBe("0xaa36a7");
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
