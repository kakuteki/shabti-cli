import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { run } from "./helpers.js";

describe("REPL / no-args (non-TTY)", () => {
  it("shows the banner when run with no args", async () => {
    const { stdout } = await run([]);
    expect(stdout).toContain("I shall do it");
  });

  it("shows help commands when run with no args (non-TTY)", async () => {
    const { stdout } = await run([]);
    expect(stdout).toContain("hello");
    expect(stdout).toContain("chat");
    expect(stdout).toContain("list");
    expect(stdout).toContain("spin");
  });

  it("--help still works as before", async () => {
    const { stdout } = await run(["--help"]);
    expect(stdout).toContain("I shall do it");
    expect(stdout).toContain("hello");
    expect(stdout).toContain("chat");
  });
});

describe("launchRepl", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    vi.resetModules();
  });

  afterEach(() => {
    process.env = { ...originalEnv };
  });

  it("returns null when OPENAI_API_KEY is not set", async () => {
    delete process.env.OPENAI_API_KEY;
    vi.doMock("dotenv/config", () => ({}));
    const { launchRepl } = await import("../src/repl/index.js");
    const result = await launchRepl();
    expect(result).toBeNull();
  });

  it("returns null when OPENAI_API_KEY is placeholder", async () => {
    process.env.OPENAI_API_KEY = "your-api-key-here";
    vi.doMock("dotenv/config", () => ({}));
    const { launchRepl } = await import("../src/repl/index.js");
    const result = await launchRepl();
    expect(result).toBeNull();
  });

  it("exports launchRepl as a function", async () => {
    vi.doMock("dotenv/config", () => ({}));
    const mod = await import("../src/repl/index.js");
    expect(typeof mod.launchRepl).toBe("function");
  });
});
