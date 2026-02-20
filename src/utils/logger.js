const LEVELS = { debug: 0, info: 1, warn: 2, error: 3, silent: 4 };

function getLevel() {
  const name = (process.env.SHABTI_LOG_LEVEL || "info").toLowerCase();
  return LEVELS[name] ?? LEVELS.info;
}

function isJsonMode() {
  return process.env.SHABTI_LOG_JSON === "1";
}

function shouldLog(level) {
  return LEVELS[level] >= getLevel();
}

function formatTimestamp() {
  return new Date().toISOString();
}

function log(level, message, data) {
  if (!shouldLog(level)) return;

  if (isJsonMode()) {
    const entry = { timestamp: formatTimestamp(), level, message };
    if (data !== undefined) entry.data = data;
    process.stderr.write(JSON.stringify(entry) + "\n");
  } else {
    const prefix = `[${formatTimestamp()}] [${level.toUpperCase()}]`;
    const msg =
      data !== undefined ? `${prefix} ${message} ${JSON.stringify(data)}` : `${prefix} ${message}`;
    process.stderr.write(msg + "\n");
  }
}

export const logger = {
  debug: (message, data) => log("debug", message, data),
  info: (message, data) => log("info", message, data),
  warn: (message, data) => log("warn", message, data),
  error: (message, data) => log("error", message, data),
};
