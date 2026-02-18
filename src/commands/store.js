import { createEngine } from "../core/engine.js";
import { success, error, info } from "../utils/style.js";

export function registerStore(program) {
  program
    .command("store")
    .description("Store a memory entry")
    .argument("<content>", "Text content to store")
    .option("-n, --namespace <ns>", "Namespace for the entry")
    .option("-s, --session <id>", "Session ID")
    .option("-t, --tags <tags>", "Comma-separated tags")
    .action(async (content, opts) => {
      try {
        const engine = createEngine();
        const options = {};
        if (opts.namespace) options.namespace = opts.namespace;
        if (opts.session) options.sessionId = opts.session;
        if (opts.tags) options.tags = opts.tags.split(",").map((t) => t.trim());

        const result = await engine.store(content, options);

        if (result.status === "stored") {
          success(`Stored: ${result.id}`);
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
