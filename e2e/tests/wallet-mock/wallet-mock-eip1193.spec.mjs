import { expect } from "@playwright/test";
import { ethereumWalletMockFixtures as test } from "@synthetixio/ethereum-wallet-mock/playwright";
import { ethers } from "ethers";

import { WalletMockDriver } from "./drivers/wallet-mock-driver.mjs";
import { classifyFailureTaxonomy, taxonomyTriageLabel } from "./failure-taxonomy.mjs";
import { WM_BSS_SCENARIOS, WM_PARITY_SCENARIOS } from "./scenario-manifest.mjs";

const DEFAULT_RECIPIENT = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";
const DEFAULT_SAFE_ADDRESS = "0x000000000000000000000000000000000000BEEF";
const MESSAGE_HEX = "0x72757374792d736166652d707264303561";
const PERSONAL_SIGN_RESULT = `0x${"11".repeat(65)}`;
const TYPED_DATA_SIGN_RESULT = `0x${"22".repeat(65)}`;
const PRIVATE_KEY_A = `0x${"0".repeat(63)}1`;
const PRIVATE_KEY_B = `0x${"0".repeat(63)}2`;

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

let nonceCursor = 1000;
function nextNonce() {
  nonceCursor += 1;
  return nonceCursor;
}

function walletAddress(privateKey) {
  return new ethers.Wallet(privateKey).address;
}

function signDigest(digestHex, privateKey) {
  const signingKey = new ethers.utils.SigningKey(privateKey);
  return ethers.utils.joinSignature(signingKey.signDigest(digestHex));
}

