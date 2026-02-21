import chalk from "chalk";
import { loadConfig, saveConfig, DEFAULT_CONFIG } from "../core/engine.js";
import { success, error, heading } from "../utils/style.js";

const ALLOWED_KEYS = Object.keys(DEFAULT_CONFIG);

export function registerConfig(program) {
  const cmd = program.command("config").description("Manage shabti configuration");

  cmd
    .command("show")
    .description("Show current configuration")
    .option("-j, --json", "Output as JSON")
    .action((opts) => {
      const config = loadConfig();

      if (opts.json) {
        console.log(JSON.stringify(config, null, 2));
      } else {
        heading("Configuration");
        console.log();
        for (const [key, value] of Object.entries(config)) {
          console.log(`  ${chalk.cyan(key + ":")}  ${value}`);
        }
        console.log();
      }
    });

  cmd
    .command("set")
    .description("Set a configuration value")
    .argument("<key>", "Config key to set")
    .argument("<value>", "New value")
    .action((key, value) => {
      if (!ALLOWED_KEYS.includes(key)) {
        error(`Unknown config key: ${key}`);
        console.log(`  Allowed keys: ${ALLOWED_KEYS.join(", ")}`);
        process.exitCode = 1;
        return;
      }

      const config = loadConfig();
      config[key] = value;
      saveConfig(config);
      success(`${key} = ${value}`);
    });

  cmd
    .command("setup")
    .description("Show Qdrant setup instructions")
    .option("--check", "Test connection to Qdrant")
    .option("--detect", "Detect locally installed qdrant binary")
    .action(async (opts) => {
      const config = loadConfig();
      heading("Qdrant Setup");
      console.log();
      console.log("  shabti requires a running Qdrant instance for vector storage.");
      console.log();

      // Option 1: Docker
      console.log(chalk.cyan("  Option 1: Docker"));
      console.log();
      console.log(`    docker run -d --name qdrant -p 6333:6333 -p 6334:6334 qdrant/qdrant`);
      console.log();

      // Option 2: Native binary
      console.log(chalk.cyan("  Option 2: Native binary (no Docker required)"));
      console.log();
      console.log(
        `    Download from: ${chalk.underline("https://github.com/qdrant/qdrant/releases")}`,
      );
      console.log();
      const plat = process.platform;
      if (plat === "linux") {
        console.log("    Linux:");
        console.log(
          "      wget https://github.com/qdrant/qdrant/releases/latest/download/qdrant-x86_64-unknown-linux-gnu.tar.gz",
        );
        console.log("      tar xzf qdrant-x86_64-unknown-linux-gnu.tar.gz");
        console.log("      ./qdrant");
      } else if (plat === "darwin") {
        console.log("    macOS:");
        console.log("      Download the macOS binary from the releases page,");
        console.log("      or build from source: cargo install qdrant");
      } else if (plat === "win32") {
        console.log("    Windows:");
        console.log("      Download qdrant.exe from the releases page and run it.");
      }
      console.log();

      console.log(`  Current Qdrant URL: ${chalk.yellow(config.qdrant_url)}`);
      console.log();
      console.log("  To change the URL:");
      console.log(`    shabti config set qdrant_url ${chalk.dim("<url>")}`);
      console.log();

      if (opts.detect) {
        const { execFileSync } = await import("node:child_process");
        const cmd = plat === "win32" ? "where" : "which";
        try {
          const result = execFileSync(cmd, ["qdrant"], {
            encoding: "utf8",
            timeout: 5000,
          }).trim();
          success(`Qdrant binary found at: ${result}`);
        } catch (_) {
          console.log(chalk.yellow("  Qdrant binary not found in PATH."));
          console.log("  Download it from https://github.com/qdrant/qdrant/releases");
        }
      }

      if (opts.check) {
        const restUrl = config.qdrant_url.replace(":6334", ":6333");
        try {
          const res = await fetch(`${restUrl}/healthz`);
          if (res.ok) {
            success("Qdrant is reachable and healthy.");
          } else {
            error(`Qdrant responded with status ${res.status}`);
            process.exitCode = 1;
          }
        } catch (err) {
          error(`Cannot reach Qdrant at ${restUrl}: ${err.message}`);
          process.exitCode = 1;
        }
      }
    });
}
