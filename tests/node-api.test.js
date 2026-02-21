import { describe, it, expect } from "vitest";
import { readFileSync, existsSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const HAS_NATIVE = existsSync(resolve(ROOT, "native.cjs"));

describe("Node.js API exports", () => {
  it("package.json has main field pointing to native.cjs", () => {
    const pkg = JSON.parse(readFileSync(resolve(ROOT, "package.json"), "utf8"));
    expect(pkg.main).toBe("native.cjs");
  });

  it("package.json has types field pointing to native.d.ts", () => {
    const pkg = JSON.parse(readFileSync(resolve(ROOT, "package.json"), "utf8"));
    expect(pkg.types).toBe("native.d.ts");
  });

  it.skipIf(!HAS_NATIVE)("native.cjs exists and exports ShabtiEngine", () => {
    const nativePath = resolve(ROOT, "native.cjs");
    const native = require(nativePath);
    expect(native).toHaveProperty("ShabtiEngine");
    expect(typeof native.ShabtiEngine).toBe("function");
  });

  it.skipIf(!HAS_NATIVE)("native.d.ts exists and declares ShabtiEngine", () => {
    const dtsPath = resolve(ROOT, "native.d.ts");
    const content = readFileSync(dtsPath, "utf8");
    expect(content).toContain("ShabtiEngine");
    expect(content).toContain("store");
    expect(content).toContain("search");
    expect(content).toContain("modelId");
  });

  it("package.json files includes native binaries", () => {
    const pkg = JSON.parse(readFileSync(resolve(ROOT, "package.json"), "utf8"));
    expect(pkg.files).toContain("native.cjs");
    expect(pkg.files).toContain("native.d.ts");
    expect(pkg.files).toContain("*.node");
  });
});

describe("Type definitions completeness", () => {
  const dtsPath = resolve(ROOT, "crates", "shabti-napi", "index.d.ts");
  const hasDts = existsSync(dtsPath);

  it.skipIf(!hasDts)("declares all ShabtiEngine methods", () => {
    const content = readFileSync(dtsPath, "utf8");
    const methods = [
      "store",
      "search",
      "executeQuery",
      "get",
      "delete",
      "snapshotCreate",
      "snapshotRestore",
      "snapshotList",
      "modelId",
      "status",
      "listEntries",
      "gc",
      "shutdown",
    ];
    for (const m of methods) {
      expect(content).toContain(m);
    }
  });

  it.skipIf(!hasDts)("declares NapiEngineConfig interface", () => {
    const content = readFileSync(dtsPath, "utf8");
    expect(content).toContain("NapiEngineConfig");
    expect(content).toContain("qdrantUrl");
    expect(content).toContain("dataDir");
  });

  it.skipIf(!hasDts)("declares NapiQuery with all search options", () => {
    const content = readFileSync(dtsPath, "utf8");
    const fields = [
      "text",
      "limit",
      "namespace",
      "timeStart",
      "timeEnd",
      "maxHops",
      "minScore",
      "withExplanation",
    ];
    for (const f of fields) {
      expect(content).toContain(f);
    }
  });

  it.skipIf(!hasDts)("declares NapiStoreOptions with TTL", () => {
    const content = readFileSync(dtsPath, "utf8");
    expect(content).toContain("NapiStoreOptions");
    expect(content).toContain("ttlSeconds");
    expect(content).toContain("tags");
    expect(content).toContain("namespace");
  });

  it.skipIf(!hasDts)("declares NapiExplanation with score breakdown", () => {
    const content = readFileSync(dtsPath, "utf8");
    expect(content).toContain("NapiExplanation");
    expect(content).toContain("semanticSimilarity");
    expect(content).toContain("timeDecayFactor");
    expect(content).toContain("accessBoostFactor");
    expect(content).toContain("finalScore");
  });

  it.skipIf(!hasDts)("declares NapiExportEntry with lifecycle fields", () => {
    const content = readFileSync(dtsPath, "utf8");
    expect(content).toContain("NapiExportEntry");
    expect(content).toContain("lifecycleState");
    expect(content).toContain("originType");
    expect(content).toContain("expiresAt");
  });

  it.skipIf(!hasDts)("declares NapiStatus interface", () => {
    const content = readFileSync(dtsPath, "utf8");
    expect(content).toContain("NapiStatus");
    expect(content).toContain("entryCount");
    expect(content).toContain("tier");
  });
});
