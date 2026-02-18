import chalk from "chalk";

export const success = (msg) => console.log(chalk.green("[ok] ") + msg);
export const error = (msg) => console.log(chalk.red("[error] ") + msg);
export const info = (msg) => console.log(chalk.cyan("[info] ") + msg);
export const warn = (msg) => console.log(chalk.yellow("[warn] ") + msg);
export const heading = (msg) => console.log(chalk.bold.underline(msg));
