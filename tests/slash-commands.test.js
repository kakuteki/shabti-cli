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

  it("/exit closes readline", () => {
    const rl = mockRl();
    const handled = handleSlashCommand("/exit", "", mockSession(), rl);
    expect(handled).toBe(true);
    expect(rl.close).toHaveBeenCalledOnce();
  });

  it("/clear resets session history", () => {
    const session = mockSession();
    const handled = handleSlashCommand("/clear", "", session, mockRl());
    expect(handled).toBe(true);
    expect(session.clearHistory).toHaveBeenCalledOnce();
  });

  it("/model without args shows current model", () => {
    const handled = handleSlashCommand("/model", "", mockSession(), mockRl());
    expect(handled).toBe(true);
    const output = logs.join("\n");
    expect(output).toContain("gpt-4o-mini");
  });

  it("/model with args switches model", () => {
    const session = mockSession();
    const handled = handleSlashCommand("/model", "gpt-4o", session, mockRl());
    expect(handled).toBe(true);
    expect(session.setModel).toHaveBeenCalledWith("gpt-4o");
  });

  it("/history shows conversation messages", () => {
    const session = {
      ...mockSession(),
      getHistory: () => [
        { role: "system", content: "sys" },
        { role: "user", content: "hello" },
        { role: "assistant", content: "hi there" },
      ],
    };
    const handled = handleSlashCommand("/history", "", session, mockRl());
    expect(handled).toBe(true);
    const output = logs.join("\n");
    expect(output).toContain("hello");
    expect(output).toContain("hi there");
  });

  it("/history with no messages shows placeholder", () => {
    const session = {
      ...mockSession(),
      getHistory: () => [{ role: "system", content: "sys" }],
    };
    const handled = handleSlashCommand("/history", "", session, mockRl());
    expect(handled).toBe(true);
    const output = logs.join("\n");
    expect(output).toContain("no messages yet");
  });

  it("unknown command returns false", () => {
    const handled = handleSlashCommand("/unknown", "", mockSession(), mockRl());
    expect(handled).toBe(false);
  });

  it("/recall without engine shows warning", async () => {
    const handled = await handleSlashCommand("/recall", "test", mockSession(), mockRl(), null);
    expect(handled).toBe(true);
    const output = logs.join("\n");
    expect(output).toContain("not available");
  });

  it("/recall with no results shows placeholder", async () => {
    const engine = { ...mockEngine(), executeQuery: vi.fn().mockResolvedValue([]) };
    const handled = await handleSlashCommand("/recall", "nothing", mockSession(), mockRl(), engine);
    expect(handled).toBe(true);
    const output = logs.join("\n");
    expect(output).toContain("No memories found");
  });

  it("/remember handles store error", async () => {
    const engine = {
      ...mockEngine(),
      store: vi.fn().mockRejectedValue(new Error("store failed")),
    };
    const handled = await handleSlashCommand("/remember", "test", mockSession(), mockRl(), engine);
    expect(handled).toBe(true);
    const output = logs.join("\n");
    expect(output).toContain("store failed");
  });

  it("/remember handles duplicate result", async () => {
    const engine = {
      ...mockEngine(),
      store: vi.fn().mockResolvedValue({ status: "duplicate", existingId: "dup-id" }),
    };
    const handled = await handleSlashCommand("/remember", "test", mockSession(), mockRl(), engine);
    expect(handled).toBe(true);
    const output = logs.join("\n");
    expect(output).toContain("duplicate");
  });

  it("/recall handles search error", async () => {
    const engine = {
      ...mockEngine(),
      executeQuery: vi.fn().mockRejectedValue(new Error("search failed")),
    };
    const handled = await handleSlashCommand("/recall", "test", mockSession(), mockRl(), engine);
    expect(handled).toBe(true);
    const output = logs.join("\n");
    expect(output).toContain("search failed");
  });

  it("/gc without engine shows warning", async () => {
    const handled = await handleSlashCommand("/gc", "", mockSession(), mockRl(), null);
    expect(handled).toBe(true);
    const output = logs.join("\n");
    expect(output).toContain("not available");
  });

  it("/gc runs garbage collection", async () => {
    const engine = { ...mockEngine(), gc: vi.fn().mockResolvedValue(3) };
    const handled = await handleSlashCommand("/gc", "", mockSession(), mockRl(), engine);
    expect(handled).toBe(true);
    const output = logs.join("\n");
    expect(output).toContain("3");
  });

  it("/gc handles error", async () => {
    const engine = { ...mockEngine(), gc: vi.fn().mockRejectedValue(new Error("gc failed")) };
    const handled = await handleSlashCommand("/gc", "", mockSession(), mockRl(), engine);
    expect(handled).toBe(true);
    const output = logs.join("\n");
    expect(output).toContain("gc failed");
  });
});
