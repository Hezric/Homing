import { test, expect } from "@playwright/test";
import { writeFile } from "node:fs/promises";
import { mockState, mockSavedNetworks } from "../../src/state.js";

test.use({ reducedMotion: "no-preference", deviceScaleFactor: 2 });

async function sampleMotion(page, action, selector = "footer") {
  await page.evaluate((selector) => {
    window.motionFrames = [];
    window.samplingMotion = true;
    const sample = () => {
      const panel = document.querySelector(".flyout").getBoundingClientRect();
      const rect = document.querySelector(selector).getBoundingClientRect();
      window.motionFrames.push({ top: panel.top, bottom: panel.bottom - innerHeight,
        footer: rect.bottom - innerHeight, height: rect.height,
        nativeClip: window.nativeRegionHeight !== undefined && document.querySelector(".current").getBoundingClientRect().top < innerHeight - window.nativeRegionHeight - 1,
        outerScroll: [document.documentElement, document.body, document.querySelector(".flyout"), document.querySelector("#views")].some(el => {
          const s = getComputedStyle(el);
          const x = el.scrollWidth > el.clientWidth + 1, y = el.scrollHeight > el.clientHeight + 1;
          return (el === document.documentElement && (x || y)) ||
            (x && ["auto", "scroll"].includes(s.overflowX)) || (y && ["auto", "scroll"].includes(s.overflowY));
        }) });
      if (window.samplingMotion) requestAnimationFrame(sample);
    };
    sample();
  }, selector);
  await action();
  await page.waitForTimeout(350);
  const frames = await page.evaluate(() => { window.samplingMotion = false; return window.motionFrames; });
  const framePath = test.info().outputPath(`motion-${test.info().attachments.length}.json`);
  await writeFile(framePath, JSON.stringify(frames));
  await test.info().attach("motion-frames", {path:framePath,contentType:"application/json"});
  return frames;
}

function expectFixedBottom(frames) {
  expect(frames.length).toBeGreaterThan(3);
  expect(frames.some(f => f.outerScroll || f.nativeClip)).toBe(false);
  for (const key of ["bottom", "footer"]) {
    expect(Math.max(...frames.map(f => f[key])) - Math.min(...frames.map(f => f[key]))).toBeLessThanOrEqual(1);
  }
}

for (const [lang, theme] of [["en", "light"], ["zh", "dark"]]) {
  test(`mock expands upward and keeps its screen bottom and footer fixed (${lang}, ${theme})`, async ({ page }) => {
    await page.goto(`/?lang=${lang}&theme=${theme}`);
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(250);
    const open = await sampleMotion(page, () => page.locator("#gear").click());
    expectFixedBottom(open);
    for (let i = 1; i < open.length; i++) expect(open[i].top).toBeLessThanOrEqual(open[i - 1].top + 1);
    expect(open.at(-1).top).toBeLessThan(open[0].top);
    const close = await sampleMotion(page, () => page.locator("#gear").click());
    expectFixedBottom(close);
    for (let i = 1; i < close.length; i++) expect(close[i].top).toBeGreaterThanOrEqual(close[i - 1].top - 1);
    expect(close.at(-1).top).toBeGreaterThan(close[0].top);
  });
}

test("native resize replies keep the footer at the same simulated screen coordinate", async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 400 });
  let canvasReady = false;
  await page.exposeFunction("resizeViewport", async (height, reserve, settled) => {
    if (!canvasReady) {canvasReady = true; await page.setViewportSize({width:401,height:620});}
    await new Promise(resolve => setTimeout(resolve, 64));
    await page.evaluate(({height, reserve, settled}) => {
      window.nativeHostHeight = Math.min(579, Math.ceil(height));
      const wanted = Math.min(620, Math.ceil(Math.max(height, reserve || height)) + 41);
      window.nativeRegionHeight = settled ? wanted : Math.max(window.nativeRegionHeight || 0, wanted);
    }, {height, reserve, settled});
    return { limit: 579 };
  });
  await page.addInitScript(({ state, catalog }) => {
    window.isTauri = true;
    let id = 0;
    window.__TAURI_INTERNALS__ = {
      transformCallback() { return ++id; },
      async invoke(command, args) {
        if (command === "plugin:event|listen") return 1;
        if (command === "snapshot") return state;
        if (command === "saved_networks") return catalog;
        if (command === "resize_flyout") return window.resizeViewport(args.height, args.reserve, args.settled);
        if (["prepare_flyout", "present_flyout", "dismiss_flyout"].includes(command)) return;
        throw Error(`Unexpected simulated command ${command}`);
      },
    };
  }, { state: { ...mockState("connected"), language: "en" }, catalog: mockSavedNetworks() });
  await page.goto("/");
  await expect(page.locator("body")).toHaveClass(/presented/);
  expect((await page.locator('.flyout').boundingBox()).height).toBeLessThan(500);
  const opening = await sampleMotion(page, () => page.locator("#gear").click());
  expectFixedBottom(opening);
  expect(await page.evaluate(() => innerHeight)).toBe(620);
  await page.locator("#pause-minutes").fill("invalid");
  expectFixedBottom(await sampleMotion(page, () => page.locator("#pause-minutes").press("Enter")));
  await expect(page.locator("#pause-minutes")).toHaveAttribute("aria-invalid", "true");
  const closing = await sampleMotion(page, () => page.locator("#gear").click());
  expectFixedBottom(closing);
  expect((await page.locator(".flyout").boundingBox()).height).toBeCloseTo(await page.evaluate(() => window.nativeHostHeight), 0);
  const rapid = await sampleMotion(page, async () => {
    for (let i=0;i<8;i++) { await page.locator("#gear").click(); await page.waitForTimeout(45); }
  });
  expectFixedBottom(rapid);
  expect((await page.locator(".flyout").boundingBox()).height).toBeCloseTo(await page.evaluate(() => window.nativeHostHeight), 0);
  await expect(page.locator("#gear")).toHaveAttribute("aria-expanded", "false");
});

