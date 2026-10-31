export const states = ["connected", "disconnected", "paused"];
export function draw(size, state, ink) {
  const mark = state === "connected"
    ? '<path d="M21.6 18a3.6 3.6 0 1 1-1.91-3.18M19 12.92L20.22 15.1L18.04 16.32" fill="none" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>'
    : state === "paused"
      ? '<rect x="15.2" y="13.8" width="2" height="8.4" rx=".8" stroke="none"/><rect x="18.8" y="13.8" width="2" height="8.4" rx=".8" stroke="none"/>'
      : '<path d="M18 14.8v3.8" fill="none" stroke-width="2" stroke-linecap="round"/><circle cx="18" cy="21.1" r="1.1" stroke="none"/>';
  return `<g transform="scale(${size / 24})" fill="${ink}" stroke="${ink}"><defs><mask id="mark" maskUnits="userSpaceOnUse" x="0" y="0" width="24" height="24"><rect width="24" height="24" fill="white"/><circle cx="18" cy="18" r="6.6" fill="black"/></mask></defs><g mask="url(#mark)"><path d="M2.3 10.6a12 12 0 0 1 16.9 0M5.2 13.5a8 8 0 0 1 11.1 0M8 16.3a4 4 0 0 1 5.6 0" fill="none" stroke-width="2" stroke-linecap="round"/><circle cx="10.8" cy="19.6" r="1.6" stroke="none"/></g>${mark}</g>`;
}
