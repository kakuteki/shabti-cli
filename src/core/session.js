import { createInterface } from "readline";
import OpenAI from "openai";
import chalk from "chalk";
import { error } from "../utils/style.js";

const DEFAULT_SYSTEM_PROMPT =
  "You are Shabti, a helpful assistant accessed via CLI. Be concise and direct.";

export class ChatSession {
  #client;
  #messages;
  #model;
  #promptPrefix;
  #onSlashCommand;
  #legacyExitWord;

  /**
   * @param {object} opts
   * @param {string} opts.apiKey
   * @param {string} [opts.model]
   * @param {string} [opts.systemPrompt]
   * @param {string} [opts.promptPrefix]
   * @param {((cmd: string, args: string, session: ChatSession) => boolean) | null} [opts.onSlashCommand]
   * @param {boolean} [opts.legacyExitWord]
   */
  constructor({
    apiKey,
    model = "gpt-4o-mini",
    systemPrompt = DEFAULT_SYSTEM_PROMPT,
    promptPrefix = "You: ",
    onSlashCommand = null,
    legacyExitWord = false,
  }) {
    this.#client = new OpenAI({ apiKey });
    this.#model = model;
    this.#promptPrefix = promptPrefix;
    this.#onSlashCommand = onSlashCommand;
    this.#legacyExitWord = legacyExitWord;
    this.#messages = [{ role: "system", content: systemPrompt }];
  }

  getHistory() {
    return [...this.#messages];
  }

  clearHistory() {
    const system = this.#messages[0];
    this.#messages = [system];
  }

  setModel(model) {
    this.#model = model;
  }

  getModel() {
    return this.#model;
  }

  async start() {
    const rl = createInterface({
      input: process.stdin,
      output: process.stdout,
    });

    const prompt = () => {
      rl.question(chalk.green(this.#promptPrefix), async (input) => {
        const trimmed = input.trim();
        if (!trimmed) return prompt();

        // Legacy "exit" word support (chat command compatibility)
        if (this.#legacyExitWord && trimmed.toLowerCase() === "exit") {
          console.log(chalk.dim("\nBye.\n"));
          rl.close();
          return;
        }

        // Slash command handling
        if (trimmed.startsWith("/") && this.#onSlashCommand) {
          const spaceIdx = trimmed.indexOf(" ");
          const cmd = spaceIdx === -1 ? trimmed : trimmed.slice(0, spaceIdx);
          const args = spaceIdx === -1 ? "" : trimmed.slice(spaceIdx + 1).trim();
          const handled = this.#onSlashCommand(cmd, args, this, rl);
          if (handled) return prompt();
        }

        this.#messages.push({ role: "user", content: trimmed });

        try {
          process.stdout.write(chalk.cyan("shabti: "));

          const stream = await this.#client.chat.completions.create({
            model: this.#model,
            messages: this.#messages,
            stream: true,
          });

          let reply = "";
          for await (const chunk of stream) {
            const delta = chunk.choices[0]?.delta?.content || "";
            reply += delta;
            process.stdout.write(delta);
          }
          console.log("\n");

          this.#messages.push({ role: "assistant", content: reply });
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
  }
}
