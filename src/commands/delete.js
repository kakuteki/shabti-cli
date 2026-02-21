import { createEngine } from "../core/engine.js";
import { success, error } from "../utils/style.js";

export function registerDelete(program) {
  program
    .command("delete")
    .description("Delete a memory entry by ID")
    .argument("<id>", "UUID of the memory entry to delete")
    .action(async (id) => {
      try {
        const engine = createEngine();
        await engine.delete(id);
        success(`Deleted: ${id}`);
        await engine.shutdown();
      } catch (err) {
        error(err.message);
        process.exitCode = 1;
      }
    });
}
