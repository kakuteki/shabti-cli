import chalk from "chalk";
import { info, warn } from "../utils/style.js";
import { ChatSession } from "../core/session.js";
import { handleSlashCommand } from "./slashCommands.js";

/**
 * Launch the interactive REPL session.
 * Returns null if the API key is not configured (caller should fall back to help).
 */
export async function launchRepl() {
  await import("dotenv/config");

  const apiKey = process.env.OPENAI_API_KEY;
  if (!apiKey || apiKey === "your-api-key-here") {
    warn("OPENAI_API_KEY is not set. Add it to .env to use interactive mode.");
    return null;
  }

  const model = "gpt-4o-mini";

  console.log();
  info(`Interactive mode — model: ${chalk.cyan(model)}`);
  console.log(chalk.dim("  Type a message, or /help for commands. Ctrl+C or /exit to quit.\n"));

  const session = new ChatSession({
    apiKey,
    model,
    promptPrefix: "shabti> ",
    onSlashCommand: handleSlashCommand,
    legacyExitWord: false,
  });

  await session.start();
}
