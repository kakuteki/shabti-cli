import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { logger } from "../src/utils/logger.js";

describe("logger", () => {
  let stderrSpy;
  let savedLevel;
  let savedJson;

  beforeEach(() => {
    savedLevel = process.env.SHABTI_LOG_LEVEL;
    savedJson = process.env.SHABTI_LOG_JSON;
    delete process.env.SHABTI_LOG_LEVEL;
    delete process.env.SHABTI_LOG_JSON;
    stderrSpy = vi.spyOn(process.stderr, "write").mockImplementation(() => true);
  });

  afterEach(() => {
    stderrSpy.mockRestore();
    if (savedLevel !== undefined) process.env.SHABTI_LOG_LEVEL = savedLevel;
    else delete process.env.SHABTI_LOG_LEVEL;
    if (savedJson !== undefined) process.env.SHABTI_LOG_JSON = savedJson;
    else delete process.env.SHABTI_LOG_JSON;
  });

  it("logs info messages by default", () => {
    logger.info("test message");
    expect(stderrSpy).toHaveBeenCalledOnce();
    const output = stderrSpy.mock.calls[0][0];
    expect(output).toContain("[INFO]");
    expect(output).toContain("test message");
  });

  it("suppresses debug when level is info", () => {
    process.env.SHABTI_LOG_LEVEL = "info";
    logger.debug("hidden");
    expect(stderrSpy).not.toHaveBeenCalled();
  });

  it("shows debug when level is debug", () => {
    process.env.SHABTI_LOG_LEVEL = "debug";
    logger.debug("visible");
    expect(stderrSpy).toHaveBeenCalledOnce();
    expect(stderrSpy.mock.calls[0][0]).toContain("[DEBUG]");
  });

  it("outputs JSON when SHABTI_LOG_JSON=1", () => {
    process.env.SHABTI_LOG_JSON = "1";
    logger.info("json test", { key: "val" });
    const output = stderrSpy.mock.calls[0][0];
    const parsed = JSON.parse(output.trim());
    expect(parsed.level).toBe("info");
    expect(parsed.message).toBe("json test");
    expect(parsed.data.key).toBe("val");
    expect(parsed.timestamp).toBeDefined();
  });

  it("includes data in text mode", () => {
    logger.warn("warning", { count: 5 });
    const output = stderrSpy.mock.calls[0][0];
    expect(output).toContain("[WARN]");
    expect(output).toContain('"count":5');
  });

  it("suppresses all logs when level is silent", () => {
    process.env.SHABTI_LOG_LEVEL = "silent";
    logger.error("should not appear");
    expect(stderrSpy).not.toHaveBeenCalled();
  });

  it("logs error level messages", () => {
    logger.error("something broke");
    expect(stderrSpy).toHaveBeenCalledOnce();
    expect(stderrSpy.mock.calls[0][0]).toContain("[ERROR]");
  });

  it("includes ISO timestamp", () => {
    logger.info("ts test");
    const output = stderrSpy.mock.calls[0][0];
    // ISO timestamp pattern: 2024-01-01T00:00:00.000Z
    expect(output).toMatch(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);
  });
});
