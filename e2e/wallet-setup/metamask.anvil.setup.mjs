import { defineWalletSetup } from "@synthetixio/synpress";
import { expect } from "@playwright/test";

const DEFAULT_TEST_MNEMONIC =
  "test test test test test test test test test test test junk";
const walletPassword = process.env.PRD05A_METAMASK_PASSWORD ?? "Prd05aMetaMask!123";
const seedPhrase = process.env.PRD05A_METAMASK_SEED ?? DEFAULT_TEST_MNEMONIC;

async function clickIfVisible(locator) {
  if (await locator.isVisible().catch(() => false)) {
    await locator.click();
    return true;
  }
  return false;
}

async function selectTwelveWordsIfVisible(walletPage) {
  const wordCountDropdown = walletPage.locator(
    ".import-srp__number-of-words-dropdown > .dropdown__select",
  );
  if (!(await wordCountDropdown.isVisible().catch(() => false))) {
    return;
  }

  await wordCountDropdown.click();

  const byRole = walletPage.getByRole("option", { name: /^12/ }).first();
  if (await byRole.isVisible().catch(() => false)) {
    await byRole.click();
    return;
  }

  const byDropdownItem = walletPage.locator(".dropdown__item").filter({ hasText: /^12/ }).first();
  if (await byDropdownItem.isVisible().catch(() => false)) {
    await byDropdownItem.click();
    return;
  }

  const byText = walletPage.getByText(/^12(\s*words?)?$/i).first();
  if (await byText.isVisible().catch(() => false)) {
    await byText.click();
  }
}

async function fillSrpByWords(walletPage, words) {
  for (const [index, word] of words.entries()) {
    const input = walletPage.getByTestId(`import-srp__srp-word-${index}`);
    await input.click();
    await input.fill(word);
  }
}

async function fillSrp(walletPage, seed, words) {
  await selectTwelveWordsIfVisible(walletPage);

  const srpTextarea = walletPage.getByTestId("srp-input-import__srp-note");
  if (await srpTextarea.isVisible().catch(() => false)) {
    await srpTextarea.click();
    await srpTextarea.fill("");
    await srpTextarea.type(seed, { delay: 10 });
    return "textarea";
  }

  await fillSrpByWords(walletPage, words);
  return "word-inputs";
}

async function runSetup(walletPage) {
  const words = seedPhrase.trim().split(/\s+/);
  if (words.length !== 12) {
    throw new Error(`Expected 12-word mnemonic for setup, got ${words.length}`);
  }

  const termsCheckbox = walletPage.getByTestId("onboarding-terms-checkbox");
  if (await termsCheckbox.isVisible().catch(() => false)) {
    const checked = await termsCheckbox.isChecked().catch(() => false);
    if (!checked) {
      await termsCheckbox.click();
    }
  }

  if (!(await clickIfVisible(walletPage.getByTestId("onboarding-import-wallet")))) {
    await walletPage
      .getByRole("button", { name: /existing wallet|i have an existing wallet/i })
      .click();
  }
  await walletPage.getByTestId("onboarding-import-with-srp-button").click();

  const fillMode = await fillSrp(walletPage, seedPhrase, words);
  console.log(`[metamask-setup] srp-fill-mode=${fillMode}`);

  const confirmImport = walletPage.getByTestId("import-srp-confirm");
  if (!(await confirmImport.isEnabled().catch(() => false))) {
    console.log("[metamask-setup] import continue disabled after first SRP fill; retrying fallback");
    if (fillMode === "textarea") {
      await fillSrpByWords(walletPage, words);
    } else {
      const srpTextarea = walletPage.getByTestId("srp-input-import__srp-note");
      if (await srpTextarea.isVisible().catch(() => false)) {
        await srpTextarea.fill("");
        await srpTextarea.type(seedPhrase, { delay: 10 });
      }
    }
  }

  await expect(confirmImport).toBeEnabled({ timeout: 45000 });
  await confirmImport.click();

  await walletPage.getByTestId("create-password-new-input").fill(walletPassword);
  await walletPage.getByTestId("create-password-confirm-input").fill(walletPassword);
  const acceptPasswordTerms = walletPage.getByTestId("create-password-terms");
  const passwordTermsChecked = await acceptPasswordTerms.isChecked().catch(() => false);
  if (!passwordTermsChecked) {
    await acceptPasswordTerms.click();
  }
  await walletPage.getByTestId("create-password-submit").click();

  const metametricsToggle = walletPage.locator("#metametrics-opt-in");
  if (await metametricsToggle.isVisible().catch(() => false)) {
    await metametricsToggle.click();
  }
  if (!(await clickIfVisible(walletPage.getByTestId("metametrics-i-agree")))) {
    await walletPage.getByRole("button", { name: /i agree|confirm|continue/i }).click();
  }

  // This screen often appears after analytics and must be closed for cached state to be reusable.
  const openWalletButton = walletPage.getByRole("button", { name: /open wallet/i }).first();
  for (let attempt = 0; attempt < 3; attempt += 1) {
    const readyScreenVisible = await openWalletButton
      .waitFor({ state: "visible", timeout: 5000 })
      .then(() => true)
      .catch(() => false);
    if (!readyScreenVisible) {
      console.log("[metamask-setup] ready-screen=not-visible");
      break;
    }

    const openWalletEnabled = await openWalletButton.isEnabled().catch(() => false);
    console.log(`[metamask-setup] ready-screen=visible enabled=${openWalletEnabled}`);
    if (openWalletEnabled) {
      await openWalletButton.click();
      await walletPage.waitForTimeout(400);
      continue;
    }

    const enabledEventually = await openWalletButton
      .waitFor({ state: "attached", timeout: 120000 })
      .then(async () => {
        const deadline = Date.now() + 120000;
        while (Date.now() < deadline) {
          if (await openWalletButton.isEnabled().catch(() => false)) {
            return true;
          }
          await walletPage.waitForTimeout(1000);
        }
        return false;
      })
      .catch(() => false);
    if (enabledEventually) {
      console.log("[metamask-setup] open-wallet became enabled");
      await openWalletButton.click();
      await walletPage.waitForTimeout(400);
      continue;
    }

    // Some builds keep this button disabled. Navigate directly to extension home as a fallback.
    const extensionId = walletPage.url().match(/^chrome-extension:\/\/([^/]+)\//)?.[1];
    if (extensionId) {
      console.log("[metamask-setup] open-wallet remained disabled; using home.html fallback");
      await walletPage.goto(`chrome-extension://${extensionId}/home.html`);
      await walletPage.waitForLoadState("domcontentloaded");
      break;
    }

    await walletPage.waitForTimeout(400);
  }

  const isOnboardingStillVisible = await walletPage
    .getByRole("button", { name: /i have an existing wallet/i })
    .isVisible()
    .catch(() => false);
  console.log(`[metamask-setup] post-setup-onboarding-visible=${isOnboardingStillVisible}`);
}

export default defineWalletSetup(walletPassword, async (_context, walletPage) => {
  await runSetup(walletPage);
});
