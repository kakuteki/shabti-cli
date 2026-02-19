import ora from "ora";
import { success } from "../utils/style.js";

export function registerSpin(program) {
  program
    .command("spin")
    .description("Demo async operation with spinner (demo)")
    .option("-d, --duration <ms>", "Spinner duration in ms", "2000")
    .action(async (opts) => {
      const ms = parseInt(opts.duration, 10);
      const spinner = ora("Processing...").start();

      await new Promise((resolve) => setTimeout(resolve, ms));

      spinner.succeed("Done!");
      success(`Completed in ${ms}ms`);
    });
}
