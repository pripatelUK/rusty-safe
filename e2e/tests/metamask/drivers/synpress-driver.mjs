import { assertWalletDriverContract } from "./wallet-driver.mjs";

async function tryAction(label, action) {
  try {
    await action();
    return true;
  } catch (error) {
    console.log(`[synpress-driver] ${label} unavailable: ${String(error?.message ?? error)}`);
    return false;
  }
}

export class SynpressDriver {
  constructor(metamask) {
    this.name = "synpress";
    this.releaseGateEligible = true;
    this._metamask = metamask;
    assertWalletDriverContract(this, "synpress-driver");
  }

  async bootstrapWallet() {
    // Bootstrap is handled in fixture setup via bootstrapMetaMaskRuntime.
    return { supported: true, delegatedTo: "fixture-bootstrap" };
  }

  _extensionPages(context, extensionId) {
    const prefix = `chrome-extension://${extensionId}/`;
    return context.pages().filter((page) => !page.isClosed() && page.url().startsWith(prefix));
  }

  _candidatePages(context, extensionId) {
    const extensionPrefix = `chrome-extension://${extensionId}/`;
    return context
      .pages()
      .filter((page) => !page.isClosed())
      .filter((page) => {
        const url = page.url();
        if (url.startsWith(extensionPrefix)) {
          return true;
        }
        if (url.startsWith("http://") || url.startsWith("https://")) {
          return true;
        }
        return false;
      });
  }

  async _ensureExtensionHomePage(context, extensionId) {
    const extensionPrefix = `chrome-extension://${extensionId}/`;
    const homePrefix = `${extensionPrefix}home.html`;
    const homeRoot = `${homePrefix}#`;
    const existingHome = context
      .pages()
      .find((page) => !page.isClosed() && page.url().startsWith(homePrefix));
    if (existingHome) {
      const url = existingHome.url();
      const shouldNormalizeRoute =
        url.includes("#notifications/") ||
        url.includes("#account-list") ||
        url.includes("#unlock") ||
        url.includes("#onboarding/");
      if (shouldNormalizeRoute) {
        await existingHome.goto(homeRoot, { waitUntil: "domcontentloaded" }).catch(() => {});
      }
      return existingHome;
    }
    const page = await context.newPage();
    await page.goto(homeRoot, { waitUntil: "domcontentloaded" }).catch(() => {});
    return page;
  }

  async _recoverExtensionRuntime(label) {
    const context = this._metamask.context;
    const extensionId = this._metamask.extensionId;
    if (!extensionId) {
      return false;
    }
    const page = await this._ensureExtensionHomePage(context, extensionId).catch(() => null);
    if (!page) {
      return false;
    }

    await page.bringToFront().catch(() => {});
    const bodyText = await page
      .locator("body")
      .innerText()
      .then((text) => String(text).replace(/\s+/g, " ").trim())
      .catch(() => "");
    if (!bodyText) {
      await page.reload({ waitUntil: "domcontentloaded" }).catch(() => {});
      await page.waitForTimeout(400).catch(() => {});
      const afterReloadText = await page
        .locator("body")
        .innerText()
        .then((text) => String(text).replace(/\s+/g, " ").trim())
        .catch(() => "");
      if (!afterReloadText) {
        const homeRoot = `chrome-extension://${extensionId}/home.html#`;
        await page.goto(homeRoot, { waitUntil: "domcontentloaded" }).catch(() => {});
        await page.waitForTimeout(400).catch(() => {});
      }
    }

    const crashed = await page
      .getByRole("heading", { name: /metamask had trouble starting/i })
      .isVisible()
      .catch(() => false);
    if (crashed) {
      const restartButton = page.getByRole("button", { name: /restart metamask/i }).first();
      await this._clickLocator(restartButton, `${label}:restart-metamask`).catch(() => {});
      await page.waitForTimeout(1500).catch(() => {});
    }

    const unlockPassword =
      this._metamask.password ?? process.env.PRD05A_METAMASK_PASSWORD ?? "Prd05aMetaMask!123";
    const unlockRoute = page.url().includes("#unlock");
    const unlockInput = page.getByTestId("unlock-password").first();
    const unlockInputVisible = await unlockInput.isVisible().catch(() => false);
    const unlockInputFallback = page.getByPlaceholder(/enter your password/i).first();
    const unlockInputFallbackVisible = await unlockInputFallback.isVisible().catch(() => false);
    if (unlockRoute || unlockInputVisible || unlockInputFallbackVisible) {
      try {
        if (unlockInputVisible) {
          await unlockInput.fill(unlockPassword);
        } else {
          await unlockInputFallback.fill(unlockPassword);
        }
        const unlockSubmit = page.getByTestId("unlock-submit").first();
        if (await unlockSubmit.isVisible().catch(() => false)) {
          await unlockSubmit.click();
        } else {
          await page.getByRole("button", { name: /unlock/i }).first().click();
        }
      } catch (error) {
        console.log(`[synpress-driver] ${label} unlock failed: ${String(error?.message ?? error)}`);
      }
      await page.waitForTimeout(1000).catch(() => {});
    }

    const onboardingVisible = page.url().includes("#onboarding/");
    if (onboardingVisible) {
      console.log(`[synpress-driver] ${label} onboarding-visible-after-recovery url=${page.url()}`);
      return false;
    }

    return true;
  }

