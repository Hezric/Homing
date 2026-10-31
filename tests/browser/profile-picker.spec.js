import { test, expect } from "@playwright/test";

for (const lang of ["en", "zh"])
  for (const theme of ["light", "dark"]) {
    test(`saved network names filter, select and stay above the footer (${lang}, ${theme})`, async ({
      page,
    }, testInfo) => {
      const errors = [];
      page.on("pageerror", (error) => errors.push(error.message));
      page.on("console", (message) => {
        if (["error", "warning"].includes(message.type())) errors.push(message.text());
      });
      await page.goto(`/?view=settings&lang=${lang}&theme=${theme}`);
      await expect(page).toHaveTitle("Homing");
      const input = page.getByRole("combobox");
      const panel = page.locator(".flyout");
      const height = (await panel.boundingBox()).height;
      await input.fill("");
      await input.focus();
      const list = page.getByRole("listbox");
      await expect(list).toBeVisible();
      await expect(page.locator("#use-current-profile")).toHaveText(
        lang === "en" ? "Use current" : "使用当前网络",
      );
      const fieldBox = await input.boundingBox();
      const popupBox = await page.locator("#profile-popup").boundingBox();
      expect(popupBox.x).toBeCloseTo(fieldBox.x, 0);
      expect(popupBox.width).toBeCloseTo(fieldBox.width, 0);
      expect(popupBox.y).toBeCloseTo(fieldBox.y + fieldBox.height, 0);
      const focusStyle = await input.evaluate((el) => ({
        bottom: getComputedStyle(el).borderBottomWidth,
        color: getComputedStyle(el).borderBottomColor,
        shadow: getComputedStyle(el).boxShadow,
        outline: getComputedStyle(el).outlineStyle,
      }));
      expect(focusStyle.bottom).toBe("2px");
      expect(focusStyle.shadow).toBe("none");
      expect(focusStyle.outline).toBe("none");
      expect(
        await page.locator("#profile-popup").evaluate((el) => getComputedStyle(el).borderTopWidth),
      ).toBe("0px");
      const labelBox = await page.locator("#profile-label").boundingBox();
      const currentBox = await page.locator("#use-current-profile").boundingBox();
      expect(
        Math.abs(labelBox.y + labelBox.height / 2 - currentBox.y - currentBox.height / 2),
      ).toBeLessThan(2);
      expect(currentBox.y + currentBox.height).toBeLessThanOrEqual(fieldBox.y);
      expect(await list.getByRole("option").count()).toBeGreaterThan(20);
      const bounds = await list.evaluate((el) => ({
        bottom: el.getBoundingClientRect().bottom,
        height: el.clientHeight,
        scroll: el.scrollHeight,
      }));
      expect(bounds.scroll).toBeGreaterThan(bounds.height);
      expect(bounds.bottom).toBeLessThanOrEqual((await page.locator("footer").boundingBox()).y - 3);
      expect((await panel.boundingBox()).height).toBe(height);
      await page.screenshot({ path: testInfo.outputPath("saved-profiles.png") });
      await input.fill("Studio");
      const broad = await list.getByRole("option").count();
      await input.fill("Studio 1");
      expect(await list.getByRole("option").count()).toBeLessThan(broad);
      await input.press("ArrowDown");
      const selected = await input.getAttribute("aria-activedescendant");
      const selectedText = await page.locator(`#${selected}`).textContent();
      expect(
        await page.locator(`#${selected}`).evaluate((el) => getComputedStyle(el).outlineStyle),
      ).toBe("none");
      await page.screenshot({ path: testInfo.outputPath("active-network-option.png") });
      await input.press("Enter");
      await expect(input).toHaveValue(selectedText);
      await expect(list).toBeHidden();
      await expect(page.locator("#target")).toHaveText("Home Wi-Fi");
      await input.press("Tab");
      await expect(page.locator("#target")).toHaveText("Home Wi-Fi");
      const idleStyle = await input.evaluate((el) => ({
        color: getComputedStyle(el).borderBottomColor,
        shadow: getComputedStyle(el).boxShadow,
        outline: getComputedStyle(el).outlineStyle,
      }));
      expect(idleStyle.color).not.toBe(focusStyle.color);
      expect(idleStyle.shadow).toBe("none");
      expect(idleStyle.outline).toBe("none");
      await page.screenshot({ path: testInfo.outputPath("idle-network-input.png") });
      await page.locator("#save-profile").click();
      await expect(page.locator("#target")).toHaveText(selectedText);
      await input.fill("家里的网络");
      await expect(list.getByRole("option")).toHaveText(["家里的网络"]);
      await input.press("Escape");
      await expect(list).toBeHidden();
      await expect(panel).toBeVisible();
      await expect(input).toHaveAttribute("aria-expanded", "false");
      expect(errors).toEqual([]);
    });
  }

