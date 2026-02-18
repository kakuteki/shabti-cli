import chalk from "chalk";
import { success, info } from "../utils/style.js";

const COMMANDS = {
  "/help": "Show this help message",
  "/exit": "Exit the REPL",
  "/clear": "Clear conversation history",
  "/model": "Show or switch the model (e.g. /model gpt-4o)",
  "/history": "Show conversation history",
};

/**
 * Handle slash commands in the REPL.
 * Returns true if the input was handled, false otherwise.
 *
 * @param {string} cmd   - The slash command (e.g. "/help")
 * @param {string} args  - Arguments after the command
 * @param {import("../core/session.js").ChatSession} session
 * @param {import("readline").Interface} rl
 * @returns {boolean}
 */
export function handleSlashCommand(cmd, args, session, rl) {
  switch (cmd) {
    case "/help":
      console.log();
      console.log(chalk.bold("  Available commands:\n"));
      for (const [name, desc] of Object.entries(COMMANDS)) {
        console.log(`    ${chalk.cyan(name.padEnd(12))} ${desc}`);
      }
      console.log();
      return true;

    case "/exit":
      console.log(chalk.dim("Bye."));
      rl.close();
      return true;

    case "/clear":
      session.clearHistory();
      success("Conversation cleared.");
      console.log();
      return true;

    case "/model":
      if (args) {
        session.setModel(args);
        success(`Model switched to ${chalk.cyan(args)}`);
      } else {
        info(`Current model: ${chalk.cyan(session.getModel())}`);
      }
      console.log();
      return true;

    case "/history": {
      const history = session.getHistory();
      console.log();
      for (const msg of history) {
        if (msg.role === "system") continue;
        const label = msg.role === "user" ? chalk.green("You: ") : chalk.cyan("shabti: ");
        console.log(`  ${label}${msg.content}`);
      }
      if (history.length <= 1) {
        console.log(chalk.dim("  (no messages yet)"));
      }
      console.log();
      return true;
    }

    default:
      return false;
  }
}
