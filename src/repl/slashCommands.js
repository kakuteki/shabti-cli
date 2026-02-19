import chalk from "chalk";
import { success, info, warn, error } from "../utils/style.js";

const COMMANDS = {
  "/help": "Show this help message",
  "/exit": "Exit the REPL",
  "/clear": "Clear conversation history",
  "/model": "Show or switch the model (e.g. /model gpt-4o)",
  "/history": "Show conversation history",
  "/remember": "Store a memory (e.g. /remember Tokyo is the capital of Japan)",
  "/recall": "Search memories (e.g. /recall capital of Japan)",
};

/**
 * Handle slash commands in the REPL.
 * Returns true if the input was handled, false otherwise.
 *
 * @param {string} cmd   - The slash command (e.g. "/help")
 * @param {string} args  - Arguments after the command
 * @param {import("../core/session.js").ChatSession} session
 * @param {import("readline").Interface} rl
 * @param {object|null} [engine] - ShabtiEngine instance (nullable)
 * @returns {boolean|Promise<boolean>}
 */
export function handleSlashCommand(cmd, args, session, rl, engine = null) {
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

    case "/remember":
      return handleRemember(args, engine);

    case "/recall":
      return handleRecall(args, engine);

    default:
      return false;
  }
}

async function handleRemember(text, engine) {
  if (!engine) {
    console.log();
    warn("Memory engine not available. Start Qdrant to enable memory features.");
    console.log();
    return true;
  }
  if (!text) {
    console.log();
    info("Usage: /remember <text to store>");
    console.log();
    return true;
  }
  try {
    const result = await engine.store(text, {});
    if (result.status === "stored") {
      success(`Remembered: ${result.id}`);
    } else {
      info(`Already remembered (duplicate)`);
    }
  } catch (err) {
    error(`Failed to store: ${err.message}`);
  }
  console.log();
  return true;
}

async function handleRecall(query, engine) {
  if (!engine) {
    console.log();
    warn("Memory engine not available. Start Qdrant to enable memory features.");
    console.log();
    return true;
  }
  if (!query) {
    console.log();
    info("Usage: /recall <search query>");
    console.log();
    return true;
  }
  try {
    const results = await engine.executeQuery({ text: query, limit: 5 });
    console.log();
    if (results.length === 0) {
      console.log(chalk.dim("  No memories found."));
    } else {
      for (const r of results) {
        console.log(`  ${chalk.yellow(r.score.toFixed(4))}  ${r.content}`);
      }
    }
  } catch (err) {
    error(`Failed to search: ${err.message}`);
  }
  console.log();
  return true;
}
