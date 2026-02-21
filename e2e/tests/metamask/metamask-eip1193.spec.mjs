import { expect } from "@playwright/test";
import { test } from "./metamask-patched-fixtures.mjs";
import { MM_PARITY_SCENARIOS } from "./scenario-manifest.mjs";

const DEFAULT_RECIPIENT = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";
const DEFAULT_MESSAGE_HEX = "0x72757374792d736166652d707264303561";

function toHexChainId(chainId) {
  return `0x${chainId.toString(16)}`;
}

async function awaitWithTimeout(promise, timeoutMs, label) {
  return await Promise.race([
    promise,
    new Promise((_, reject) =>
      setTimeout(() => reject(new Error(`${label}-timeout-${timeoutMs}ms`)), timeoutMs),
    ),
  ]);
}

async function tryWalletAction(label, action) {
  try {
    await action();
  } catch (error) {
    console.log(`[metamask-e2e] ${label} unavailable: ${String(error?.message ?? error)}`);
  }
}

async function ensureProvider(page) {
  const hasProvider = await page.evaluate(() => typeof window.ethereum !== "undefined");
  expect(hasProvider).toBe(true);
}

async function ensureConnectedAccount(page, walletDriver) {
  let accounts = await page.evaluate(async () => {
    return await window.ethereum.request({ method: "eth_accounts" });
  });
  if (accounts.length === 0) {
    const accountsPromise = page.evaluate(async () => {
      return await window.ethereum.request({ method: "eth_requestAccounts" });
    });
    await tryWalletAction("connectToDapp", () => walletDriver.connectToDapp());
    accounts = await awaitWithTimeout(accountsPromise, 45000, "eth_requestAccounts");
  }
  expect(accounts.length).toBeGreaterThan(0);
  return accounts[0];
}

for (const scenario of MM_PARITY_SCENARIOS) {
  test(`${scenario.scenarioId}: ${scenario.title}`, async ({ page, walletDriver, connectToAnvil }) => {
    console.log(
      `[metamask-e2e] scenario=${scenario.scenarioId} parity=${scenario.parityIds.join(",")}`,
    );

    await ensureProvider(page);

    if (scenario.method === "eth_requestAccounts") {
      const requestPromise = page.evaluate(async () => {
        return await window.ethereum.request({ method: "eth_requestAccounts" });
      });
      await tryWalletAction("connectToDapp", () => walletDriver.connectToDapp());
      const accounts = await awaitWithTimeout(requestPromise, scenario.timeoutMs, scenario.method);
      expect(accounts.length).toBeGreaterThan(0);
      return;
    }

    const from = await ensureConnectedAccount(page, walletDriver);

    const chainIdHex = await page.evaluate(async () => {
      return await window.ethereum.request({
        method: "eth_chainId",
      });
    });
    const chainIdNumber = Number.parseInt(chainIdHex, 16);
    expect(chainIdNumber).toBeGreaterThan(0);

    if (scenario.method === "personal_sign") {
      const personalSignPromise = page.evaluate(
        async ({ fromAddress, messageHex }) =>
          await window.ethereum.request({
            method: "personal_sign",
            params: [messageHex, fromAddress],
          }),
        { fromAddress: from, messageHex: DEFAULT_MESSAGE_HEX },
      );
      await tryWalletAction("approveSignature(personal_sign)", () => walletDriver.approveSignature());
      const personalSignature = await awaitWithTimeout(
        personalSignPromise,
        scenario.timeoutMs,
        scenario.method,
      );
      expect(personalSignature).toMatch(/^0x[0-9a-fA-F]{130}$/);
      return;
    }

    if (scenario.method === "eth_signTypedData_v4") {
      const typedDataV4 = JSON.stringify({
        domain: {
          name: "RustySafe PRD05A",
          version: "1",
          chainId: chainIdNumber,
          verifyingContract: DEFAULT_RECIPIENT,
        },
        message: {
          contents: "MetaMask typed data parity gate",
          from,
        },
        primaryType: "Mail",
        types: {
          EIP712Domain: [
            { name: "name", type: "string" },
            { name: "version", type: "string" },
            { name: "chainId", type: "uint256" },
            { name: "verifyingContract", type: "address" },
          ],
          Mail: [
            { name: "contents", type: "string" },
            { name: "from", type: "address" },
          ],
        },
      });

      const typedDataPromise = page.evaluate(
        async ({ fromAddress, typedDataJson }) =>
          await window.ethereum.request({
            method: "eth_signTypedData_v4",
            params: [fromAddress, typedDataJson],
          }),
        { fromAddress: from, typedDataJson: typedDataV4 },
      );
      await tryWalletAction("approveSignature(typedData)", () => walletDriver.approveSignature());
      const typedDataSignature = await awaitWithTimeout(
        typedDataPromise,
        scenario.timeoutMs,
        scenario.method,
      );
      expect(typedDataSignature).toMatch(/^0x[0-9a-fA-F]{130}$/);
      return;
    }

    if (scenario.method === "eth_sendTransaction") {
      if (scenario.requiresAnvil) {
        await connectToAnvil();
      }

      const sendTxPromise = page.evaluate(
        async ({ fromAddress, toAddress, chainIdValue }) =>
          await window.ethereum.request({
            method: "eth_sendTransaction",
            params: [
              {
                from: fromAddress,
                to: toAddress,
                value: "0x16345785d8a0000",
                chainId: chainIdValue,
              },
            ],
          }),
        {
          fromAddress: from,
          toAddress: process.env.PRD05A_METAMASK_RECIPIENT ?? DEFAULT_RECIPIENT,
          chainIdValue: toHexChainId(chainIdNumber),
        },
      );
      await tryWalletAction("approveTransaction", () => walletDriver.approveTransaction());
      const txHash = await awaitWithTimeout(sendTxPromise, scenario.timeoutMs, scenario.method);
      expect(txHash).toMatch(/^0x[0-9a-fA-F]{64}$/);
      return;
    }

    throw new Error(`unsupported-scenario-method:${scenario.method}`);
  });
}
