import { execFile } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const CLI = resolve(__dirname, "..", "src", "index.js");

/**
 * Run the CLI as a subprocess and return { stdout, stderr, code }.
 * FORCE_COLOR=0 disables chalk colouring so assertions use plain text.
 */
export function run(args = []) {
  return new Promise((resolve) => {
    execFile(
      "node",
      [CLI, ...args],
      {
        env: { ...process.env, FORCE_COLOR: "0" },
        timeout: 10_000,
      },
      (err, stdout, stderr) => {
        resolve({
          stdout: stdout.toString(),
          stderr: stderr.toString(),
          code: err ? (err.code ?? 1) : 0,
        });
      },
    );
  });
}
