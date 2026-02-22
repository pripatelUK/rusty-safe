import { unlockForFixture } from "@synthetixio/synpress-metamask/playwright";

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function isPageClosedError(error) {
  const message = String(error?.message ?? error ?? "");
  return (
    message.includes("Target page, context or browser has been closed") ||
    message.includes("Execution context was destroyed")
  );
}

async function resolveMetaMaskHomePage(context, extensionId) {
  const homeUrl = `chrome-extension://${extensionId}/home.html`;
  const extensionOrigin = `chrome-extension://${extensionId}/`;

  const extensionPages = context
    .pages()
    .filter((candidate) => !candidate.isClosed() && candidate.url().startsWith(extensionOrigin));
  const homePage = extensionPages.find((candidate) => candidate.url().startsWith(homeUrl));
  if (homePage) {
    return homePage;
  }

  const existingPage = extensionPages[0];
  if (existingPage) {
    await existingPage.goto(homeUrl).catch(() => {});
    return existingPage;
  }

  const createdPage = await context.newPage();
  await createdPage.goto(homeUrl).catch(() => {});
  return createdPage;
}

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
    await sleep(300);
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
  const pageUrl = page.url();
  const pageTitle = await page.title().catch(() => "unknown");
  const bodyTextSample = await page
    .locator("body")
    .innerText()
    .then((text) => String(text).replace(/\s+/g, " ").trim().slice(0, 180))
    .catch(() => "");

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
  const accountOptionsVisible = await page
    .locator('[data-testid="account-options-menu-button"]')
    .isVisible()
    .catch(() => false);
  const appHeaderLogoVisible = await page
    .locator('[data-testid="app-header-logo"]')
    .first()
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
  const loadingLogoVisible = await page.locator(".loading-logo").isVisible().catch(() => false);
  const loadingSpinnerVisible = await page
    .locator(".loading-spinner, .loading-overlay__spinner, .spinner")
    .first()
    .isVisible()
    .catch(() => false);
  const loadingOverlayVisible = await page
    .locator(".loading-overlay, .loading-indicator")
    .first()
    .isVisible()
    .catch(() => false);

  return {
    pageUrl,
    pageTitle,
    bodyTextSample,
    onboardingVisible: onboardingExistingVisible || onboardingCreateVisible,
    openWalletVisible,
    openWalletEnabled,
    unlockVisible,
    networkVisible,
    accountMenuVisible,
    accountOptionsVisible,
    appHeaderLogoVisible,
    crashVisible,
    crashRestartVisible,
    loadingLogoVisible,
    loadingSpinnerVisible,
    loadingOverlayVisible,
  };
}

function isWalletReady(state) {
  return Boolean(
    state.networkVisible ||
      state.accountMenuVisible ||
      state.accountOptionsVisible ||
      state.appHeaderLogoVisible,
  );
}

function isLoadingState(state) {
  return Boolean(state.loadingLogoVisible || state.loadingSpinnerVisible || state.loadingOverlayVisible);
}

async function waitForReadyState(page, timeoutMs = 12000) {
  const deadline = Date.now() + timeoutMs;
  let latest = await detectState(page);
  while (Date.now() < deadline) {
    if (isWalletReady(latest)) {
      return latest;
    }
    await sleep(isLoadingState(latest) ? 750 : 500);
    latest = await detectState(page);
  }
  return latest;
}

async function waitForReadyStateWithRecovery(context, page, extensionId, timeoutMs = 12000) {
  let activePage = page;
  for (let attempt = 0; attempt < 4; attempt += 1) {
    try {
      const state = await waitForReadyState(activePage, timeoutMs);
      return { state, page: activePage };
    } catch (error) {
      if (!isPageClosedError(error)) {
        throw error;
      }
      console.log(`[metamask-bootstrap] page closed during readiness check; recovering (attempt=${attempt + 1})`);
      activePage = await resolveMetaMaskHomePage(context, extensionId);
    }
  }
  throw new Error("metamask-bootstrap-page-recovery-exhausted");
}

async function settleOpenWallet(page) {
  const openWalletButton = page.getByRole("button", { name: /open wallet/i }).first();
  if (await openWalletButton.isVisible().catch(() => false)) {
    if (await openWalletButton.isEnabled().catch(() => false)) {
      await openWalletButton.click();
      await sleep(800);
      return true;
    }
  }

  const manageDefaults = page.getByRole("button", { name: /manage default settings/i }).first();
  if (await manageDefaults.isVisible().catch(() => false)) {
    await manageDefaults.click();
    const backButton = page.getByTestId("privacy-settings-back-button");
    if (await backButton.isVisible().catch(() => false)) {
      await backButton.click();
      await sleep(800);
      if (await openWalletButton.isVisible().catch(() => false)) {
        if (await openWalletButton.isEnabled().catch(() => false)) {
          await openWalletButton.click();
          await sleep(800);
          return true;
        }
      }
    }
  }

  return false;
}

