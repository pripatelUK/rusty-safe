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
    return { url, body, buttons };
  }

  async _clickApprovalControls(page) {
    let clicked = 0;
    const testIds = [
      "confirm-btn",
      "confirm-footer-button",
      "page-container-footer-next",
      "request-confirm-button",
      "allow-authorize-button",
    ];
    for (const testId of testIds) {
      const locator = page.getByTestId(testId).first();
      const canClick =
        (await locator.isVisible().catch(() => false)) && (await locator.isEnabled().catch(() => false));
      if (!canClick) {
        continue;
      }
      await locator.click().catch(() => {});
      clicked += 1;
      await page.waitForTimeout(250).catch(() => {});
    }

    const genericButton = page
      .getByRole("button", { name: /connect|next|approve|confirm|sign|submit|continue|ok/i })
      .first();
    const canClickGeneric =
      (await genericButton.isVisible().catch(() => false)) &&
      (await genericButton.isEnabled().catch(() => false));
    if (canClickGeneric) {
      await genericButton.click().catch(() => {});
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

    const notificationUrl = `chrome-extension://${extensionId}/notification.html`;
    let notificationEnsured = false;
    let totalClicks = 0;

    for (let attempt = 0; attempt < 12; attempt += 1) {
      let pages = this._candidatePages(context, extensionId);
      if (!notificationEnsured) {
        const hasNotification = this._extensionPages(context, extensionId).some((page) =>
          page.url().startsWith(notificationUrl),
        );
        if (!hasNotification) {
          try {
            const notificationPage = await context.newPage();
            await notificationPage.goto(notificationUrl, { waitUntil: "domcontentloaded" });
          } catch (error) {
            console.log(
              `[synpress-driver] ${label} notification goto failed: ${String(error?.message ?? error)}`,
            );
          }
        }
        notificationEnsured = true;
        pages = this._candidatePages(context, extensionId);
      }

      for (const page of pages) {
        await page.bringToFront().catch(() => {});
        totalClicks += await this._clickApprovalControls(page);
      }

      if (totalClicks > 0 && attempt >= 2) {
        break;
      }
      await context.waitForEvent("page", { timeout: 500 }).catch(() => {});
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
    try {
      await this._metamask.connectToDapp();
      return true;
    } catch (error) {
      console.log(`[synpress-driver] connectToDapp unavailable: ${String(error?.message ?? error)}`);
      const pageUrls = this._metamask.context
        .pages()
        .filter((page) => !page.isClosed())
        .map((page) => page.url());
      console.log(`[synpress-driver] connectToDapp context-pages=${JSON.stringify(pageUrls)}`);
      return await this._approveFromExtensionSurfaces("connectToDapp");
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