test("manual validation, stale enumeration and IME preserve the previous target", async ({
  page,
}) => {
  await page.goto("/?view=settings&profiles=stale");
  const input = page.getByRole("combobox");
  for (const value of ["", "Not saved", "Removed network", "Hidden Home alias"]) {
    await input.fill(value);
    await page.locator("#save-profile").click();
    await expect(page.locator("#profile-error")).toBeVisible();
    await expect(input).toHaveAttribute("aria-invalid", "true");
    await expect(page.locator("#target")).toHaveText("Home Wi-Fi");
    await expect(input).toHaveValue(value);
  }
  await input.fill("家里的网络");
  await input.dispatchEvent("keydown", { key: "Enter", isComposing: true });
  await expect(page.locator("#target")).toHaveText("Home Wi-Fi");
  await input.press("Tab");
  await page.locator("#save-profile").click();
  await expect(page.locator("#target")).toHaveText("家里的网络");
});

test("current shortcut resolves network names and disables uncertain or failed lookups", async ({
  page,
}, testInfo) => {
  await page.goto("/?view=settings");
  await page.getByRole("combobox").fill("Studio 02");
  await page.locator("#save-profile").click();
  await expect(page.locator("#target")).toHaveText("Studio 02");
  await page.locator("#use-current-profile").click();
  await expect(page.locator("#target")).toHaveText("Home Wi-Fi");
  expect(await page.locator("body").textContent()).not.toContain("Hidden alias");
  await page.goto("/?view=settings&profiles=no-profile-identity");
  await expect(page.locator("#use-current-profile")).toBeEnabled();
  await page.goto("/?view=settings&profiles=ambiguous");
  await page.getByRole("combobox").fill("Shared Wi-Fi");
  await page.locator("#save-profile").click();
  await expect(page.locator("#profile-error")).toContainText("more than one saved entry");
  await expect(page.locator("#target")).toHaveText("Home Wi-Fi");
  for (const mode of ["no-current", "current-mismatch", "empty", "unavailable"]) {
    await page.goto(`/?view=settings&profiles=${mode}`);
    await expect(page.locator("#use-current-profile")).toBeDisabled();
    await expect(page.locator("#current-profile-hint")).not.toBeEmpty();
    const input = page.getByRole("combobox");
    await input.fill("No match");
    await expect(page.locator("#profile-list-status")).toBeVisible();
    await page.locator("#save-profile").click();
    await expect(page.locator("#target")).toHaveText("Home Wi-Fi");
    await expect(page.locator("#profile-error")).toBeVisible();
    if (mode === "unavailable") {
      await input.focus();
      await input.press("Tab");
      await page.keyboard.press("Tab");
      await expect(page.locator("#retry-profiles")).toBeFocused();
      await page.keyboard.press("Enter");
      await expect(page.locator("#current-profile-hint")).toContainText(
        "Could not read saved networks",
      );
      await page.screenshot({ path: testInfo.outputPath("unavailable-profiles.png") });
    }
  }
});

