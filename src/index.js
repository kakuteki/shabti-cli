#!/usr/bin/env node

import { readFileSync } from "fs";
import chalk from "chalk";
import { Command } from "commander";
import { registerChat } from "./commands/chat.js";
import { registerHello } from "./commands/hello.js";
import { registerList } from "./commands/list.js";
import { registerSpin } from "./commands/spin.js";

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
    .description("Demo CLI tool — showcasing npm-publishable CLI structure")
    .version(version);
  registerChat(program);
  registerHello(program);
  registerList(program);
  registerSpin(program);
  return program;
}

function showHelp() {
  buildProgram().outputHelp();
}
