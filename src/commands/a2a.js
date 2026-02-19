import { startA2AServer } from "../a2a/server.js";

export function registerA2A(program) {
  program
    .command("a2a")
    .description("Start the A2A (Agent-to-Agent) protocol server")
    .option("-p, --port <port>", "Port to listen on", "3000")
    .action((opts) => {
      const port = parseInt(opts.port, 10);
      startA2AServer(port);
    });
}
