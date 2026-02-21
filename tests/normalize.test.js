import { describe, it, expect } from "vitest";
import { normalizeText } from "../src/utils/normalize.js";

describe("normalizeText", () => {
  it("applies NFKC normalization to full-width ASCII", () => {
    expect(normalizeText("Ｒｕｓｔ")).toBe("Rust");
  });

  it("normalizes full-width digits", () => {
    expect(normalizeText("１２３")).toBe("123");
  });

  it("normalizes half-width katakana to full-width", () => {
    expect(normalizeText("ｶﾀｶﾅ")).toBe("カタカナ");
  });

  it("normalizes circled numbers", () => {
    // NFKC converts ① → 1, ② → 2, ③ → 3 (no spacing added)
    expect(normalizeText("①②③")).toBe("123");
  });

  it("preserves normal Japanese text", () => {
    const text = "猫が好きです";
    expect(normalizeText(text)).toBe(text);
  });

  it("preserves normal ASCII text", () => {
    const text = "hello world";
    expect(normalizeText(text)).toBe(text);
  });

  it("trims whitespace", () => {
    expect(normalizeText("  hello  ")).toBe("hello");
  });

  it("collapses multiple spaces", () => {
    expect(normalizeText("hello   world")).toBe("hello world");
  });

  it("handles mixed Japanese and ASCII", () => {
    expect(normalizeText("Ｔｏｋｙｏは日本の首都")).toBe("Tokyoは日本の首都");
  });

  it("returns empty string for empty input", () => {
    expect(normalizeText("")).toBe("");
  });

  it("handles null/undefined gracefully", () => {
    expect(normalizeText(null)).toBe("");
    expect(normalizeText(undefined)).toBe("");
  });
});