test("compact geometry, long names and keyboard scrolling remain accessible", async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 480 });
  await page.goto("/?view=settings&theme=dark");
  await page.locator("body").evaluate((el) => el.classList.add("native"));
  const input = page.getByRole("combobox");
  await input.fill("");
  await input.focus();
  const list = page.getByRole("listbox");
  for (let i = 0; i < 25; i++) await input.press("ArrowDown");
  const id = await input.getAttribute("aria-activedescendant");
  const selected = await page.locator(`#${id}`).boundingBox();
  const box = await list.boundingBox();
  expect(selected.y).toBeGreaterThanOrEqual(box.y);
  expect(selected.y + selected.height).toBeLessThanOrEqual(box.y + box.height);
  expect(box.y + box.height).toBeLessThanOrEqual(
    (await page.locator("footer").boundingBox()).y - 3,
  );
  await input.fill("Long");
  const option = list.getByRole("option").first();
  await expect(option).toHaveAttribute("title", /Long/);
  expect(await option.evaluate((el) => el.scrollWidth > el.clientWidth)).toBe(true);
  await input.press("Tab");
  await expect(list).toBeHidden();
});

test("a slow saved target reply preserves a later draft and reports its committed target", async ({ page }) => {
  await page.addInitScript(() => {
    window.isTauri = true;
    const callbacks = {};
    let id = 0;
    const bridge = window.__TEST_WIFI__ = {
      pending: null,
      state: { current: "Home Wi-Fi", targetName: "Home Wi-Fi", configured: true,
        connection: "connected", pauseRemaining: 0, pauseMinutes: 60, lastReconnect: null,
        languageMode: "en", language: "en", dark: false, accent: "#005fb8", accentInk: "#fff" },
      complete(ok) {
        const pending = this.pending;
        this.pending = null;
        if (!ok) return pending.reject("networkNotSaved");
        this.state = { ...this.state, targetName: pending.action.network ?? "Home Wi-Fi" };
        callbacks[this.listener]({ payload: this.state });
        pending.resolve(this.state);
      },
    };
    window.__TAURI_INTERNALS__ = {
      transformCallback(fn) { callbacks[++id] = fn; return id; },
      async invoke(command, args) {
        if (command === "plugin:event|listen") { bridge.listener = args.handler; return 1; }
        if (command === "snapshot") return bridge.state;
        if (command === "saved_networks") return { names: ["Home Wi-Fi", "Studio 10", "Office Wi-Fi"], current: "Home Wi-Fi" };
        if (command === "resize_flyout") return { limit: 620 };
        if (command === "update") return new Promise((resolve, reject) => { bridge.pending = { action: args.action, resolve, reject }; });
        if (["prepare_flyout", "present_flyout", "dismiss_flyout"].includes(command)) return;
        throw Error(`Unexpected simulated command ${command}`);
      },
    };
  });
  await page.goto("/");
  await expect(page.locator("body")).toHaveClass(/presented/);
  await page.locator("#gear").click();
  const input = page.getByRole("combobox");
  await input.fill("Studio 10");
  await page.locator("#save-profile").click();
  await page.waitForFunction(() => window.__TEST_WIFI__.pending !== null);
  await input.fill("Office Wi-Fi");
  await page.waitForTimeout(5100);
  await expect(input).toHaveValue("Office Wi-Fi");
  await expect(page.locator("#profile-error")).toBeHidden();
  await expect(page.locator("#save-profile")).toBeDisabled();
  await page.evaluate(() => window.__TEST_WIFI__.complete(true));
  await expect(page.locator("#target")).toHaveText("Studio 10");
  await expect(page.locator("#save-profile")).toBeEnabled();
  await expect(input).toHaveValue("Office Wi-Fi");
  await page.locator("#save-profile").click();
  await page.waitForFunction(() => window.__TEST_WIFI__.pending !== null);
  await page.evaluate(() => window.__TEST_WIFI__.complete(true));
  await expect(page.locator("#target")).toHaveText("Office Wi-Fi");
  await expect(input).toHaveValue("Office Wi-Fi");
  await input.fill("Not saved");
  await page.locator("#save-profile").click();
  await page.waitForFunction(() => window.__TEST_WIFI__.pending !== null);
  await page.evaluate(() => window.__TEST_WIFI__.complete(false));
  await expect(page.locator("#profile-error")).toBeVisible();
  await expect(input).toHaveValue("Not saved");
  await expect(page.locator("#target")).toHaveText("Office Wi-Fi");
});