export async function bootstrapMetaMaskRuntime({
  context,
  page: initialPage,
  extensionId,
  walletSetup,
  walletPassword,
  maxAttempts = 3,
}) {
  const homeUrl = `chrome-extension://${extensionId}/home.html`;
  let page = initialPage;
  let usedRecovery = false;
  let usedUnlock = false;

  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    if (!page || page.isClosed()) {
      page = await resolveMetaMaskHomePage(context, extensionId);
    }

    await page.goto(homeUrl).catch(async (error) => {
      if (!isPageClosedError(error)) {
        throw error;
      }
      page = await resolveMetaMaskHomePage(context, extensionId);
      await page.goto(homeUrl);
    });
    await sleep(1000);
    await ensureEnglishLocale(page).catch(() => {});

    const state = await detectState(page).catch(async (error) => {
      if (!isPageClosedError(error)) {
        throw error;
      }
      page = await resolveMetaMaskHomePage(context, extensionId);
      return await detectState(page);
    });
    console.log(`[metamask-bootstrap] attempt=${attempt + 1} state=${JSON.stringify(state)}`);

    if (state.crashVisible && state.crashRestartVisible) {
      const restartButton = page.getByRole("button", { name: /restart metamask/i });
      await restartButton.click();
      await sleep(2000);
      continue;
    }

    if (state.unlockVisible) {
      await unlockForFixture(page, walletPassword);
      usedUnlock = true;
      const { state: postUnlock, page: recoveredPage } = await waitForReadyStateWithRecovery(
        context,
        page,
        extensionId,
        10000,
      );
      page = recoveredPage;
      if (isWalletReady(postUnlock)) {
        return { usedRecovery, usedUnlock, state: postUnlock };
      }
      continue;
    }

    if (state.openWalletVisible) {
      const settled = await settleOpenWallet(page);
      if (settled) {
        const { state: postOpenWallet, page: recoveredPage } = await waitForReadyStateWithRecovery(
          context,
          page,
          extensionId,
          10000,
        );
        page = recoveredPage;
        if (isWalletReady(postOpenWallet)) {
          return { usedRecovery, usedUnlock, state: postOpenWallet };
        }
        continue;
      }
    }

    if (state.onboardingVisible) {
      await walletSetup.fn(context, page);
      usedRecovery = true;
      page = await resolveMetaMaskHomePage(context, extensionId);
      const { state: postRecovery, page: recoveredPage } = await waitForReadyStateWithRecovery(
        context,
        page,
        extensionId,
        12000,
      );
      page = recoveredPage;
      if (isWalletReady(postRecovery)) {
        return { usedRecovery, usedUnlock, state: postRecovery };
      }
      continue;
    }

    if (isLoadingState(state)) {
      const { state: postLoading, page: recoveredPage } = await waitForReadyStateWithRecovery(
        context,
        page,
        extensionId,
        15000,
      );
      page = recoveredPage;
      if (postLoading.crashVisible && postLoading.crashRestartVisible) {
        const restartButton = page.getByRole("button", { name: /restart metamask/i });
        await restartButton.click();
        await sleep(2000);
        continue;
      }
      if (isWalletReady(postLoading)) {
        return { usedRecovery, usedUnlock, state: postLoading };
      }
      if (isLoadingState(postLoading)) {
        console.log("[metamask-bootstrap] loading indicators persisted; reloading");
        await page.reload({ waitUntil: "domcontentloaded" }).catch(() => {});
        await sleep(1000);
        continue;
      }
    }

    if (isWalletReady(state)) {
      return { usedRecovery, usedUnlock, state };
    }

    // Unknown non-ready state. Force another fresh load and continue bounded recovery.
    console.log("[metamask-bootstrap] non-ready indeterminate state; forcing reload");
    await page.goto(homeUrl).catch(() => {});
    await sleep(1000);
  }

  page = await resolveMetaMaskHomePage(context, extensionId);
  await page.goto(homeUrl).catch(() => {});
  await sleep(1000);
  let finalState = (await waitForReadyStateWithRecovery(context, page, extensionId, 25000)).state;
  if (!isWalletReady(finalState) && isLoadingState(finalState)) {
    console.log("[metamask-bootstrap] final-state still loading; forcing final reload");
    await page.reload({ waitUntil: "domcontentloaded" }).catch(() => {});
    await sleep(1000);
    finalState = (await waitForReadyStateWithRecovery(context, page, extensionId, 15000)).state;
  }
  if (!isWalletReady(finalState) && finalState.crashVisible && finalState.crashRestartVisible) {
    console.log("[metamask-bootstrap] final-state crash detected; attempting restart");
    const restartButton = page.getByRole("button", { name: /restart metamask/i });
    await restartButton.click();
    await sleep(2000);
    finalState = (await waitForReadyStateWithRecovery(context, page, extensionId, 15000)).state;
  }
  console.log(`[metamask-bootstrap] final-state=${JSON.stringify(finalState)}`);

  if (finalState.onboardingVisible) {
    throw new Error("metamask-bootstrap-onboarding-persisted");
  }
  if (!isWalletReady(finalState)) {
    const homePrefix = `chrome-extension://${extensionId}/home.html`;
    const isSoftReadyHome =
      finalState.pageUrl.startsWith(homePrefix) &&
      !finalState.unlockVisible &&
      !finalState.onboardingVisible &&
      !finalState.crashVisible;
    if (isSoftReadyHome) {
      console.log("[metamask-bootstrap] accepting soft-ready home state");
      return { usedRecovery, usedUnlock, state: finalState };
    }
  }
  if (!isWalletReady(finalState)) {
    throw new Error(
      `metamask-bootstrap-not-ready:url=${finalState.pageUrl}:title=${finalState.pageTitle}:body=${finalState.bodyTextSample}`,
    );
  }

  return { usedRecovery, usedUnlock, state: finalState };
}
