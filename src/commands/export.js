import { writeFileSync } from "fs";
import { createEngine } from "../core/engine.js";
import { success, error } from "../utils/style.js";

export function registerExport(program) {
  program
    .command("export")
    .description("Export memory entries as JSONL")
    .option("-n, --namespace <ns>", "Filter by namespace")
    .option("--no-embeddings", "Exclude embedding vectors from output")
    .option("-o, --output <file>", "Write to file instead of stdout")
    .option("--limit <n>", "Maximum number of entries to export")
    .action(async (opts) => {
      try {
        const engine = createEngine();
        const listOpts = {};
        if (opts.namespace) listOpts.namespace = opts.namespace;
        if (opts.limit) listOpts.limit = parseInt(opts.limit, 10);
        listOpts.includeEmbeddings = opts.embeddings !== false ? false : false;
        // --no-embeddings sets opts.embeddings = false (commander negatable)
        // default: exclude embeddings for smaller output
        listOpts.includeEmbeddings = opts.embeddings === true;

        const entries = engine.listEntries(listOpts);

        const lines = entries.map((e) => JSON.stringify(e));
        const output = lines.join("\n");

        if (opts.output) {
          writeFileSync(opts.output, output + (lines.length ? "\n" : ""), "utf8");
          success(`Exported ${entries.length} entries to ${opts.output}`);
        } else {
          if (output) console.log(output);
          // Print summary to stderr so it doesn't pollute piped output
          process.stderr.write(`\n${entries.length} entries exported\n`);
        }

        await engine.shutdown();
      } catch (err) {
        error(err.message);
        process.exitCode = 1;
      }
    });
}
