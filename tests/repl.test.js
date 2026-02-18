import { describe, it, expect } from "vitest";
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