  async _ensureNotificationPage(context, extensionId) {
    const notificationPrefix = `chrome-extension://${extensionId}/notification.html`;
    const existing = context
      .pages()
      .find((page) => !page.isClosed() && page.url().startsWith(notificationPrefix));
    if (existing) {
      return existing;
    }
    const created = await context.newPage();
    await created.goto(notificationPrefix, { waitUntil: "domcontentloaded" }).catch(() => {});
    return created;
  }

  async _describePage(page) {
    const url = page.url();
    const body = await page
      .locator("body")
      .innerText()
      .then((text) => String(text).replace(/\s+/g, " ").trim().slice(0, 220))
      .catch(() => "");
    const buttons = await page
      .evaluate(() => {
        return Array.from(document.querySelectorAll("button"))
          .map((button) => (button.textContent ?? "").replace(/\s+/g, " ").trim())
          .filter((text) => text.length > 0)
          .slice(0, 8);
      })
      .catch(() => []);
    const interestingTestIds = [
      "account-options-menu-button",
      "account-menu-icon",
      "notifications-tag-counter__unread-dot",
      "global-menu-notification-count",
      "notifications-menu-item",
      "notifications-list",
      "page-container-footer-next",
      "confirm-footer-button",
      "confirm-btn",
      "request-confirm-button",
      "confirmation-submit-button",
      "connect-page",
    ];
    const visibleTestIds = [];
    for (const testId of interestingTestIds) {
      const visible = await page.getByTestId(testId).first().isVisible().catch(() => false);
      if (visible) {
        visibleTestIds.push(testId);
      }
    }
    const indexedTestIds = await page
      .evaluate(() => {
        return Array.from(document.querySelectorAll("[data-testid]"))
          .map((element) => element.getAttribute("data-testid"))
          .filter((value) => typeof value === "string" && value.length > 0)
          .slice(0, 80);
      })
      .catch(() => []);
    return { url, body, buttons, visibleTestIds, indexedTestIds };
  }

