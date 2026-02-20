import { startA2AServer } from "../a2a/server.js";
import { error } from "../utils/style.js";
import { parsePort } from "../utils/validate.js";

export function registerA2A(program) {
  program
    .command("a2a")
    .description("Start the A2A (Agent-to-Agent) protocol server")
    .option("-p, --port <port>", "Port to listen on", "3000")
    .action((opts) => {
      try {
        const port = parsePort(opts.port, "--port");
        startA2AServer(port);
      } catch (err) {
        error(err.message);
        process.exitCode = 1;
      }
    });
}
