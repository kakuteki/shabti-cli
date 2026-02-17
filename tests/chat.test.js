import { describe, it, expect } from "vitest";
import { execFile } from "child_process";
import { promisify } from "util";

const run = promisify(execFile);
const CLI = "node";
const ENTRY = "src/index.js";

describe("shabti chat", () => {
  it("should show chat in help output", async () => {
    const { stdout } = await run(CLI, [ENTRY, "--help"], {
      env: { ...process.env, FORCE_COLOR: "0" },
    });
    expect(stdout).toContain("chat");
    expect(stdout).toContain("Start an interactive chat session with GPT");
  });

  it("should show chat-specific help", async () => {
    const { stdout } = await run(CLI, [ENTRY, "chat", "--help"], {
      env: { ...process.env, FORCE_COLOR: "0" },
    });
    expect(stdout).toContain("--model");
    expect(stdout).toContain("--system");
    expect(stdout).toContain("gpt-4o-mini");
  });

  it("should error when OPENAI_API_KEY is missing", async () => {
    try {
      await run(CLI, [ENTRY, "chat"], {
        env: { ...process.env, OPENAI_API_KEY: "your-api-key-here", FORCE_COLOR: "0" },
        timeout: 3000,
      });
    } catch (err) {
      expect(err.stderr + err.stdout).toContain("OPENAI_API_KEY is not set");
    }
  });
});
