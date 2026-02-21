import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    testTimeout: 15_000,
    exclude: ["node_modules/**", "poc/**"],
    coverage: {
      provider: "v8",
      include: ["src/**/*.js"],
      exclude: ["src/index.js", "src/mcp/server.js", "src/a2a/standalone.js"],
      reporter: ["text", "lcov", "json-summary"],
      thresholds: {
        lines: 35,
        branches: 40,
        functions: 35,
      },
    },
  },
});
