import chalk from "chalk";
import Table from "cli-table3";
import { createEngine } from "../core/engine.js";
import { error, heading } from "../utils/style.js";
import { parsePositiveInt, parseScore } from "../utils/validate.js";

export function registerSearch(program) {
  program
    .command("search")
    .description("Search memory entries")
    .argument("<query>", "Search query text")
    .option("-l, --limit <n>", "Maximum results", "10")
    .option("--namespace <ns>", "Filter by namespace")
    .option("--time-start <ts>", "Time range start (unix timestamp)")
    .option("--time-end <ts>", "Time range end (unix timestamp)")
    .option("--cluster <id>", "Filter by cluster ID")
    .option("--follow-links <hops>", "Expand via graph links (max hops)")
    .option("--min-score <score>", "Minimum score threshold")
    .option("--explain", "Show score explanation")
    .option("--exclude-superseded", "Exclude superseded entries")
    .option("-j, --json", "Output as JSON")
    .action(async (queryText, opts) => {
      try {
        const engine = createEngine();

        const query = { text: queryText };
        query.limit = parsePositiveInt(opts.limit, "--limit");
        if (opts.namespace) query.namespace = opts.namespace;
        if (opts.timeStart) query.timeStart = parsePositiveInt(opts.timeStart, "--time-start");
        if (opts.timeEnd) query.timeEnd = parsePositiveInt(opts.timeEnd, "--time-end");
        if (opts.cluster) query.clusterId = parsePositiveInt(opts.cluster, "--cluster");
        if (opts.followLinks) query.maxHops = parsePositiveInt(opts.followLinks, "--follow-links");
        if (opts.minScore) query.minScore = parseScore(opts.minScore, "--min-score");
        if (opts.explain) query.withExplanation = true;
        if (opts.excludeSuperseded) query.excludeSuperseded = true;

        const results = await engine.executeQuery(query);

        if (opts.json) {
          console.log(JSON.stringify(results, null, 2));
        } else {
          heading(`Search results for "${queryText}" (${results.length} hits)`);
          console.log();

          if (results.length === 0) {
            console.log(chalk.dim("  No results found."));
          } else {
            const table = new Table({
              head: [
                chalk.cyan("Score"),
                chalk.cyan("Content"),
                ...(opts.explain ? [chalk.cyan("Explanation")] : []),
              ],
              colWidths: [10, opts.explain ? 50 : 70, ...(opts.explain ? [30] : [])],
              wordWrap: true,
            });

            for (const r of results) {
              const row = [chalk.yellow(r.score.toFixed(4)), r.content];
              if (opts.explain && r.explanation) {
                const exp = r.explanation;
                row.push(
                  `sim=${exp.semanticSimilarity.toFixed(3)} decay=${exp.timeDecayFactor.toFixed(3)} boost=${exp.accessBoostFactor.toFixed(3)}`,
                );
              }
              table.push(row);
            }

            console.log(table.toString());
          }
        }

        await engine.shutdown();
      } catch (err) {
        error(err.message);
        process.exitCode = 1;
      }
    });
}
