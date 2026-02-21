/**
 * Normalize text for consistent storage and search.
 * Applies NFKC Unicode normalization, trims, and collapses whitespace.
 *
 * NFKC normalizes:
 *  - Full-width ASCII → half-width (Ａ → A, ０ → 0)
 *  - Half-width katakana → full-width (ｶ → カ)
 *  - Compatibility characters (① → 1, ㍻ → 平成)
 */
export function normalizeText(text) {
  if (!text) return "";
  return text.normalize("NFKC").replace(/\s+/g, " ").trim();
}
