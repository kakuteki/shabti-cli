import chalk from "chalk";
import { createEngine, loadConfig } from "../core/engine.js";
import { heading, success, warn } from "../utils/style.js";

export function registerHealth(program) {
  program
    .command("health")
    .description("Run health checks on the shabti engine")
    .option("-j, --json", "Output as JSON")
    .action(async (opts) => {
      const checks = [];

      // 1. Qdrant connectivity
      const config = loadConfig();
      const qdrantUrl = config.qdrant_url.replace(/:\d+$/, ":6333");
      try {
        const res = await fetch(`${qdrantUrl}/healthz`, {
          signal: AbortSignal.timeout(3000),
        });
        checks.push({
          name: "qdrant",
          status: res.ok ? "ok" : "degraded",
          message: res.ok ? "Reachable" : `HTTP ${res.status}`,
        });
      } catch (err) {
        checks.push({
          name: "qdrant",
          status: "error",
          message: err.message,
        });
      }

      // 2. Engine initialization
      let engine = null;
      try {
        engine = createEngine();
        const status = engine.status();
        checks.push({
          name: "engine",
          status: "ok",
          message: `${status.entryCount} entries, tier: ${status.tier}`,
        });
      } catch (err) {
        checks.push({
          name: "engine",
          status: "error",
          message: err.message,
        });
      }

      // 3. Embedding model
      if (engine) {
        try {
          const modelId = engine.modelId();
          checks.push({
            name: "embedding",
            status: "ok",
            message: modelId,
          });
        } catch (err) {
          checks.push({
            name: "embedding",
            status: "error",
            message: err.message,
          });
        }
      }

      const allOk = checks.every((c) => c.status === "ok");

      if (opts.json) {
        console.log(JSON.stringify({ healthy: allOk, checks }, null, 2));
      } else {
        heading("Health Check");
        console.log();
        for (const check of checks) {
          const icon =
            check.status === "ok"
              ? chalk.green("✓")
              : check.status === "degraded"
                ? chalk.yellow("⚠")
                : chalk.red("✗");
          console.log(`  ${icon} ${chalk.cyan(check.name.padEnd(12))} ${check.message}`);
        }
        console.log();
        if (allOk) {
          success("All checks passed");
        } else {
          warn("Some checks failed");
        }
        console.log();
      }

      if (engine) {
        try {
          await engine.shutdown();
        } catch (_) {
          // best-effort
        }
      }

      if (!allOk) process.exitCode = 1;
    });
}
