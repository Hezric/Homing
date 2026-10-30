import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

const sizes = [16, 20, 24, 32, 48];
const glyph = name => readFileSync(`assets/fluent-v1/${name}.svg`, 'utf8').replace(/<svg[^>]*>|<\/svg>/g, '');
for (const small of [false, true]) for (const state of ['connected', 'reconnecting', 'disconnected', 'paused']) for (const theme of ['light', 'dark']) {
  const variant = `${small ? 'small-' : ''}${state}-${theme}`;
  const entries = [];
  for (const size of sizes) {
    const extent = small ? size * .75 : size;
    const inset = Math.floor((size - extent) / 2);
    const ink = theme === 'dark' ? '#ffffff' : '#1b1b1b';
    const wifi = glyph(`${state === 'disconnected' ? 'wifi_off' : 'wifi_1'}_20_regular`);
    const badge = glyph(`${state === 'paused' ? 'pause' : 'arrow_sync'}_16_filled`);
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}" fill="${ink}"><g transform="translate(${inset} ${inset}) scale(${extent / 20})"><defs><mask id="cut" maskUnits="userSpaceOnUse" x="0" y="0" width="20" height="20"><rect width="20" height="20" fill="white"/><circle cx="15.5" cy="15.5" r="5.1" fill="black"/></mask></defs><g mask="url(#cut)" opacity="${state === 'reconnecting' ? '.4' : '1'}"><g transform="scale(1.25) translate(-2 -2)" stroke="${ink}" stroke-width=".3" stroke-linejoin="round">${wifi}</g></g><g transform="translate(11 11) scale(.5625)">${badge}</g></g></svg>`;
    mkdirSync(`src-tauri/icons/${variant}`, { recursive: true });
    const source = `assets/tray-${variant}-${size}.svg`;
    writeFileSync(source, svg);
    execFileSync(process.execPath, ['node_modules/@tauri-apps/cli/tauri.js', 'icon', source, '--output', `src-tauri/icons/${variant}`, '--png', String(size)], { stdio: 'inherit' });
    if (small) continue;
    const png = readFileSync(`src-tauri/icons/${variant}/${size}x${size}.png`);
    const entry = Buffer.alloc(16);
    entry[0] = size; entry[1] = size; entry.writeUInt16LE(1, 4); entry.writeUInt16LE(32, 6); entry.writeUInt32LE(png.length, 8);
    entries.push({ entry, png });
  }
  if (small) continue;
  const header = Buffer.alloc(6); header.writeUInt16LE(1, 2); header.writeUInt16LE(entries.length, 4);
  let offset = 6 + entries.length * 16;
  for (const { entry, png } of entries) { entry.writeUInt32LE(offset, 12); offset += png.length; }
  writeFileSync(`src-tauri/icons/${variant}.ico`, Buffer.concat([header, ...entries.map(e => e.entry), ...entries.map(e => e.png)]));
}