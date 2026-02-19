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

  it("config show includes qdrant_url", async () => {
    const { stdout, code } = await run(["config", "show"]);
    expect(code).toBe(0);
    expect(stdout).toContain("qdrant_url");
    expect(stdout).toContain("localhost");
  });
});
