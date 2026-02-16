#!/usr/bin/env node

import chalk from "chalk";
import { Command } from "commander";
import { registerHello } from "./commands/hello.js";
import { registerList } from "./commands/list.js";
import { registerSpin } from "./commands/spin.js";

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

  ${chalk.dim(`v1.0.0  —  "I shall do it. Here I am."`)}
`;

// Show banner when run without subcommand, or with --help
const args = process.argv.slice(2);
const isHelp = args.includes("--help") || args.includes("-h") || args.length === 0;
if (isHelp) {
  console.log(BANNER);
}

const program = new Command();

program
  .name("shabti")
  .description("Demo CLI tool — showcasing npm-publishable CLI structure")
  .version("1.0.0");

// Register commands
registerHello(program);
registerList(program);
registerSpin(program);

program.parse();
