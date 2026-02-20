import { logger } from "../utils/logger.js";

const DEFAULT_MAX_RETRIES = 3;
const DEFAULT_BACKOFF_MS = [500, 1000, 2000];

/**
 * Create an engine with retry and exponential backoff.
 * Intended for long-running servers (MCP, A2A) — not for one-shot CLI commands.
 *
 * @param {Function} factory - Function that creates the engine (e.g., createEngine)
 * @param {object} [options]
 * @param {number} [options.maxRetries=3]
 * @param {number[]} [options.backoffMs=[500,1000,2000]]
 * @returns {Promise<object>} The created engine
 */
export async function createEngineWithRetry(factory, options = {}) {
  const maxRetries = options.maxRetries ?? DEFAULT_MAX_RETRIES;
  const backoffs = options.backoffMs ?? DEFAULT_BACKOFF_MS;

  let lastError;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      const engine = factory();
      if (attempt > 0) {
        logger.info(`Engine connected after ${attempt + 1} attempts`);
      }
      return engine;
    } catch (err) {
      lastError = err;
      if (attempt < maxRetries) {
        const delay = backoffs[Math.min(attempt, backoffs.length - 1)];
        logger.warn(
          `Engine connection failed (attempt ${attempt + 1}/${maxRetries + 1}), retrying in ${delay}ms`,
          {
            error: err.message,
          },
        );
        await new Promise((resolve) => setTimeout(resolve, delay));
      }
    }
  }

  logger.error(`Engine connection failed after ${maxRetries + 1} attempts`, {
    error: lastError.message,
    hint: "Is Qdrant running? Try: shabti config setup --check",
  });
  throw lastError;
}
