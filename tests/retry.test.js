import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { createEngineWithRetry } from "../src/core/retry.js";

describe("createEngineWithRetry", () => {
  let stderrSpy;

  beforeEach(() => {
    // Suppress logger output during tests
    stderrSpy = vi.spyOn(process.stderr, "write").mockImplementation(() => true);
  });

  afterEach(() => {
    stderrSpy.mockRestore();
  });

  it("returns engine on first successful attempt", async () => {
    const mockEngine = { status: () => "ok" };
    const factory = vi.fn().mockReturnValue(mockEngine);

    const engine = await createEngineWithRetry(factory, { maxRetries: 3, backoffMs: [10] });

    expect(engine).toBe(mockEngine);
    expect(factory).toHaveBeenCalledOnce();
  });

  it("retries on failure and succeeds", async () => {
    const mockEngine = { status: () => "ok" };
    const factory = vi
      .fn()
      .mockImplementationOnce(() => {
        throw new Error("connection refused");
      })
      .mockImplementationOnce(() => {
        throw new Error("connection refused");
      })
      .mockReturnValue(mockEngine);

    const engine = await createEngineWithRetry(factory, { maxRetries: 3, backoffMs: [10, 10, 10] });

    expect(engine).toBe(mockEngine);
    expect(factory).toHaveBeenCalledTimes(3);
  });

  it("throws after all retries exhausted", async () => {
    const factory = vi.fn().mockImplementation(() => {
      throw new Error("connection refused");
    });

    await expect(
      createEngineWithRetry(factory, { maxRetries: 2, backoffMs: [10, 10] }),
    ).rejects.toThrow("connection refused");

    expect(factory).toHaveBeenCalledTimes(3); // initial + 2 retries
  });

  it("uses default options when not provided", async () => {
    const mockEngine = { status: () => "ok" };
    const factory = vi.fn().mockReturnValue(mockEngine);

    const engine = await createEngineWithRetry(factory);
    expect(engine).toBe(mockEngine);
  });

  it("logs retry attempts", async () => {
    const factory = vi
      .fn()
      .mockImplementationOnce(() => {
        throw new Error("fail");
      })
      .mockReturnValue({ ok: true });

    await createEngineWithRetry(factory, { maxRetries: 2, backoffMs: [10] });

    // Check that logger.warn was called (via stderr)
    const warnCalls = stderrSpy.mock.calls.filter(
      (c) => c[0].includes("WARN") || c[0].includes('"warn"'),
    );
    expect(warnCalls.length).toBeGreaterThanOrEqual(1);
  });
});
