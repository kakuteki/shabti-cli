import { describe, it, expect } from "vitest";
import { run } from "./helpers.js";

describe("config command", () => {
  it("shows help with --help", async () => {
    const { stdout, code } = await run(["config", "--help"]);
    expect(code).toBe(0);
    expect(stdout).toContain("Manage shabti configuration");
    expect(stdout).toContain("show");
    expect(stdout).toContain("set");
  });

  it("config show displays current settings", async () => {
    const { stdout, code } = await run(["config", "show"]);
    expect(code).toBe(0);
    expect(stdout).toContain("qdrant_url");
    expect(stdout).toContain("data_dir");
    expect(stdout).toContain("collection_name");
  });

  it("config show --json outputs valid JSON", async () => {
    const { stdout, code } = await run(["config", "show", "--json"]);
    expect(code).toBe(0);
    const parsed = JSON.parse(stdout);
    expect(parsed).toHaveProperty("qdrant_url");
    expect(parsed).toHaveProperty("data_dir");
    expect(parsed).toHaveProperty("collection_name");
  });

  it("config set requires key and value arguments", async () => {
    const { stderr, code } = await run(["config", "set"]);
    expect(code).not.toBe(0);
    expect(stderr).toContain("missing required argument");
  });

  it("config set rejects unknown keys", async () => {
    const { stdout, code } = await run(["config", "set", "unknown_key", "value"]);
    expect(code).not.toBe(0);
    expect(stdout).toContain("Unknown config key");
  });

  it("config setup --check reports connection result", async () => {
    const { stdout, code } = await run(["config", "setup", "--check"]);
    // Either succeeds (Qdrant running) or fails gracefully
    expect(stdout).toContain("Qdrant");
    if (code === 0) {
      expect(stdout).toContain("reachable");
    } else {
      expect(stdout).toContain("Cannot reach");
    }
  });
});
