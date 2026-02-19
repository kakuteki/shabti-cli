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

  // Try to connect memory engine (non-blocking)
  let engine = null;
  try {
    const { createEngine } = await import("../core/engine.js");
    engine = createEngine();
    info(
      `Memory engine connected (${chalk.cyan("/remember")}, ${chalk.cyan("/recall")} available)`,
    );
  } catch {
    warn("Memory engine not available. Chat-only mode (start Qdrant to enable memory).");
  }

  console.log();
  info(`Interactive mode — model: ${chalk.cyan(model)}`);
  console.log(chalk.dim("  Type a message, or /help for commands. Ctrl+C or /exit to quit.\n"));

  const session = new ChatSession({
    apiKey,
    model,
    promptPrefix: "you> ",
    onSlashCommand: (cmd, args, sess, rl) => handleSlashCommand(cmd, args, sess, rl, engine),
    legacyExitWord: false,
  });

  await session.start();
}
