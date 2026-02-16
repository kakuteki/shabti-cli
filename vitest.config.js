import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      include: ["src/**/*.js"],
      reporter: ["text", "lcov"],
      // TODO: Enable thresholds when unit tests (direct module imports) are added.
      // Current tests run CLI as subprocess, so v8 cannot track coverage.
      // thresholds: {
      //   statements: 70,
      //   branches: 70,
      //   functions: 80,
      //   lines: 70,
      // },
    },
  },
});
