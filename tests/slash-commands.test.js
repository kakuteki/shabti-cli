import { describe, it, expect, vi, beforeEach } from "vitest";
import { handleSlashCommand } from "../src/repl/slashCommands.js";

/** Minimal mock ChatSession */
function mockSession() {
  return {
    getHistory: () => [{ role: "system", content: "sys" }],
    clearHistory: vi.fn(),
    setModel: vi.fn(),
    getModel: () => "gpt-4o-mini",
  };
}

/** Minimal mock readline interface */
function mockRl() {
  return { close: vi.fn() };
}

/** Minimal mock memory engine */
function mockEngine() {
  return {
    store: vi.fn().mockResolvedValue({ status: "stored", id: "test-id" }),
    executeQuery: vi
      .fn()
      .mockResolvedValue([
        { content: "Tokyo is the capital of Japan", score: 0.92, namespace: "default" },
      ]),
    shutdown: vi.fn().mockResolvedValue(undefined),
  };
}

describe("slash commands", () => {
  let logs;

  beforeEach(() => {
    logs = [];
    vi.spyOn(console, "log").mockImplementation((...args) => logs.push(args.join(" ")));
  });

  it("/help lists all commands", () => {
    const handled = handleSlashCommand("/help", "", mockSession(), mockRl());
    expect(handled).toBe(true);
    const output = logs.join("\n");
    expect(output).toContain("/help");
    expect(output).toContain("/exit");
    expect(output).toContain("/remember");
    expect(output).toContain("/recall");
    expect(output).toContain("/gc");
  });

  it("/remember stores content via engine", async () => {
    const engine = mockEngine();
    const handled = await handleSlashCommand(
      "/remember",
      "Tokyo is the capital of Japan",
      mockSession(),
      mockRl(),
      engine,
    );
    expect(handled).toBe(true);
    expect(engine.store).toHaveBeenCalledWith("Tokyo is the capital of Japan", {});
  });

  it("/remember without text shows usage", async () => {
    const engine = mockEngine();
    const handled = await handleSlashCommand("/remember", "", mockSession(), mockRl(), engine);
    expect(handled).toBe(true);
    expect(engine.store).not.toHaveBeenCalled();
    const output = logs.join("\n");
    expect(output).toContain("Usage");
  });

  it("/recall searches via engine", async () => {
    const engine = mockEngine();
    const handled = await handleSlashCommand(
      "/recall",
      "capital of Japan",
      mockSession(),
      mockRl(),
      engine,
    );
    expect(handled).toBe(true);
    expect(engine.executeQuery).toHaveBeenCalled();
    const output = logs.join("\n");
    expect(output).toContain("Tokyo");
  });

  it("/recall without query shows usage", async () => {
    const engine = mockEngine();
    const handled = await handleSlashCommand("/recall", "", mockSession(), mockRl(), engine);
    expect(handled).toBe(true);
    expect(engine.executeQuery).not.toHaveBeenCalled();
  });

  it("/remember without engine shows warning", async () => {
    const handled = await handleSlashCommand("/remember", "test", mockSession(), mockRl(), null);
    expect(handled).toBe(true);
    const output = logs.join("\n");
    expect(output).toContain("not available");
  });
});
