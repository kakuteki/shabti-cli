import { readFileSync, existsSync, mkdirSync, writeFileSync } from "fs";
import { homedir } from "os";
import { join } from "path";

const CONFIG_DIR = join(homedir(), ".shabti");
const CONFIG_PATH = join(CONFIG_DIR, "config.json");

const DEFAULT_CONFIG = {
  qdrant_url: "http://localhost:6334",
  data_dir: join(CONFIG_DIR, "data"),
  collection_name: "shabti",
};

export function loadConfig() {
  if (existsSync(CONFIG_PATH)) {
    try {
      return { ...DEFAULT_CONFIG, ...JSON.parse(readFileSync(CONFIG_PATH, "utf8")) };
    } catch {
      return DEFAULT_CONFIG;
    }
  }
  return DEFAULT_CONFIG;
}

export function saveConfig(config) {
  mkdirSync(CONFIG_DIR, { recursive: true });
  writeFileSync(CONFIG_PATH, JSON.stringify(config, null, 2));
}

export function createEngine(configOverrides = {}) {
  const config = { ...loadConfig(), ...configOverrides };
  mkdirSync(config.data_dir, { recursive: true });

  const { ShabtiEngine } = require("../../native.cjs");

  return new ShabtiEngine({
    qdrantUrl: config.qdrant_url,
    collectionName: config.collection_name,
    dataDir: config.data_dir,
  });
}

export { CONFIG_DIR, CONFIG_PATH, DEFAULT_CONFIG };
