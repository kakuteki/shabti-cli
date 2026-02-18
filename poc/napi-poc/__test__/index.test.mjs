import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  add,
  createMemoryEntry,
  summarizeEntry,
  computeSimilarity,
  createBatchEntries,
} from "../index.js";

// ── 0-7b: Basic function call ────────────────────────────────────────────

describe("add (sync function)", () => {
  it("adds two positive numbers", () => {
    assert.equal(add(2, 3), 5);
  });

  it("handles negative numbers", () => {
    assert.equal(add(-1, -2), -3);
  });

  it("handles zero", () => {
    assert.equal(add(0, 0), 0);
  });
});

// ── 0-7c: Struct passing ─────────────────────────────────────────────────

describe("createMemoryEntry (struct → JS object)", () => {
  it("returns an object with correct fields", () => {
    const entry = createMemoryEntry("test-1", "hello world", 0.95);
    assert.equal(entry.id, "test-1");
    assert.equal(entry.content, "hello world");
    assert.equal(entry.score, 0.95);
    assert.equal(typeof entry.createdAt, "number");
  });

  it("preserves Japanese content", () => {
    const entry = createMemoryEntry("jp-1", "こんにちは世界", 0.8);
    assert.equal(entry.content, "こんにちは世界");
  });
});

describe("summarizeEntry (JS object → Rust struct)", () => {
  it("formats entry as summary string", () => {
    const summary = summarizeEntry({
      id: "a1",
      content: "test content",
      score: 0.75,
      createdAt: 1700000000,
    });
    assert.equal(summary, "[a1] test content (score: 0.75)");
  });
});

// ── 0-7d: Async function ─────────────────────────────────────────────────

describe("computeSimilarity (async → Promise)", () => {
  it("returns 1.0 for identical vectors", async () => {
    const result = await computeSimilarity([1, 2, 3], [1, 2, 3]);
    assert.ok(Math.abs(result - 1.0) < 1e-10);
  });

  it("returns 0.0 for orthogonal vectors", async () => {
    const result = await computeSimilarity([1, 0], [0, 1]);
    assert.ok(Math.abs(result) < 1e-10);
  });

  it("returns -1.0 for opposite vectors", async () => {
    const result = await computeSimilarity([1, 2, 3], [-1, -2, -3]);
    assert.ok(Math.abs(result + 1.0) < 1e-10);
  });

  it("rejects on dimension mismatch", async () => {
    await assert.rejects(
      () => computeSimilarity([1, 2], [1, 2, 3]),
      /dimension mismatch/
    );
  });
});

// ── 0-7c: Array of structs ───────────────────────────────────────────────

describe("createBatchEntries (Vec<Struct> → Array<Object>)", () => {
  it("returns correct number of entries", () => {
    const entries = createBatchEntries(10);
    assert.equal(entries.length, 10);
  });

  it("entries have correct structure", () => {
    const entries = createBatchEntries(3);
    for (const entry of entries) {
      assert.equal(typeof entry.id, "string");
      assert.equal(typeof entry.content, "string");
      assert.equal(typeof entry.score, "number");
      assert.equal(typeof entry.createdAt, "number");
    }
  });

  it("entries have sequential IDs", () => {
    const entries = createBatchEntries(5);
    assert.equal(entries[0].id, "entry-0");
    assert.equal(entries[4].id, "entry-4");
  });

  it("handles zero count", () => {
    const entries = createBatchEntries(0);
    assert.equal(entries.length, 0);
  });
});
