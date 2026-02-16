import { describe, it, expect } from "vitest";
import { run } from "./helpers.js";

describe("list command", () => {
  it("outputs a table by default", async () => {
    const { stdout, code } = await run(["list"]);
    expect(code).toBe(0);
    expect(stdout).toContain("Task List");
    expect(stdout).toContain("Build CLI");
  });

  it("outputs JSON with --json", async () => {
    const { stdout, code } = await run(["list", "--json"]);
    expect(code).toBe(0);
    const data = JSON.parse(stdout);
    expect(Array.isArray(data)).toBe(true);
    expect(data.length).toBeGreaterThan(0);
    expect(data[0]).toHaveProperty("name");
  });

  it("filters by status with --filter", async () => {
    const { stdout, code } = await run(["list", "--json", "--filter", "done"]);
    expect(code).toBe(0);
    const data = JSON.parse(stdout);
    expect(data.every((d) => d.status === "done")).toBe(true);
  });
});
