import chalk from "chalk";
import { createEngine } from "../core/engine.js";
import { error, heading } from "../utils/style.js";

export function registerGet(program) {
  program
    .command("get")
    .description("Retrieve a memory entry by ID")
    .argument("<id>", "UUID of the memory entry to retrieve")
    .option("-j, --json", "Output as JSON")
    .action(async (id, opts) => {
      try {
        const engine = createEngine();
        const entry = await engine.get(id);

        if (opts.json) {
          console.log(JSON.stringify(entry, null, 2));
        } else {
          heading(`Memory Entry`);
          console.log();
          console.log(`  ${chalk.cyan("ID:")}         ${entry.id}`);
          console.log(`  ${chalk.cyan("Content:")}    ${entry.content}`);
          console.log(`  ${chalk.cyan("Namespace:")}  ${entry.namespace}`);
          console.log(`  ${chalk.cyan("Score:")}      ${entry.score}`);
          console.log(
            `  ${chalk.cyan("Created:")}    ${new Date(entry.createdAt * 1000).toISOString()}`,
          );
          if (entry.expiresAt) {
            console.log(
              `  ${chalk.cyan("Expires:")}    ${new Date(entry.expiresAt * 1000).toISOString()}`,
            );
          }
          console.log();
        }

        await engine.shutdown();
      } catch (err) {
        error(err.message);
        process.exitCode = 1;
      }
    });
}
