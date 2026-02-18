import chalk from "chalk";
import { error, info } from "../utils/style.js";
import { ChatSession } from "../core/session.js";

const SYSTEM_PROMPT = `You are Shabti, a helpful assistant accessed via CLI. Be concise and direct.`;

export function registerChat(program) {
  program
    .command("chat")
    .description("Start an interactive chat session with GPT")
    .option("-m, --model <model>", "OpenAI model to use", "gpt-4o-mini")
    .option("-s, --system <prompt>", "Custom system prompt", SYSTEM_PROMPT)
    .action(async (opts) => {
      await import("dotenv/config");
      const apiKey = process.env.OPENAI_API_KEY;
      if (!apiKey || apiKey === "your-api-key-here") {
        error("OPENAI_API_KEY is not set. Add it to .env in the project root.");
        process.exit(1);
      }

      console.log();
      info(`shabti chat — model: ${chalk.cyan(opts.model)}`);
      console.log(chalk.dim("  Type your message. Ctrl+C or 'exit' to quit.\n"));

      const session = new ChatSession({
        apiKey,
        model: opts.model,
        systemPrompt: opts.system,
        promptPrefix: "You: ",
        onSlashCommand: null,
        legacyExitWord: true,
      });

      await session.start();
    });
}
