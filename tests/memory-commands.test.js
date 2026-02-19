import { describe, it, expect } from "vitest";
import { run } from "./helpers.js";

describe("store command", () => {
  it("shows help with --help", async () => {
    const { stdout, code } = await run(["store", "--help"]);
    expect(code).toBe(0);
    expect(stdout).toContain("Store a memory entry");
    expect(stdout).toContain("<content>");
    expect(stdout).toContain("--namespace");
    expect(stdout).toContain("--session");
    expect(stdout).toContain("--tags");
  });

  it("fails when no content is provided", async () => {
    const { stderr, code } = await run(["store"]);
    expect(code).not.toBe(0);
    expect(stderr).toContain("missing required argument");
  });
});

describe("search command", () => {
  it("shows help with --help", async () => {
    const { stdout, code } = await run(["search", "--help"]);
    expect(code).toBe(0);
    expect(stdout).toContain("Search memory entries");
    expect(stdout).toContain("<query>");
    expect(stdout).toContain("--limit");
    expect(stdout).toContain("--namespace");
    expect(stdout).toContain("--explain");
    expect(stdout).toContain("--follow-links");
    expect(stdout).toContain("--min-score");
    expect(stdout).toContain("--json");
  });

  it("fails when no query is provided", async () => {
    const { stderr, code } = await run(["search"]);
    expect(code).not.toBe(0);
    expect(stderr).toContain("missing required argument");
  });
});

describe("snapshot command", () => {
  it("shows help with --help", async () => {
    const { stdout, code } = await run(["snapshot", "--help"]);
    expect(code).toBe(0);
    expect(stdout).toContain("Manage storage snapshots");
    expect(stdout).toContain("create");
    expect(stdout).toContain("restore");
    expect(stdout).toContain("list");
  });
});

describe("status command", () => {
  it("shows help with --help", async () => {
    const { stdout, code } = await run(["status", "--help"]);
    expect(code).toBe(0);
    expect(stdout).toContain("Show engine status");
    expect(stdout).toContain("--json");
  });
});

describe("graceful degradation", () => {
  it("store command shows error on invalid qdrant", async () => {
    const { stdout, code } = await run(["store", "test"], {
      SHABTI_QDRANT_URL: "http://localhost:19999",
    });
    expect(stdout).toContain("[error]");
    expect(code).not.toBe(0);
  }, 15_000);

  it("search command shows error on invalid qdrant", async () => {
    const { stdout, code } = await run(["search", "test"], {
      SHABTI_QDRANT_URL: "http://localhost:19999",
    });
    expect(stdout).toContain("[error]");
    expect(code).not.toBe(0);
  }, 15_000);
});
