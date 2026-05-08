import { readFileSync, existsSync } from "fs";
import { resolve } from "path";
import { createEngine } from "../core/engine.js";
import { success, error, info, warn } from "../utils/style.js";

export function registerImport(program) {
  program
    .command("import")
    .description("Import memory entries from a JSONL file")
    .argument("<file>", "Path to JSONL file")
    .option("-n, --namespace <ns>", "Override namespace for all imported entries")
    .option("--dry-run", "Parse and validate without storing")
    .action(async (file, opts) => {
      try {
        const absFile = resolve(file);
        if (!existsSync(absFile)) {
          error(`File not found: ${absFile}`);
          process.exitCode = 1;
          return;
        }
        const raw = readFileSync(absFile, "utf8");
        const lines = raw
          .split("\n")
          .map((l) => l.trim())
          .filter((l) => l.length > 0);

        if (lines.length === 0) {
          info("No entries found in file.");
          return;
        }

        // Parse all lines first to validate
        const entries = [];
        for (let i = 0; i < lines.length; i++) {
          try {
            entries.push(JSON.parse(lines[i]));
          } catch (_) {
            warn(`Skipping invalid JSON on line ${i + 1}`);
          }
        }

        if (opts.dryRun) {
          info(`Dry run: ${entries.length} entries parsed, 0 stored.`);
          return;
        }

        const engine = createEngine();
        let stored = 0;
        let skipped = 0;

        for (const entry of entries) {
          const storeOpts = {};
          storeOpts.namespace = opts.namespace || entry.namespace || undefined;
          if (entry.tags && entry.tags.length) storeOpts.tags = entry.tags;
          if (entry.sessionId || entry.session_id)
            storeOpts.sessionId = entry.sessionId || entry.session_id;

          const result = await engine.store(entry.content, storeOpts);
          if (result.status === "stored") {
            stored++;
          } else {
            skipped++;
          }
        }

        success(`Import complete: ${stored} stored, ${skipped} skipped (duplicates)`);
        await engine.shutdown();
      } catch (err) {
        error(err.message);
        process.exitCode = 1;
      }
    });
}
