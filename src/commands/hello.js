import chalk from "chalk";
import { success } from "../utils/style.js";

export function registerHello(program) {
  program
    .command("hello")
    .description("Greet someone (demo)")
    .argument("<name>", "Name to greet")
    .option("-s, --shout", "Shout the greeting")
    .action((name, opts) => {
      const msg = `Hello, ${name}!`;
      if (opts.shout) {
        console.log(chalk.bold.magenta(msg.toUpperCase()));
      } else {
        success(msg);
      }
    });
}