function toBase64UrlJson(value) {
  return Buffer.from(JSON.stringify(value), "utf8").toString("base64url");
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
    await driver.openSigningTab({ surface: "Queue" });
    await driver.acquireWriterLock();

    const create = await driver.createRawTxDraft({
      chainId: 1,
      safeAddress: DEFAULT_SAFE_ADDRESS,
      nonce: nextNonce(),
      to: DEFAULT_RECIPIENT,
      value: "0",
      data: "0x",
      threshold: 2,
    });
    const txHash = create.safe_tx_hash ?? create.safeTxHash;
    expect(txHash).toMatch(/^0x[a-fA-F0-9]{64}$/);
    const flowId = create.flow_id ?? create.flowId;
    expect(flowId).toMatch(/^tx:0x[a-fA-F0-9]{64}$/);

    await driver.addTxSignature({
      safeTxHash: txHash,
      signer: walletAddress(PRIVATE_KEY_A),
      signature: signDigest(txHash, PRIVATE_KEY_A),
    });
    await driver.proposeTx({ safeTxHash: txHash });
    await driver.confirmTx({
      safeTxHash: txHash,
      signature: signDigest(txHash, PRIVATE_KEY_B),
    });
    const txAfterConfirm = await driver.loadTx({ safeTxHash: txHash });
    expect(txAfterConfirm).toBeTruthy();
    expect(Array.isArray(txAfterConfirm.signatures)).toBe(true);
    expect(txAfterConfirm.signatures.length).toBeGreaterThanOrEqual(2);
    expect(["Confirming", "ReadyToExecute", "Executed"]).toContain(String(txAfterConfirm.status));
    await driver.executeTx({ safeTxHash: txHash });

    const tx = await driver.waitForTxStatus(txHash, "Executed");
    expect(tx?.executed_tx_hash ?? tx?.executedTxHash).toMatch(/^0x[a-fA-F0-9]{64}$/);
    expect(String(tx?.status)).toBe("Executed");

    const transitions = await driver.loadTransitionLog({ flowId });
    expect(Array.isArray(transitions)).toBe(true);
    expect(transitions.some((row) => String(row.state_after).includes("Executed"))).toBe(true);

    const banner = await driver.readStatusBanner();
    expect((banner.last_info ?? banner.lastInfo) || (banner.last_error ?? banner.lastError)).toBeTruthy();
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

  test(`${WM_BSS_SCENARIOS[3].scenarioId} - bundle roundtrip deterministic`, async ({
    page,
    ethereumWalletMock,
  }) => {
    const driver = await createDriver(page, ethereumWalletMock);
    await driver.openSigningTab({ surface: "Queue" });
    await driver.acquireWriterLock();

    const create = await driver.createRawTxDraft({
      chainId: 1,
      safeAddress: DEFAULT_SAFE_ADDRESS,
      nonce: nextNonce(),
      to: DEFAULT_RECIPIENT,
      value: "0",
      data: "0x",
      threshold: 1,
    });
    const flowId = create.flow_id ?? create.flowId;
    const exported = await driver.triggerExportBundle({ flowIds: [flowId] });
    const bundle = exported.bundle;
    expect(bundle?.schema_version).toBe(1);
    expect(Array.isArray(bundle?.txs)).toBe(true);
    expect(bundle.txs.length).toBeGreaterThan(0);

    const imported = await driver.triggerImportBundle({ bundle });
    const merge = imported.merge ?? {};
    const mergeCount =
      (merge.tx_added ?? 0) +
      (merge.tx_updated ?? 0) +
      (merge.tx_skipped ?? 0) +
      (merge.tx_conflicted ?? 0);
    expect(mergeCount).toBeGreaterThan(0);

    const banner = await driver.readStatusBanner();
    expect((banner.last_info ?? banner.lastInfo) || (banner.last_error ?? banner.lastError)).toBeTruthy();
  });

  test(`${WM_BSS_SCENARIOS[4].scenarioId} - localsafe URL keys compatibility`, async ({
    page,
    ethereumWalletMock,
  }) => {
    const driver = await createDriver(page, ethereumWalletMock);
    await driver.openSigningTab({ surface: "ImportExport" });
    await driver.acquireWriterLock();

    const txHash = `0x${"bb".repeat(32)}`;
    const txPayload = {
      schema_version: 1,
      chain_id: 1,
      safe_address: DEFAULT_SAFE_ADDRESS,
      nonce: 1,
      payload: {
        to: DEFAULT_RECIPIENT,
        value: "0",
        data: "0x",
        threshold: 1,
      },
      build_source: "RawCalldata",
      abi_context: null,
      safe_tx_hash: txHash,
      signatures: [],
      status: "Draft",
      state_revision: 0,
      idempotency_key: "idem-url-import",
      created_at_ms: 1,
      updated_at_ms: 1,
      executed_tx_hash: null,
      mac_algorithm: "HmacSha256V1",
      mac_key_id: "mac-key-v1",
      integrity_mac: `0x${"00".repeat(32)}`,
    };
    const txImport = await driver.triggerImportUrlPayload({
      key: "importTx",
      payloadBase64url: toBase64UrlJson(txPayload),
    });
    expect((txImport.merge?.tx_added ?? 0) + (txImport.merge?.tx_updated ?? 0)).toBeGreaterThan(0);

    const txSigImport = await driver.triggerImportUrlPayload({
      key: "importSig",
      payloadBase64url: toBase64UrlJson({
        txHash,
        signature: {
          signer: walletAddress(PRIVATE_KEY_A),
          data: signDigest(txHash, PRIVATE_KEY_A),
        },
      }),
    });
    expect(txSigImport.merge?.tx_updated ?? 0).toBeGreaterThan(0);

    const messageHash = `0x${"cc".repeat(32)}`;
    const messageImport = await driver.triggerImportUrlPayload({
      key: "importMsg",
      payloadBase64url: toBase64UrlJson({
        schema_version: 1,
        chain_id: 1,
        safe_address: DEFAULT_SAFE_ADDRESS,
        method: "PersonalSign",
        payload: {
          message: "mock-url-message",
          threshold: 1,
        },
        message_hash: messageHash,
        signatures: [],
        status: "Draft",
        state_revision: 0,
        idempotency_key: "idem-url-message",
        created_at_ms: 1,
        updated_at_ms: 1,
        mac_algorithm: "HmacSha256V1",
        mac_key_id: "mac-key-v1",
        integrity_mac: `0x${"00".repeat(32)}`,
      }),
    });
    expect((messageImport.merge?.message_added ?? 0) + (messageImport.merge?.message_updated ?? 0)).toBeGreaterThan(0);

    const messageSigImport = await driver.triggerImportUrlPayload({
      key: "importMsgSig",
      payloadBase64url: toBase64UrlJson({
        messageHash,
        signature: {
          signer: walletAddress(PRIVATE_KEY_B),
          data: signDigest(`0x${"dd".repeat(32)}`, PRIVATE_KEY_B),
        },
      }),
    });
    expect(messageSigImport.merge?.message_updated ?? 0).toBeGreaterThan(0);

    const tx = await driver.loadTx({ safeTxHash: txHash });
    expect(tx?.signatures?.length ?? 0).toBeGreaterThan(0);
    const message = await driver.loadMessage({ messageHash });
    expect(message?.signatures?.length ?? 0).toBeGreaterThan(0);
  });

  test(`${WM_BSS_SCENARIOS[5].scenarioId} - tampered bundle quarantine`, async ({
    page,
    ethereumWalletMock,
  }) => {
    const driver = await createDriver(page, ethereumWalletMock);
    await driver.openSigningTab({ surface: "Queue" });
    await driver.acquireWriterLock();

    const create = await driver.createRawTxDraft({
      chainId: 1,
      safeAddress: DEFAULT_SAFE_ADDRESS,
      nonce: nextNonce(),
      to: DEFAULT_RECIPIENT,
      value: "0",
      data: "0x",
      threshold: 1,
    });
    const flowId = create.flow_id ?? create.flowId;
    const exported = await driver.triggerExportBundle({ flowIds: [flowId] });
    const bundle = exported.bundle;
    expect(bundle).toBeTruthy();

    const tampered = {
      ...bundle,
      bundle_digest: `0x${"00".repeat(32)}`,
    };
    await expect(driver.triggerImportBundle({ bundle: tampered })).rejects.toThrow(
      /bundle digest mismatch|bundle integrity mac mismatch/i,
    );
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
