import { describe, it, expect } from "vitest";
import { run } from "./helpers.js";

describe("spin command", () => {
  it("completes with a short duration", async () => {
    const { stdout, code } = await run(["spin", "--duration", "100"]);
    expect(code).toBe(0);
    expect(stdout).toContain("Completed in 100ms");
  });
});