  async _clickLocator(locator, label, options = {}) {
    const { allowForce = true } = options;
    const canClick =
      (await locator.isVisible().catch(() => false)) && (await locator.isEnabled().catch(() => false));
    if (!canClick) {
      return false;
    }

    try {
      await locator.click({ timeout: 1500 });
      return true;
    } catch (error) {
      const message = String(error?.message ?? error);
      const canForce =
        allowForce &&
        (message.includes("subtree intercepts pointer events") ||
          message.includes("element receives pointer events") ||
          message.includes("intercepts pointer events") ||
          message.includes("would receive the click"));
      if (canForce) {
        try {
          await locator.click({ force: true, timeout: 1500 });
          return true;
        } catch (forceError) {
          console.log(
            `[synpress-driver] ${label} force-click failed: ${String(forceError?.message ?? forceError)}`,
          );
        }
      } else {
        console.log(`[synpress-driver] ${label} click failed: ${message}`);
      }
    }

    try {
      const handle = await locator.elementHandle();
      if (!handle) {
        return false;
      }
      await handle.evaluate((element) => {
        element.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true, composed: true }));
        element.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, composed: true }));
        element.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, composed: true }));
        element.dispatchEvent(new MouseEvent("click", { bubbles: true, composed: true }));
        if (typeof element.click === "function") {
          element.click();
        }
      });
      return true;
    } catch (evaluateError) {
      console.log(
        `[synpress-driver] ${label} eval-click failed: ${String(evaluateError?.message ?? evaluateError)}`,
      );
      return false;
    }
  }

  async _clickApprovalControls(page) {
    let clicked = 0;
    const pageUrl = page.url();
    if (pageUrl.includes("#account-list")) {
      await page.keyboard.press("Escape").catch(() => {});
      await page.waitForTimeout(200).catch(() => {});
    }
    if (pageUrl.includes("#notifications/")) {
      const notificationBack = page.getByTestId("notification-details-back-button").first();
      if (await this._clickLocator(notificationBack, "notification-details-back")) {
        clicked += 1;
        await page.waitForTimeout(250).catch(() => {});
      }
      const walletInitiatedBack = page.getByTestId("wallet-initiated-header-back-button").first();
      if (await this._clickLocator(walletInitiatedBack, "wallet-initiated-back")) {
        clicked += 1;
        await page.waitForTimeout(250).catch(() => {});
      }
    }

    const crashed = await page
      .getByRole("heading", { name: /metamask had trouble starting/i })
      .isVisible()
      .catch(() => false);
    if (crashed) {
      const restartButton = page.getByRole("button", { name: /restart metamask/i }).first();
      if (await this._clickLocator(restartButton, "restart-metamask")) {
        await page.waitForTimeout(1000).catch(() => {});
        clicked += 1;
      }
    }

    const testIds = [
      "notifications-menu-item",
      "global-menu-notification-count",
      "notifications-page",
      "notifications-list",
      "notification-details-back-button",
      "wallet-initiated-header-back-button",
      "page-container-footer-next",
      "confirmation-submit-button",
      "confirm-footer-button",
      "confirm-btn",
      "confirm-button",
      "request-confirm-button",
      "allow-authorize-button",
      "connect-more-accounts-button",
      "connect-more-accounts",
      "notifications-tag-counter__unread-dot",
      "connect-page",
      "confirmation_request-section",
    ];
    for (const testId of testIds) {
      const locator = page.getByTestId(testId).first();
      if (await this._clickLocator(locator, `testid:${testId}`)) {
        clicked += 1;
        await page.waitForTimeout(250).catch(() => {});
      }
    }

    const notificationRows = page.locator(
      '[data-testid="notifications-list"] button, [data-testid="notifications-list"] [role="button"], [data-testid="notifications-list"] a',
    );
    const notificationCount = await notificationRows.count().catch(() => 0);
    if (notificationCount > 0) {
      const rowClicked = await this._clickLocator(
        notificationRows.first(),
        "notifications-list:first-actionable",
      );
      if (rowClicked) {
        clicked += 1;
        await page.waitForTimeout(250).catch(() => {});
      }
    }

    const genericButton = page
      .getByRole("button", { name: /connect|next|approve|confirm|sign|submit|continue|ok/i })
      .first();
    if (await this._clickLocator(genericButton, "generic-action-button")) {
      clicked += 1;
      await page.waitForTimeout(250).catch(() => {});
    }

    const notificationsButton = page
      .getByRole("button", { name: /notification|request|pending/i })
      .first();
    const canClickNotifications =
      (await notificationsButton.isVisible().catch(() => false)) &&
      (await notificationsButton.isEnabled().catch(() => false));
    if (canClickNotifications) {
      await this._clickLocator(notificationsButton, "generic-notifications-button");
      clicked += 1;
      await page.waitForTimeout(250).catch(() => {});
    }
    return clicked;
  }

  async _approveFromExtensionSurfaces(label) {
    const context = this._metamask.context;
    const extensionId = this._metamask.extensionId;
    if (!extensionId) {
      console.log(`[synpress-driver] ${label} fallback skipped: missing-extension-id`);
      return false;
    }

    let totalClicks = 0;

    for (let attempt = 0; attempt < 20; attempt += 1) {
      const homePage = await this._ensureExtensionHomePage(context, extensionId).catch(() => null);
      if (homePage) {
        if (attempt % 3 === 1) {
          const notificationsUrl = `chrome-extension://${extensionId}/home.html#notifications`;
          await homePage.goto(notificationsUrl, { waitUntil: "domcontentloaded" }).catch(() => {});
        }
        await homePage.bringToFront().catch(() => {});
        totalClicks += await this._clickApprovalControls(homePage);
      }
      if (attempt === 0 || attempt % 4 === 3) {
        const notificationPage = await this._ensureNotificationPage(context, extensionId).catch(() => null);
        if (notificationPage) {
          const notificationBody = await notificationPage
            .locator("body")
            .innerText()
            .then((text) => String(text).replace(/\s+/g, " ").trim())
            .catch(() => "");
          if (!notificationBody) {
            await notificationPage.reload({ waitUntil: "domcontentloaded" }).catch(() => {});
            await notificationPage.waitForTimeout(200).catch(() => {});
          }
          await notificationPage.bringToFront().catch(() => {});
          totalClicks += await this._clickApprovalControls(notificationPage);
        }
      }

      const pages = this._candidatePages(context, extensionId);
      for (const page of pages) {
        await page.bringToFront().catch(() => {});
        totalClicks += await this._clickApprovalControls(page);
      }

      if (totalClicks > 0 && attempt >= 2) {
        break;
      }
      await context.waitForEvent("page", { timeout: 700 }).catch(() => {});
    }

    const snapshots = [];
    for (const page of this._candidatePages(context, extensionId)) {
      snapshots.push(await this._describePage(page));
    }
    console.log(`[synpress-driver] ${label} extension-snapshots=${JSON.stringify(snapshots)}`);
    console.log(`[synpress-driver] ${label} extension-approval-clicks=${totalClicks}`);
    return totalClicks > 0;
  }

  async connectToDapp() {
    const initialFallback = await Promise.race([
      this._approveFromExtensionSurfaces("connectToDapp-pre"),
      new Promise((resolve) => setTimeout(() => resolve("timeout"), 8000)),
    ]);
    if (initialFallback === true) {
      return true;
    }

    try {
      await Promise.race([
        this._metamask.connectToDapp(),
        new Promise((_, reject) => setTimeout(() => reject(new Error("synpress-connect-timeout-6000ms")), 6000)),
      ]);
      return true;
    } catch (error) {
      console.log(`[synpress-driver] connectToDapp unavailable: ${String(error?.message ?? error)}`);
      const pageUrls = this._metamask.context
        .pages()
        .filter((page) => !page.isClosed())
        .map((page) => page.url());
      console.log(`[synpress-driver] connectToDapp context-pages=${JSON.stringify(pageUrls)}`);
      const fallbackPromise = this._approveFromExtensionSurfaces("connectToDapp");
      const fallbackResult = await Promise.race([
        fallbackPromise,
        new Promise((resolve) => setTimeout(() => resolve("timeout"), 12000)),
      ]);
      if (fallbackResult !== true) {
        const recovered = await this._recoverExtensionRuntime("connectToDapp").catch(() => false);
        if (recovered) {
          const retryResult = await Promise.race([
            this._approveFromExtensionSurfaces("connectToDapp-retry"),
            new Promise((resolve) => setTimeout(() => resolve("timeout"), 8000)),
          ]);
          if (retryResult === true) {
            return true;
          }
        }
      }
      if (fallbackResult === "timeout") {
        console.log("[synpress-driver] connectToDapp fallback timed out after 12000ms");
        return false;
      }
      return Boolean(fallbackResult);
    }
  }

  async approveSignature() {
    return await tryAction("confirmSignature", () => this._metamask.confirmSignature());
  }

  async approveTransaction() {
    return await tryAction("confirmTransaction", () => this._metamask.confirmTransaction());
  }

  async approveNetworkChange() {
    const approvedAddNetwork = await tryAction("approveNewNetwork", () =>
      this._metamask.approveNewNetwork(),
    );
    if (approvedAddNetwork) {
      await tryAction("approveSwitchNetwork", () => this._metamask.approveSwitchNetwork());
    }
    return approvedAddNetwork;
  }

  async recoverFromCrashOrOnboarding() {
    // Synpress handles restart/setup indirectly through the fixture bootstrap.
    return { supported: false, delegatedTo: "fixture-bootstrap" };
  }

  async collectWalletDiagnostics() {
    return {
      driver: this.name,
      release_gate_eligible: this.releaseGateEligible,
      source: "synpress",
      capabilities: {
        connect: true,
        sign: true,
        send_transaction: true,
        network_change: true,
      },
    };
  }
}

export function createSynpressDriver(metamask) {
  return new SynpressDriver(metamask);
}
