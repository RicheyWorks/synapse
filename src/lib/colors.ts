/**
 * Fixed-order categorical palette for track identity in the knowledge graph.
 * Validated (dataviz skill's validate_palette.js) against both theme surfaces
 * (#0b0c10 neural, #150f0a blackbeard): lightness band OK, contrast OK,
 * worst-adjacent CVD deltaE 10.3 (the 8-12 "floor" band) — legal only with
 * secondary encoding, so callers must always pair color with a text label
 * (track name), never rely on color alone.
 *
 * Order is the CVD-safety mechanism: it's the ordering that maximized the
 * minimum adjacent contrast when enumerated. Never reassign per filter/
 * selection, never cycle past 8 — fold any 9th+ series into the neutral.
 */
export const CATEGORICAL_PALETTE = [
  '#3987e5', // blue
  '#199e70', // aqua
  '#c98500', // yellow
  '#008300', // green
  '#9085e9', // violet
  '#e66767', // red
  '#d55181', // magenta
  '#d95926', // orange
] as const

const NEUTRAL_OVERFLOW = '#8b8fa3'

export function colorForIndex(i: number): string {
  return i >= 0 && i < CATEGORICAL_PALETTE.length ? CATEGORICAL_PALETTE[i] : NEUTRAL_OVERFLOW
}
