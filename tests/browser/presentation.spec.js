import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import metadata from '../../package.json' with { type: 'json' };
import { states } from '../../scripts/tray-art.mjs';
const { name } = metadata;

test('README shows the real mock flyout in both themes', async ({ browser }, info) => {
  const context = await browser.newContext({
    viewport: { width: 392, height: 640 }, deviceScaleFactor: 2,
    reducedMotion: 'reduce', locale: 'en-US', timezoneId: 'UTC',
  });
  try {
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const pictures = [];
    for (const theme of ['light', 'dark']) {
      await page.goto(`${info.project.use.baseURL}/?theme=${theme}&lang=en${name === 'homing' ? '&view=settings' : ''}`);
      await page.evaluate(() => document.fonts.ready);
      await page.addStyleTag({ content: ':root { background: #e7e9ec; }' });
      if (name === 'homing') {
        await page.getByRole('combobox').fill('Wi-Fi');
        await expect(page.getByRole('option')).toHaveCount(3);
      } else {
        await expect(page.locator('#main-devices .device-row')).toHaveCount(3);
      }
      await expect(page.locator('.flyout')).toBeVisible();
      expect(await page.locator('body').evaluate(body => body.classList.contains('native'))).toBe(false);
      const box = await page.locator('.flyout').boundingBox();
      const y = Math.max(0, Math.floor(box.y - 20));
      const png = await page.screenshot({ clip: { x: 0, y, width: 392, height: 640 - y } });
      pictures.push(`<figure><figcaption>${theme === 'light' ? 'Light' : 'Dark'}</figcaption><img width="392" src="data:image/png;base64,${png.toString('base64')}"></figure>`);
    }
    expect(errors).toEqual([]);
    await page.setViewportSize({ width: 864, height: 700 });
    await page.setContent(`<style>body{margin:0;background:#e7e9ec}main{display:flex;gap:16px;padding:24px 32px 20px;width:864px;box-sizing:border-box}figure{margin:0;width:392px}figcaption{font:12px/20px system-ui;color:#58606b;letter-spacing:.08em;padding:0 16px 8px}img{display:block}</style><main>${pictures.join('')}</main>`);
    await page.locator('img').evaluateAll(images => Promise.all(images.map(image => image.decode())));
    await page.locator('main').screenshot({ path: 'playwright-results/readme.png' });
  } finally {
    await context.close();
  }
});

test('tray artwork beside Segoe Fluent system glyphs at physical pixel sizes', async ({ browser }) => {
  const font = readFileSync(process.env.FLUENT_ICONS_FONT).toString('base64');
  const context = await browser.newContext({ deviceScaleFactor: 1, viewport: { width: 680, height: 1100 } });
  try {
    const page = await context.newPage();
    for (const theme of ['light', 'dark']) {
      const ink = theme === 'dark' ? '#fff' : '#1b1b1b';
      let rows = '<div class="row"><span>Pixels / taskbar</span><span>System Wi-Fi</span><span>System headphones</span>' + states.map(state => `<span>${state}</span>`).join('') + '</div>';
      for (const small of [false, true]) for (const size of [16, 20, 24, 32, 48]) {
        const extent = small ? size * .75 : size;
        rows += `<div class="row"><span>${size}px / ${small ? 'small' : 'normal'}</span><span class="system" style="font-size:${extent}px">&#xe701;</span><span class="system" style="font-size:${extent}px">&#xe7f6;</span>`;
        for (const state of states) {
          const png = readFileSync(`src-tauri/icons/${small ? 'small-' : ''}${state}-${theme}/${size}x${size}.png`).toString('base64');
          rows += `<span><img width="${size}" height="${size}" src="data:image/png;base64,${png}"></span>`;
        }
        rows += '</div>';
      }
      await page.setContent(`<style>@font-face{font-family:Fluent;src:url(data:font/ttf;base64,${font})}body{margin:0;background:${theme === 'dark' ? '#242424' : '#f3f3f3'};color:${ink};font:11px system-ui}main{padding:24px}h1{font-size:16px;font-weight:500}.row{display:grid;grid-template-columns:120px repeat(5,100px);align-items:center;min-height:68px;border-bottom:1px solid ${theme === 'dark' ? '#444' : '#ddd'}}.row>span{text-align:center;display:flex;align-items:center;justify-content:center}.system{font-family:Fluent;font-weight:400}img{display:block}</style><main><h1>${name} · ${theme} · 1 image pixel = 1 display pixel</h1>${rows}</main>`);
      await page.evaluate(() => document.fonts.ready);
      expect(await page.evaluate(() => document.fonts.check('16px Fluent', '\ue701\ue7f6'))).toBe(true);
      await page.locator('main').screenshot({ path: `playwright-results/tray-weight-${theme}.png` });
    }
  } finally {
    await context.close();
  }
});
