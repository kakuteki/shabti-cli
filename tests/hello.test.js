import { describe, it, expect } from "vitest";
import { run } from "./helpers.js";

describe("hello command", () => {
  it("greets the given name", async () => {
    const { stdout, code } = await run(["hello", "World"]);
    expect(code).toBe(0);
    expect(stdout).toContain("Hello, World!");
  });

  it("shouts with --shout", async () => {
    const { stdout, code } = await run(["hello", "World", "--shout"]);
    expect(code).toBe(0);
    expect(stdout).toContain("HELLO, WORLD!");
  });

  it("fails when no name is provided", async () => {
    const { stderr, code } = await run(["hello"]);
    expect(code).not.toBe(0);
    expect(stderr).toContain("missing required argument");
  });
});
