import { describe, it, expect } from "vitest";
import { run } from "./helpers.js";

describe("Qdrant setup guidance", () => {
  it("config setup shows Docker instructions", async () => {
    const { stdout, code } = await run(["config", "setup"]);
    expect(code).toBe(0);
    expect(stdout).toContain("docker");
    expect(stdout).toContain("qdrant");
    expect(stdout).toContain("6334");
  });

  it("config setup shows native binary option", async () => {
    const { stdout, code } = await run(["config", "setup"]);
    expect(code).toBe(0);
    expect(stdout).toContain("Native binary");
    expect(stdout).toContain("github.com/qdrant/qdrant/releases");
  });

  it("config setup --detect reports qdrant binary status", async () => {
    const { stdout } = await run(["config", "setup", "--detect"]);
    // Either finds qdrant or reports not found — both are valid
    expect(stdout).toMatch(/qdrant binary|not found|found at/i);
  });

  it("config show includes qdrant_url", async () => {
    const { stdout, code } = await run(["config", "show"]);
    expect(code).toBe(0);
    expect(stdout).toContain("qdrant_url");
    expect(stdout).toContain("localhost");
  });
});
