import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { loadConfig, saveConfig, CONFIG_PATH, CONFIG_DIR } from "../src/core/engine.js";

describe("loadConfig", () => {
  let savedEnv;

  beforeEach(() => {
    savedEnv = process.env.SHABTI_QDRANT_URL;
    delete process.env.SHABTI_QDRANT_URL;
  });

  afterEach(() => {
    if (savedEnv !== undefined) process.env.SHABTI_QDRANT_URL = savedEnv;
    else delete process.env.SHABTI_QDRANT_URL;
  });

  it("returns default config", () => {
    const config = loadConfig();
    expect(config).toHaveProperty("qdrant_url");
    expect(config).toHaveProperty("data_dir");
    expect(config).toHaveProperty("collection_name");
    expect(config.collection_name).toBe("shabti");
  });

  it("respects SHABTI_QDRANT_URL env var", () => {
    process.env.SHABTI_QDRANT_URL = "http://custom:1234";
    const config = loadConfig();
    expect(config.qdrant_url).toBe("http://custom:1234");
  });

  it("exports CONFIG_PATH and CONFIG_DIR", () => {
    expect(CONFIG_PATH).toContain(".shabti");
    expect(CONFIG_DIR).toContain(".shabti");
  });
});

describe("saveConfig", () => {
  it("is a callable function", () => {
    expect(typeof saveConfig).toBe("function");
  });
});
