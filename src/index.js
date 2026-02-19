#!/usr/bin/env node

import { readFileSync } from "fs";
import chalk from "chalk";
import { Command } from "commander";
import { registerChat } from "./commands/chat.js";
import { registerConfig } from "./commands/config.js";
import { registerHello } from "./commands/hello.js";
import { registerList } from "./commands/list.js";
import { registerSearch } from "./commands/search.js";
import { registerSnapshot } from "./commands/snapshot.js";
import { registerSpin } from "./commands/spin.js";
import { registerStatus } from "./commands/status.js";
import { registerStore } from "./commands/store.js";

const { version } = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));

const BANNER = `
${chalk.cyan.bold(`  ____  _           _     _   _ `)}            ${chalk.yellow(`.%/\\`)}
${chalk.cyan.bold(` / ___|| |__   __ _| |__ | |_(_)`)}          ${chalk.yellow(`.%./  &.`)}
${chalk.cyan.bold(` \\___ \\| '_ \\ / _\` | '_ \\| __| |`)}        ${chalk.yellow(`.%**/     \\`)}
${chalk.cyan.bold(`  ___) | | | | (_| | |_) | |_| |`)}      ${chalk.yellow(`.%***/       &.`)}
${chalk.cyan.bold(` |____/|_| |_|\\__,_|_.__/ \\__|_|`)}    ${chalk.yellow(`.%.***/  .d99b_  \\`)}
                                  ${chalk.yellow(`.%*****/ -'      \`'.&.`)}
                                ${chalk.yellow(`.%******/  ._."""'~::,  \\`)}
                              ${chalk.yellow(`.%*******/._'\` .'"^':,  :.,&.`)}
                            ${chalk.yellow(`.%********/',_-^{  ( )  }    :.\\`)}
                          ${chalk.yellow(`.%*********/%^    '.     .'     ;.&.`)}
                        ${chalk.yellow(`.%**********/;        ".,."        ;#.\\`)}
                      ${chalk.yellow(`.%***********/  ~'.,,.          ,.-'^    &.`)}
                    ${chalk.yellow(`.%************/         ""-.,.-""~           \\`)}
                  ${chalk.yellow(`.%*************/                                &.`)}
                ${chalk.yellow(`%**************/                                   \\`)}

  ${chalk.dim(`v${version}  —  "I shall do it. Here I am."`)}
`;

const args = process.argv.slice(2);
const isNoArgs = args.length === 0;
const isHelp = args.includes("--help") || args.includes("-h");

// Show banner for help or no-args
if (isHelp || isNoArgs) {
  console.log(BANNER);
}

// Try launching REPL when no args and running in an interactive terminal
if (isNoArgs && process.stdin.isTTY) {
  const { launchRepl } = await import("./repl/index.js");
  const result = await launchRepl();
  if (result !== null) {
    // REPL started successfully — nothing more to do
    // (process.exit is handled inside ChatSession)
  } else {
    // API key not set — fall back to showing help below
    showHelp();
  }
} else if (isNoArgs) {
  // Non-TTY (piped / subprocess) — show help
  showHelp();
} else {
  // Normal command parsing
  buildProgram().parse();
}

function buildProgram() {
  const program = new Command();
  program
    .name("shabti")
    .description("Agent Memory OS — semantic memory for AI agents")
    .version(version);
  registerChat(program);
  registerConfig(program);
  registerHello(program);
  registerList(program);
  registerSearch(program);
  registerSnapshot(program);
  registerSpin(program);
  registerStatus(program);
  registerStore(program);

  program
    .command("mcp-config")
    .description("Print MCP server configuration JSON for Claude Code / Cursor")
    .action(() => {
      const config = {
        mcpServers: {
          "shabti-memory": {
            command: "npx",
            args: ["shabti-mcp"],
          },
        },
      };
      console.log(JSON.stringify(config, null, 2));
    });

  return program;
}

function showHelp() {
  buildProgram().outputHelp();
}
