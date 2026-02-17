import "dotenv/config";
import { createInterface } from "readline";
import OpenAI from "openai";
import chalk from "chalk";
import { error, info } from "../utils/style.js";

const SYSTEM_PROMPT = `You are Shabti, a helpful assistant accessed via CLI. Be concise and direct.`;

export function registerChat(program) {
  program
    .command("chat")
    .description("Start an interactive chat session with GPT")
    .option("-m, --model <model>", "OpenAI model to use", "gpt-4o-mini")
    .option("-s, --system <prompt>", "Custom system prompt", SYSTEM_PROMPT)
    .action(async (opts) => {
      const apiKey = process.env.OPENAI_API_KEY;
      if (!apiKey || apiKey === "your-api-key-here") {
        error("OPENAI_API_KEY is not set. Add it to .env in the project root.");
        process.exit(1);
      }

      const client = new OpenAI({ apiKey });
      const messages = [{ role: "system", content: opts.system }];

      console.log();
      info(`shabti chat — model: ${chalk.cyan(opts.model)}`);
      console.log(chalk.dim("  Type your message. Ctrl+C or 'exit' to quit.\n"));

      const rl = createInterface({
        input: process.stdin,
        output: process.stdout,
      });

      const prompt = () => {
        rl.question(chalk.green("You: "), async (input) => {
          const trimmed = input.trim();
          if (!trimmed) return prompt();
          if (trimmed.toLowerCase() === "exit") {
            console.log(chalk.dim("\nBye.\n"));
            rl.close();
            return;
          }

          messages.push({ role: "user", content: trimmed });

          try {
            process.stdout.write(chalk.cyan("shabti: "));

            const stream = await client.chat.completions.create({
              model: opts.model,
              messages,
              stream: true,
            });

            let reply = "";
            for await (const chunk of stream) {
              const delta = chunk.choices[0]?.delta?.content || "";
              reply += delta;
              process.stdout.write(delta);
            }
            console.log("\n");

            messages.push({ role: "assistant", content: reply });
          } catch (err) {
            console.log();
            if (err.status === 401) {
              error("Invalid API key. Check your .env file.");
            } else if (err.status === 429) {
              error("Rate limited. Wait a moment and try again.");
            } else {
              error(`API error: ${err.message}`);
            }
            console.log();
          }

          prompt();
        });
      };

      prompt();

      rl.on("close", () => process.exit(0));
    });
}
