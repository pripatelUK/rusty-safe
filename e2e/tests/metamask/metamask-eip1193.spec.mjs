import { expect } from "@playwright/test";
import { test } from "./metamask-patched-fixtures.mjs";

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

async function tryMetaMaskAction(label, action) {
  try {
    await action();
  } catch (error) {
    console.log(`[metamask-e2e] ${label} unavailable: ${String(error?.message ?? error)}`);
  }
}

test("MetaMask EIP-1193 smoke: connect + sign + typedData + sendTx", async ({
  page,
  metamask,
  connectToAnvil,
}) => {
  const hasProvider = await page.evaluate(() => typeof window.ethereum !== "undefined");
  expect(hasProvider).toBe(true);

  let accounts = await page.evaluate(async () => {
    return await window.ethereum.request({ method: "eth_accounts" });
  });
  if (accounts.length === 0) {
    const accountsPromise = page.evaluate(async () => {
      return await window.ethereum.request({ method: "eth_requestAccounts" });
    });
    await tryMetaMaskAction("connectToDapp", () => metamask.connectToDapp());
    accounts = await awaitWithTimeout(accountsPromise, 45000, "eth_requestAccounts");
  }
  expect(accounts.length).toBeGreaterThan(0);

  await connectToAnvil();

  const from = accounts[0];
  const chainIdHex = await page.evaluate(async () => {
    return await window.ethereum.request({
      method: "eth_chainId",
    });
  });
  const chainIdNumber = Number.parseInt(chainIdHex, 16);
  expect(chainIdNumber).toBeGreaterThan(0);

  const personalSignPromise = page.evaluate(
    async ({ fromAddress, messageHex }) =>
      await window.ethereum.request({
        method: "personal_sign",
        params: [messageHex, fromAddress],
      }),
    { fromAddress: from, messageHex: DEFAULT_MESSAGE_HEX },
  );
  await tryMetaMaskAction("confirmSignature(personal_sign)", () => metamask.confirmSignature());
  const personalSignature = await awaitWithTimeout(personalSignPromise, 45000, "personal_sign");
  expect(personalSignature).toMatch(/^0x[0-9a-fA-F]{130}$/);

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
  await tryMetaMaskAction("confirmSignature(typedData)", () => metamask.confirmSignature());
  const typedDataSignature = await awaitWithTimeout(
    typedDataPromise,
    45000,
    "eth_signTypedData_v4",
  );
  expect(typedDataSignature).toMatch(/^0x[0-9a-fA-F]{130}$/);

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
  await tryMetaMaskAction("confirmTransaction", () => metamask.confirmTransaction());
  const txHash = await awaitWithTimeout(sendTxPromise, 60000, "eth_sendTransaction");
  expect(txHash).toMatch(/^0x[0-9a-fA-F]{64}$/);
});
