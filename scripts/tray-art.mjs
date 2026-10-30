export const states = [
  "connected",
  "disconnected",
  "reconnecting-0",
  "reconnecting-1",
  "reconnecting-2",
  "paused",
];
export function draw(size, state, ink) {
  const scale = size / 16;
  const shapes = [
    "M0 4.5Q8 -1.5 16 4.5L14.7 6Q8 1 1.3 6Z",
    "M3 7.5Q8 3.7 13 7.5L11.7 9Q8 6.4 4.3 9Z",
    "M6 10.5Q8 9 10 10.5L8 13Z",
  ];
  const outlines = [
    "M.5 4.8Q8 -.8 15.5 4.8",
    "M3.5 7.8Q8 4.4 12.5 7.8",
    "M6.5 10.7Q8 9.6 9.5 10.7L8 12.2Z",
  ];
  const fill = state.startsWith("reconnecting")
    ? Number(state.at(-1)) + 1
    : state === "disconnected"
      ? 0
      : 3;
  const wifi = shapes
    .map((d, i) =>
      i >= 3 - fill
        ? `<path d="${d}"/>`
        : `<path d="${outlines[i]}" fill="none" stroke="${ink}" stroke-width="${Math.max(1.25, 1 / scale)}" stroke-linecap="round" stroke-linejoin="round"/>`,
    )
    .join("");
  const mark =
    state === "paused"
      ? '<rect x="11" y="10" width="1.5" height="5"/><rect x="14" y="10" width="1.5" height="5"/>'
      : `<g fill="none" stroke="${ink}" stroke-width="${Math.max(1.25, 1 / scale)}" stroke-linecap="square"><path d="M11 12a2.3 2.3 0 0 1 4-1M15 13a2.3 2.3 0 0 1-4 1"/><path d="M15 9.5v2h-2M11 15.5v-2h2"/></g>`;
  return `<g transform="scale(${scale})"><defs><mask id="mark"><rect width="16" height="16" fill="white"/><rect x="10" y="9" width="6" height="7" fill="black"/></mask></defs><g mask="url(#mark)">${wifi}</g>${mark}</g>`;
}
