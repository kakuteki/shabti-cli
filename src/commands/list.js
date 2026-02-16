import Table from "cli-table3";
import chalk from "chalk";
import { heading } from "../utils/style.js";

const SAMPLE_DATA = [
  { id: 1, name: "Build CLI", status: "done", priority: "high" },
  { id: 2, name: "Write tests", status: "in-progress", priority: "mid" },
  { id: 3, name: "Publish npm", status: "pending", priority: "high" },
  { id: 4, name: "Add apt repo", status: "pending", priority: "low" },
];

const STATUS_COLOR = {
  done: chalk.green,
  "in-progress": chalk.yellow,
  pending: chalk.gray,
};

export function registerList(program) {
  program
    .command("list")
    .description("Show sample task list")
    .option("-j, --json", "Output as JSON")
    .option("-f, --filter <status>", "Filter by status")
    .action((opts) => {
      let data = SAMPLE_DATA;

      if (opts.filter) {
        data = data.filter((d) => d.status === opts.filter);
      }

      if (opts.json) {
        console.log(JSON.stringify(data, null, 2));
        return;
      }

      heading("Task List");
      console.log();

      const table = new Table({
        head: ["ID", "Task", "Status", "Priority"].map((h) => chalk.cyan(h)),
        style: { head: [], border: [] },
      });

      for (const row of data) {
        const colorize = STATUS_COLOR[row.status] || chalk.white;
        table.push([row.id, row.name, colorize(row.status), row.priority]);
      }

      console.log(table.toString());
    });
}
