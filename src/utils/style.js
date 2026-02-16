import chalk from "chalk";

export const success = (msg) => console.log(chalk.green("✔ ") + msg);
export const error = (msg) => console.log(chalk.red("✖ ") + msg);
export const info = (msg) => console.log(chalk.cyan("ℹ ") + msg);
export const warn = (msg) => console.log(chalk.yellow("⚠ ") + msg);
export const heading = (msg) => console.log(chalk.bold.underline(msg));
