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
}
