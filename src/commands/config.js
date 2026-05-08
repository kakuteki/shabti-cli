import chalk from "chalk";
import { loadConfig, saveConfig, DEFAULT_CONFIG } from "../core/engine.js";
import { success, error, heading } from "../utils/style.js";
import { validateQdrantUrl } from "../utils/validate.js";

const ALLOWED_KEYS = Object.keys(DEFAULT_CONFIG);

// キーごとのバリデーター
const CONFIG_VALIDATORS = {
  qdrant_url: (value) => {
    try {
      const url = new URL(value);
      if (!["http:", "https:"].includes(url.protocol)) {
        throw new Error("http://またはhttps://のみ使用できます");
      }
    } catch (e) {
      throw new Error(`qdrant_urlが無効です: ${e.message}`, { cause: e });
    }
  },
  data_dir: (value) => {
    const { resolve } = require("path");
    const abs = resolve(value);
    // 危険なシステムパスを拒否
    const dangerousPaths = ["/etc", "/usr", "/bin", "/sbin", "/sys", "/proc", "/dev"];
    if (dangerousPaths.some((p) => abs.startsWith(p))) {
      throw new Error(`data_dirにシステムパスは指定できません: ${abs}`);
    }
  },
  collection_name: (value) => {
    if (!/^[a-zA-Z0-9_-]+$/.test(value) || value.length > 256) {
      throw new Error("collection_nameは英数字、ハイフン、アンダースコアのみ使用できます");
    }
  },
};

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

      if (CONFIG_VALIDATORS[key]) {
        try {
          CONFIG_VALIDATORS[key](value);
        } catch (e) {
          error(e.message);
          return;
        }
      }

      const config = loadConfig();
      let validatedValue = value;
      if (key === "qdrant_url") {
        try {
          validatedValue = validateQdrantUrl(value);
        } catch (err) {
          error(err.message);
          process.exitCode = 1;
          return;
        }
      }
      config[key] = validatedValue;
      saveConfig(config);
      success(`${key} = ${validatedValue}`);
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
