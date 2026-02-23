import { describe, it, expect, afterAll } from "vitest";
import { run } from "./helpers.js";
import { writeFileSync, unlinkSync, existsSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import { randomUUID } from "crypto";

// Check if Qdrant is reachable AND engine can initialize
let engineAvailable = false;
try {
  const res = await fetch("http://localhost:6333/healthz", { signal: AbortSignal.timeout(2000) });
  if (res.ok) {
    const { createEngine } = await import("../src/core/engine.js");
    const eng = createEngine();
    await eng.shutdown();
    engineAvailable = true;
  }
} catch {
  // Engine not available
}

// Temp file helpers
const tempFiles = [];
function createTempFile(content) {
  const path = join(tmpdir(), `shabti-test-${randomUUID()}.jsonl`);
  writeFileSync(path, content, "utf8");
  tempFiles.push(path);
  return path;
}
afterAll(() => {
  for (const f of tempFiles) {
    try {
      if (existsSync(f)) unlinkSync(f);
    } catch {
      // best-effort
    }
  }
});

// ================================================================
// Import edge cases (no Qdrant needed)
// ================================================================

describe("import command edge cases", () => {
  it("errors when file does not exist", async () => {
    const { stdout, stderr, code } = await run(["import", "nonexistent-file.jsonl"]);
    const output = stdout + stderr;
    expect(code).not.toBe(0);
    expect(output).toContain("File not found");
  });

  it("reports no entries for empty file", async () => {
    const path = createTempFile("");
    const { stdout, code } = await run(["import", path]);
    expect(code).toBe(0);
    expect(stdout).toContain("No entries found");
  });

  it("dry-run parses entries without storing", async () => {
    const content = [
      JSON.stringify({ content: "entry one", namespace: "test" }),
      JSON.stringify({ content: "entry two", namespace: "test" }),
    ].join("\n");
    const path = createTempFile(content);
    const { stdout, code } = await run(["import", path, "--dry-run"]);
    expect(code).toBe(0);
    expect(stdout).toContain("Dry run");
    expect(stdout).toContain("2 entries parsed");
  });

  it("dry-run warns about invalid JSON lines", async () => {
    const content = [
      JSON.stringify({ content: "valid entry" }),
      "this is not json",
      JSON.stringify({ content: "another valid entry" }),
    ].join("\n");
    const path = createTempFile(content);
    const { stdout, code } = await run(["import", path, "--dry-run"]);
    expect(code).toBe(0);
    expect(stdout).toContain("Skipping invalid JSON on line 2");
    expect(stdout).toContain("2 entries parsed");
  });
});

// ================================================================
// Snapshot subcommand help
// ================================================================

describe("snapshot subcommands", () => {
  it("snapshot create --help shows usage", async () => {
    const { stdout, code } = await run(["snapshot", "create", "--help"]);
    expect(code).toBe(0);
    expect(stdout).toContain("Create a new snapshot");
  });

  it("snapshot restore --help shows usage with id argument", async () => {
    const { stdout, code } = await run(["snapshot", "restore", "--help"]);
    expect(code).toBe(0);
    expect(stdout).toContain("Restore from a snapshot");
    expect(stdout).toContain("<id>");
  });

  it("snapshot list --help shows usage", async () => {
    const { stdout, code } = await run(["snapshot", "list", "--help"]);
    expect(code).toBe(0);
    expect(stdout).toContain("List all snapshots");
  });
});

// ================================================================
// Health command (works with or without Qdrant)
// ================================================================

describe("health command output", () => {
  it("health --json returns valid JSON with healthy and checks", async () => {
    const { stdout } = await run(["health", "--json"], {}, { timeout: 15_000 });
    // health always completes (wraps errors in checks)
    const output = JSON.parse(stdout);
    expect(typeof output.healthy).toBe("boolean");
    expect(output.checks).toBeInstanceOf(Array);
    expect(output.checks.length).toBeGreaterThanOrEqual(1);
    for (const check of output.checks) {
      expect(check).toHaveProperty("name");
      expect(check).toHaveProperty("status");
      expect(check).toHaveProperty("message");
    }
  }, 20_000);

  it("health default output contains check names", async () => {
    const { stdout } = await run(["health"], {}, { timeout: 15_000 });
    expect(stdout).toContain("qdrant");
    expect(stdout).toContain("Health Check");
  }, 20_000);
});

// ================================================================
// Qdrant-dependent functional tests
// ================================================================

describe.skipIf(!engineAvailable)("CLI functional tests (Qdrant)", () => {
  const testTag = `cli-test-${Date.now()}`;
  let storedId = null;

  it("store command stores content and returns ID", async () => {
    const content = `CLI test entry ${testTag}`;
    const { stdout, code } = await run(
      ["store", content, "--tags", testTag, "--namespace", "cli-test"],
      {},
      { timeout: 30_000 },
    );
    expect(code).toBe(0);
    expect(stdout).toContain("Stored:");
    // Extract UUID from "[success] Stored: <uuid>"
    const match = stdout.match(/Stored:\s+([0-9a-f-]{36})/);
    expect(match).not.toBeNull();
    storedId = match[1];
  }, 35_000);

  it("store duplicate shows skip message", async () => {
    const content = `CLI test entry ${testTag}`;
    const { stdout, code } = await run(
      ["store", content, "--tags", testTag, "--namespace", "cli-test"],
      {},
      { timeout: 30_000 },
    );
    expect(code).toBe(0);
    expect(stdout).toContain("Skipped (duplicate)");
  }, 35_000);

  it("get retrieves stored entry by ID", async () => {
    expect(storedId).not.toBeNull();
    const { stdout, code } = await run(["get", storedId], {}, { timeout: 30_000 });
    expect(code).toBe(0);
    expect(stdout).toContain("Memory Entry");
    expect(stdout).toContain(storedId);
    expect(stdout).toContain("cli-test");
  }, 35_000);

  it("get --json returns valid JSON", async () => {
    expect(storedId).not.toBeNull();
    const { stdout, code } = await run(["get", storedId, "--json"], {}, { timeout: 30_000 });
    expect(code).toBe(0);
    const entry = JSON.parse(stdout);
    expect(entry.id).toBe(storedId);
    expect(entry.content).toContain("CLI test entry");
    expect(entry.namespace).toBe("cli-test");
  }, 35_000);

  it("search finds stored entry", async () => {
    const { stdout, code } = await run(
      ["search", `CLI test entry ${testTag}`, "--namespace", "cli-test"],
      {},
      { timeout: 30_000 },
    );
    expect(code).toBe(0);
    expect(stdout).toContain("Search results");
  }, 35_000);

  it("search --json returns valid JSON array", async () => {
    const { stdout, code } = await run(
      ["search", `CLI test entry ${testTag}`, "--json", "--namespace", "cli-test"],
      {},
      { timeout: 30_000 },
    );
    expect(code).toBe(0);
    const results = JSON.parse(stdout);
    expect(results).toBeInstanceOf(Array);
    expect(results.length).toBeGreaterThan(0);
    expect(results[0]).toHaveProperty("id");
    expect(results[0]).toHaveProperty("content");
    expect(results[0]).toHaveProperty("score");
  }, 35_000);

  it("status --json returns valid JSON with fields", async () => {
    const { stdout, code } = await run(["status", "--json"], {}, { timeout: 30_000 });
    expect(code).toBe(0);
    const status = JSON.parse(stdout);
    expect(typeof status.entryCount).toBe("number");
    expect(typeof status.tier).toBe("string");
    expect(typeof status.qdrantUrl).toBe("string");
    expect(typeof status.modelId).toBe("string");
  }, 35_000);

  it("export outputs JSONL to stdout", async () => {
    const { stdout, stderr, code } = await run(
      ["export", "--namespace", "cli-test", "--limit", "5"],
      {},
      { timeout: 30_000 },
    );
    expect(code).toBe(0);
    expect(stderr).toContain("entries exported");
    // Each non-empty line should be valid JSON
    const lines = stdout.split("\n").filter((l) => l.trim());
    expect(lines.length).toBeGreaterThan(0);
    for (const line of lines) {
      const entry = JSON.parse(line);
      expect(entry).toHaveProperty("id");
      expect(entry).toHaveProperty("content");
    }
  }, 35_000);

  it("gc runs without error", async () => {
    const { stdout, code } = await run(["gc"], {}, { timeout: 30_000 });
    expect(code).toBe(0);
    expect(stdout).toMatch(/removed|GC/i);
  }, 35_000);

  it("delete removes entry", async () => {
    expect(storedId).not.toBeNull();
    const { stdout, code } = await run(["delete", storedId], {}, { timeout: 30_000 });
    expect(code).toBe(0);
    expect(stdout).toContain("Deleted:");
    expect(stdout).toContain(storedId);
  }, 35_000);

  it("store with --ttl accepts valid TTL", async () => {
    const { stdout, code } = await run(
      ["store", `TTL test ${testTag}`, "--ttl", "3600"],
      {},
      { timeout: 30_000 },
    );
    expect(code).toBe(0);
    expect(stdout).toContain("Stored:");
  }, 35_000);

  it("store --ttl rejects non-numeric value", async () => {
    const { stdout, stderr, code } = await run(
      ["store", "test content", "--ttl", "abc"],
      {},
      { timeout: 30_000 },
    );
    const output = stdout + stderr;
    expect(code).not.toBe(0);
    expect(output).toContain("positive integer");
  }, 35_000);

  it("search --limit rejects invalid value", async () => {
    const { stdout, stderr, code } = await run(
      ["search", "test", "--limit", "-1"],
      {},
      { timeout: 30_000 },
    );
    const output = stdout + stderr;
    expect(code).not.toBe(0);
    expect(output).toContain("positive integer");
  }, 35_000);

  it("search --min-score rejects value > 1", async () => {
    const { stdout, stderr, code } = await run(
      ["search", "test", "--min-score", "2.0"],
      {},
      { timeout: 30_000 },
    );
    const output = stdout + stderr;
    expect(code).not.toBe(0);
    expect(output).toContain("between 0 and 1");
  }, 35_000);
});