test("focus preserves text geometry and suggestions expand and collapse below the input", async ({ page }) => {
  await page.goto("/?view=settings");
  const geometry = selector => page.locator(selector).evaluate(el => {
    const s = getComputedStyle(el), r = el.getBoundingClientRect();
    return { center: (r.height + parseFloat(s.borderTopWidth) + parseFloat(s.paddingTop) - parseFloat(s.borderBottomWidth) - parseFloat(s.paddingBottom)) / 2,
      lineHeight: s.lineHeight, outline: s.outlineStyle, bottom: s.borderBottomColor };
  });
  for (const selector of ["#profile", "#pause-minutes"]) {
    await page.locator("#gear").focus();
    const before = await geometry(selector);
    await page.locator(selector).focus();
    await page.waitForTimeout(250);
    const after = await geometry(selector);
    expect(after.center).toBe(before.center);
    expect(after.lineHeight).toBe(before.lineHeight);
    expect(after.outline).toBe("none");
    expect(after.bottom).not.toBe(before.bottom);
  }
  const card = await page.locator(".current").boundingBox();
  const row = await page.locator(".profile-row").boundingBox();
  const pause = await page.locator("#pause").boundingBox();
  const gear = await page.locator("#gear").boundingBox();
  expect(row.x-card.x).toBeCloseTo(card.x+card.width-row.x-row.width, 0);
  expect(pause.x-card.x).toBeCloseTo(card.x+card.width-gear.x-gear.width, 0);
  const panel = await page.locator(".flyout").boundingBox();
  expect(card.x-panel.x).toBeCloseTo(panel.x+panel.width-card.x-card.width, 0);
  const save = await page.locator("#save-profile").boundingBox();
  const current = await page.locator("#use-current-profile").boundingBox();
  expect(current.width - save.width).toBeGreaterThan(10);
  expect(current.height).toBe(save.height);
  await page.locator("#profile").fill("");
  await page.locator("#gear").focus();
  await page.waitForTimeout(250);
  const open = await sampleMotion(page, () => page.locator("#profile").focus(), "#profile-popup");
  const height = open.at(-1).height;
  expect(open.some(f => f.height > 1 && f.height < height - 1)).toBe(true);
  const close = await sampleMotion(page, () => page.locator("#profile").press("Escape"), "#profile-popup");
  expect(close.some(f => f.height > 1 && f.height < height - 1)).toBe(true);
  expect(close.at(-1).height).toBe(0);
  await expect(page.locator("#profile")).toBeFocused();
});
test("rapid settings switches never paint both pages over each other", async ({ page }) => {
  await page.goto("/");
  await page.waitForTimeout(250);
  await page.evaluate(() => {
    window.overlapFrames=[];window.samplingOverlap=true;
    const painted=label=>Array.from(document.querySelectorAll("#views *")).some(el=>el.childElementCount===0&&el.textContent===label&&el.getBoundingClientRect().height>0&&getComputedStyle(el).visibility!=="hidden"&&getComputedStyle(el).display!=="none"&&Number(getComputedStyle(el.closest("section")).opacity)>.05);
    const sample=()=>{window.overlapFrames.push(painted("Saved Wi-Fi network")&&painted("Reconnect to"));if(window.samplingOverlap)requestAnimationFrame(sample);};sample();
  });
  for(let i=0;i<6;i++){await page.locator("#gear").click();await page.waitForTimeout(65);}
  await page.waitForTimeout(250);
  const overlap=await page.evaluate(()=>{window.samplingOverlap=false;return window.overlapFrames;});
  expect(overlap.some(Boolean)).toBe(false);
});