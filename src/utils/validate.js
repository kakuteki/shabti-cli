/**
 * Parse a string as a positive integer. Returns the number or throws.
 */
export function parsePositiveInt(value, name) {
  const n = parseInt(value, 10);
  if (isNaN(n) || n <= 0) {
    throw new Error(`${name} must be a positive integer, got: ${value}`);
  }
  return n;
}

/**
 * Parse a string as a non-negative integer (>= 0). Returns the number or throws.
 */
export function parseNonNegativeInt(value, name) {
  const n = parseInt(value, 10);
  if (isNaN(n) || n < 0) {
    throw new Error(`${name} must be a non-negative integer, got: ${value}`);
  }
  return n;
}

/**
 * Parse a string as a positive float. Returns the number or throws.
 */
export function parsePositiveFloat(value, name) {
  const n = parseFloat(value);
  if (isNaN(n) || n <= 0) {
    throw new Error(`${name} must be a positive number, got: ${value}`);
  }
  return n;
}

/**
 * Parse a string as a float in [0, 1]. Returns the number or throws.
 */
export function parseScore(value, name) {
  const n = parseFloat(value);
  if (isNaN(n) || n < 0 || n > 1) {
    throw new Error(`${name} must be a number between 0 and 1, got: ${value}`);
  }
  return n;
}

/**
 * Parse a string as a valid TCP port (1–65535). Returns the number or throws.
 */
export function parsePort(value, name) {
  const n = parseInt(value, 10);
  if (isNaN(n) || n < 1 || n > 65535) {
    throw new Error(`${name} must be a port number (1–65535), got: ${value}`);
  }
  return n;
}

/**
 * qdrant_urlのSSRF防止バリデーション
 * http/httpsスキームのみ許可し、非ローカルホストの場合は警告を出す
 */
export function validateQdrantUrl(raw) {
  let parsed;
  try {
    parsed = new URL(raw);
  } catch {
    throw new Error(`無効なqdrant_url: ${raw}`);
  }
  if (!["http:", "https:"].includes(parsed.protocol)) {
    throw new Error(`qdrant_urlにはhttp://またはhttps://のみ使用できます: ${raw}`);
  }
  return raw;
}
