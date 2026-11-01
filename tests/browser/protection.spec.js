import { test, expect } from "@playwright/test";

test.use({ deviceScaleFactor: 2 });

test("the main card reports protection only for a healthy active domain match", async ({ page }, testInfo) => {
  await page.addInitScript(() => {
    window.isTauri = true;
    const callbacks = {};
    let id = 0;
    const bridge = window.__TEST_WIFI__ = {
      state: { current: "A long Windows connection label", targetName: "Different saved SSID",
        configured: true, connection: "connected", pauseRemaining: 0, pauseMinutes: 60,
        lastReconnect: null, languageMode: "en", language: "en", dark: false,
        accent: "#005fb8", accentInk: "#fff", error: null },
      publish(delta) {
        this.state = { ...this.state, ...delta };
        callbacks[this.listener]({ payload: this.state });
      },
    };
    window.__TAURI_INTERNALS__ = {
      transformCallback(fn) { callbacks[++id] = fn; return id; },
      async invoke(command, args) {
        if (command === "plugin:event|listen") { bridge.listener = args.handler; return 1; }
        if (command === "snapshot") return bridge.state;
        if (command === "resize_flyout") return { limit: 620 };
        if (["prepare_flyout", "present_flyout", "dismiss_flyout"].includes(command)) return;
        throw Error(`Unexpected simulated command ${command}`);
      },
    };
  });
  await page.goto("/");
  await expect(page.locator("body")).toHaveClass(/presented/);
  const cases = [
    { name: "protected-en-light", delta: {}, protected: true, label: "Connected · Protected" },
    { name: "protected-zh-dark", delta: { language: "zh", dark: true, accent: "#60cdff" }, protected: true, label: "已连接 · 保护中" },
    { name: "other-network", delta: { language: "en", connection: "disconnected" }, protected: false, label: "Connected" },
    { name: "unconfigured", delta: { configured: false }, protected: false, label: "Connected" },
    { name: "disconnected", delta: { configured: true, current: null }, protected: false, label: "Can't reach Different saved SSID · retrying automatically" },
    { name: "paused", delta: { language: "zh", current: "A long Windows connection label", connection: "connected", pauseRemaining: 3600 }, protected: false, label: /已暂停/ },
    { name: "guard-error", delta: { language: "en", pauseRemaining: 0, error: "Notification registration unavailable" }, protected: false, label: "Connected" },
  ];
  for (const item of cases) {
    await page.evaluate((delta) => window.__TEST_WIFI__.publish(delta), item.delta);
    await expect(page.locator("#current-status")).toHaveText(item.label);
    const shield = page.locator("#protection-shield");
    if (item.protected) {
      await expect(shield).toBeVisible();
      await expect(shield.locator("svg")).toHaveCount(1);
      const card = await page.locator(".current").boundingBox();
      const copy = await page.locator(".current-copy").boundingBox();
      const box = await shield.boundingBox();
      expect(box.x).toBeGreaterThan(copy.x + copy.width);
      expect(box.x + box.width).toBeLessThan(card.x + card.width);
      await expect(shield).toHaveAttribute("aria-hidden", "true");
      expect(await shield.evaluate((el) => el.closest("button"))).toBeNull();
    } else await expect(shield).toBeHidden();
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({ path: testInfo.outputPath(`${item.name}-200.png`) });
  }
});
