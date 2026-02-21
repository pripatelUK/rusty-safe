import { unlockForFixture } from "@synthetixio/synpress-metamask/playwright";

async function ensureEnglishLocale(page) {
  const expectedPrefix = (process.env.PRD05A_EXPECTED_LOCALE_PREFIX ?? "en").toLowerCase();
  let observed = "unknown";
  for (let attempt = 0; attempt < 3; attempt += 1) {
    const locale = await page
      .evaluate(() => {
        return navigator.language ?? (navigator.languages?.[0] ?? "unknown");
      })
      .catch(() => "unknown");
    observed = String(locale).toLowerCase();
    if (observed !== "unknown") {
      break;
    }
    await page.waitForTimeout(300);
  }

  console.log(`[metamask-bootstrap] locale=${observed}`);
  if (observed === "unknown") {
    console.log("[metamask-bootstrap] locale could not be resolved; continuing with runtime profile gate as source of truth");
    return;
  }
  if (!observed.startsWith(expectedPrefix)) {
    throw new Error(`metamask-bootstrap-locale-mismatch:${observed}:expected-${expectedPrefix}`);
  }
}

async function detectState(page) {
  const onboardingExistingVisible = await page
    .getByRole("button", { name: /i have an existing wallet/i })
    .isVisible()
    .catch(() => false);
  const onboardingCreateVisible = await page
    .getByRole("button", { name: /create a new wallet/i })
    .isVisible()
    .catch(() => false);
  const openWalletButton = page.getByRole("button", { name: /open wallet/i }).first();
  const openWalletVisible = await openWalletButton.isVisible().catch(() => false);
  const openWalletEnabled = openWalletVisible
    ? await openWalletButton.isEnabled().catch(() => false)
    : false;
  const unlockVisible = await page.getByTestId("unlock-password").isVisible().catch(() => false);
  const networkVisible = await page
    .locator('[data-testid="network-display"]')
    .isVisible()
    .catch(() => false);
  const accountMenuVisible = await page
    .locator('[data-testid="account-menu-icon"]')
    .isVisible()
    .catch(() => false);
  const crashVisible = await page
    .getByRole("heading", { name: /metamask had trouble starting/i })
    .isVisible()
    .catch(() => false);
  const crashRestartVisible = await page
    .getByRole("button", { name: /restart metamask/i })
    .isVisible()
    .catch(() => false);

  return {
    onboardingVisible: onboardingExistingVisible || onboardingCreateVisible,
    openWalletVisible,
    openWalletEnabled,
    unlockVisible,
    networkVisible,
    accountMenuVisible,
    crashVisible,
    crashRestartVisible,
  };
}

async function settleOpenWallet(page) {
  const openWalletButton = page.getByRole("button", { name: /open wallet/i }).first();
  if (await openWalletButton.isVisible().catch(() => false)) {
    if (await openWalletButton.isEnabled().catch(() => false)) {
      await openWalletButton.click();
      await page.waitForTimeout(800);
      return true;
    }
  }

  const manageDefaults = page.getByRole("button", { name: /manage default settings/i }).first();
  if (await manageDefaults.isVisible().catch(() => false)) {
    await manageDefaults.click();
    const backButton = page.getByTestId("privacy-settings-back-button");
    if (await backButton.isVisible().catch(() => false)) {
      await backButton.click();
      await page.waitForTimeout(800);
      if (await openWalletButton.isVisible().catch(() => false)) {
        if (await openWalletButton.isEnabled().catch(() => false)) {
          await openWalletButton.click();
          await page.waitForTimeout(800);
          return true;
        }
      }
    }
  }

  return false;
}

export async function bootstrapMetaMaskRuntime({
  context,
  page,
  extensionId,
  walletSetup,
  walletPassword,
  maxAttempts = 3,
}) {
  const homeUrl = `chrome-extension://${extensionId}/home.html`;
  let usedRecovery = false;
  let usedUnlock = false;

  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    await page.goto(homeUrl);
    await page.waitForTimeout(1000);
    await ensureEnglishLocale(page);

    const state = await detectState(page);
    console.log(`[metamask-bootstrap] attempt=${attempt + 1} state=${JSON.stringify(state)}`);

    if (state.crashVisible && state.crashRestartVisible) {
      const restartButton = page.getByRole("button", { name: /restart metamask/i });
      await restartButton.click();
      await page.waitForTimeout(2000);
      continue;
    }

    if (state.unlockVisible) {
      await unlockForFixture(page, walletPassword);
      usedUnlock = true;
      continue;
    }

    if (state.openWalletVisible) {
      const settled = await settleOpenWallet(page);
      if (settled) {
        continue;
      }
    }

    if (state.onboardingVisible) {
      await walletSetup.fn(context, page);
      usedRecovery = true;
      continue;
    }

    if (state.networkVisible || state.accountMenuVisible || !state.onboardingVisible) {
      return { usedRecovery, usedUnlock, state };
    }
  }

  await page.goto(homeUrl);
  await page.waitForTimeout(1000);
  const finalState = await detectState(page);
  console.log(`[metamask-bootstrap] final-state=${JSON.stringify(finalState)}`);

  if (finalState.onboardingVisible) {
    throw new Error("metamask-bootstrap-onboarding-persisted");
  }

  return { usedRecovery, usedUnlock, state: finalState };
}
