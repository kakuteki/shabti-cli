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
    expect(stdout).toContain("a2a");
    expect(stdout).toContain("hello");
    expect(stdout).toContain("list");
    expect(stdout).toContain("spin");
    expect(stdout).toContain("store");
    expect(stdout).toContain("export");
    expect(stdout).toContain("import");
    expect(stdout).toContain("search");
    expect(stdout).toContain("snapshot");
    expect(stdout).toContain("status");
    expect(stdout).toContain("config");
    expect(stdout).toContain("gc");
  });

  it("shows version with --version", async () => {
    const { stdout } = await run(["--version"]);
    expect(stdout.trim()).toMatch(/^\d+\.\d+\.\d+$/);
  });

  it("mcp-config outputs valid JSON with mcpServers", async () => {
    const { stdout, code } = await run(["mcp-config"]);
    expect(code).toBe(0);
    const config = JSON.parse(stdout);
    expect(config.mcpServers).toBeDefined();
    expect(config.mcpServers["shabti-memory"].command).toBe("npx");
    expect(config.mcpServers["shabti-memory"].args).toContain("shabti-mcp");
  });
});
