import { createEngine, loadConfig, saveConfig } from "../core/engine.js";
import { success, error, info, warn } from "../utils/style.js";
import { parsePositiveInt } from "../utils/validate.js";

export function registerStore(program) {
  program
    .command("store")
    .description("Store a memory entry")
    .argument("<content>", "Text content to store")
    .option("-n, --namespace <ns>", "Namespace for the entry")
    .option("-s, --session <id>", "Session ID")
    .option("-t, --tags <tags>", "Comma-separated tags")
    .option("--ttl <seconds>", "Time-to-live in seconds (auto-expires after this duration)")
    .action(async (content, opts) => {
      try {
        const engine = createEngine();
        const options = {};
        if (opts.namespace) options.namespace = opts.namespace;
        if (opts.session) options.sessionId = opts.session;
        if (opts.tags) options.tags = opts.tags.split(",").map((t) => t.trim());
        if (opts.ttl) options.ttlSeconds = parsePositiveInt(opts.ttl, "--ttl");

        // Model versioning check
        const currentModelId = engine.modelId();
        const config = loadConfig();
        if (config.model_id && config.model_id !== currentModelId) {
          const status = engine.status();
          if (status.entryCount > 0) {
            warn(`Embedding model changed: ${config.model_id} -> ${currentModelId}`);
            warn("Existing entries use a different model. Search accuracy may be degraded.");
          }
        }

        const result = await engine.store(content, options);

        if (result.status === "stored") {
          success(`Stored: ${result.id}`);
          // Record current model_id in config
          if (config.model_id !== currentModelId) {
            saveConfig({ ...config, model_id: currentModelId });
          }
        } else {
          info(`Skipped (duplicate): existing ${result.existingId}`);
        }

        await engine.shutdown();
      } catch (err) {
        error(err.message);
        process.exitCode = 1;
      }
    });
}
