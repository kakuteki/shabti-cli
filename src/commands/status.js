import chalk from "chalk";
import { createEngine } from "../core/engine.js";
import { error, heading } from "../utils/style.js";

export function registerStatus(program) {
  program
    .command("status")
    .description("Show engine status")
    .option("-j, --json", "Output as JSON")
    .action(async (opts) => {
      try {
        const engine = createEngine();
        const status = engine.status();

        if (opts.json) {
          console.log(JSON.stringify(status, null, 2));
        } else {
          heading("Shabti Status");
          console.log();
          console.log(`  ${chalk.cyan("Entries:")}     ${status.entryCount}`);
          console.log(`  ${chalk.cyan("Tier:")}        ${status.tier}`);
          console.log(`  ${chalk.cyan("Qdrant URL:")}  ${status.qdrantUrl}`);
          console.log(`  ${chalk.cyan("Data Dir:")}    ${status.dataDir}`);
          console.log();
        }

        await engine.shutdown();
      } catch (err) {
        error(err.message);
        process.exitCode = 1;
      }
    });
}
