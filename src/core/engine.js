import { readFileSync, existsSync, mkdirSync, writeFileSync } from "fs";
import { createRequire } from "module";
import { homedir } from "os";
import { join } from "path";

const require = createRequire(import.meta.url);

const CONFIG_DIR = join(homedir(), ".shabti");
const CONFIG_PATH = join(CONFIG_DIR, "config.json");

const DEFAULT_CONFIG = {
  qdrant_url: "http://localhost:6334",
  data_dir: join(CONFIG_DIR, "data"),
  collection_name: "shabti",
};

/** Env var overrides (highest priority). */
function envOverrides() {
  const o = {};
  if (process.env.SHABTI_QDRANT_URL) o.qdrant_url = process.env.SHABTI_QDRANT_URL;
  return o;
}

export function loadConfig() {
  let config = DEFAULT_CONFIG;
  if (existsSync(CONFIG_PATH)) {
    try {
      config = { ...config, ...JSON.parse(readFileSync(CONFIG_PATH, "utf8")) };
    } catch (_) {
      // keep defaults
    }
  }
  return { ...config, ...envOverrides() };
}

export function saveConfig(config) {
  mkdirSync(CONFIG_DIR, { recursive: true, mode: 0o700 });
  writeFileSync(CONFIG_PATH, JSON.stringify(config, null, 2), { mode: 0o600 });
}

export function createEngine(configOverrides = {}) {
  const config = { ...loadConfig(), ...configOverrides };
  mkdirSync(config.data_dir, { recursive: true, mode: 0o700 });

  const { ShabtiEngine } = require("../../native.cjs");

  return new ShabtiEngine({
    qdrantUrl: config.qdrant_url,
    collectionName: config.collection_name,
    dataDir: config.data_dir,
  });
}

export { CONFIG_DIR, CONFIG_PATH, DEFAULT_CONFIG };
