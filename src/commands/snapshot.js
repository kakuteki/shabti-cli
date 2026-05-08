import Table from "cli-table3";
import chalk from "chalk";
import { createEngine } from "../core/engine.js";
import { success, error, info } from "../utils/style.js";

export function registerSnapshot(program) {
  const cmd = program.command("snapshot").description("Manage storage snapshots");

  cmd
    .command("create")
    .description("Create a new snapshot")
    .action(async () => {
      try {
        const engine = createEngine();
        const snap = engine.snapshotCreate();
        success(`Snapshot created: ${snap.id}`);
        info(`Timestamp: ${new Date(snap.createdAt * 1000).toISOString()}`);
        await engine.shutdown();
      } catch (err) {
        error(err.message);
        process.exitCode = 1;
      }
    });

  cmd
    .command("restore")
    .description("Restore from a snapshot")
    .argument("<id>", "Snapshot ID to restore")
    .action(async (id) => {
      try {
        // UUIDフォーマット検証
        const UUID_REGEX = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
        if (!UUID_REGEX.test(id)) {
          error(`無効なスナップショットID: ${id}（UUID形式で指定してください）`);
          return;
        }
        const engine = createEngine();
        engine.snapshotRestore(id);
        success(`Restored from snapshot: ${id}`);
        await engine.shutdown();
      } catch (err) {
        error(err.message);
        process.exitCode = 1;
      }
    });

  cmd
    .command("list")
    .description("List all snapshots")
    .action(async () => {
      try {
        const engine = createEngine();
        const snaps = engine.snapshotList();

        if (snaps.length === 0) {
          info("No snapshots found.");
        } else {
          const table = new Table({
            head: [chalk.cyan("ID"), chalk.cyan("Created At")],
            colWidths: [40, 30],
          });
          for (const s of snaps) {
            table.push([s.id, new Date(s.createdAt * 1000).toISOString()]);
          }
          console.log(table.toString());
        }

        await engine.shutdown();
      } catch (err) {
        error(err.message);
        process.exitCode = 1;
      }
    });
}
