import { describe, it, expect } from "vitest";
import { run } from "./helpers.js";

describe("help / default output", () => {
  it("shows the banner when run with no args", async () => {
    const { stdout } = await run([]);
    expect(stdout).toContain("I shall do it");
  });

  it("shows the banner with --help", async () => {
    const { stdout } = await run(["--help"]);
    expect(stdout).toContain("I shall do it");
  });

  it("lists available commands in --help", async () => {
    const { stdout } = await run(["--help"]);
    expect(stdout).toContain("hello");
    expect(stdout).toContain("list");
    expect(stdout).toContain("spin");
    expect(stdout).toContain("store");
    expect(stdout).toContain("search");
    expect(stdout).toContain("snapshot");
    expect(stdout).toContain("status");
    expect(stdout).toContain("config");
  });

  it("shows version with --version", async () => {
    const { stdout } = await run(["--version"]);
    expect(stdout.trim()).toMatch(/^\d+\.\d+\.\d+$/);
  });
});
