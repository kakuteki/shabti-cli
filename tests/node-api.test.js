import { describe, it, expect } from "vitest";
import { readFileSync, existsSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

describe("Node.js API exports", () => {
  it("package.json has main field pointing to native.cjs", () => {
    const pkg = JSON.parse(readFileSync(resolve(ROOT, "package.json"), "utf8"));
    expect(pkg.main).toBe("native.cjs");
  });

  it("package.json has types field pointing to native.d.ts", () => {
    const pkg = JSON.parse(readFileSync(resolve(ROOT, "package.json"), "utf8"));
    expect(pkg.types).toBe("native.d.ts");
  });

  it("native.cjs exists and exports ShabtiEngine", () => {
    const nativePath = resolve(ROOT, "native.cjs");
    expect(existsSync(nativePath)).toBe(true);

    const native = require(nativePath);
    expect(native).toHaveProperty("ShabtiEngine");
    expect(typeof native.ShabtiEngine).toBe("function");
  });

  it("native.d.ts exists and declares ShabtiEngine", () => {
    const dtsPath = resolve(ROOT, "native.d.ts");
    expect(existsSync(dtsPath)).toBe(true);

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
