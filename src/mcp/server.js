#!/usr/bin/env node
import { createInterface } from "readline";
import { createEngine, loadConfig } from "../core/engine.js";
import { createEngineWithRetry } from "../core/retry.js";
import { logger } from "../utils/logger.js";
import { normalizeText } from "../utils/normalize.js";

const SERVER_INFO = {
  name: "shabti-memory",
  version: "2.0.0",
};

const TOOLS = [
  {
    name: "memory_store",
    description: "Store a memory entry in shabti",
    inputSchema: {
      type: "object",
      properties: {
        content: { type: "string", description: "The text content to store" },
        namespace: {
          type: "string",
          description: "Target namespace (default: 'default')",
        },
        tags: {
          type: "array",
          items: { type: "string" },
          description: "Tags to associate with the memory",
        },
        ttl: {
          type: "integer",
          description: "Time-to-live in seconds (entry auto-expires after this duration)",
        },
      },
      required: ["content"],
    },
  },
  {
    name: "memory_search",
    description: "Search stored memories by semantic similarity",
    inputSchema: {
      type: "object",
      properties: {
        query: { type: "string", description: "Search query text" },
        limit: {
          type: "integer",
          description: "Maximum number of results (default: 10)",
        },
        namespace: { type: "string", description: "Namespace to search within" },
      },
      required: ["query"],
    },
  },
  {
    name: "memory_delete",
    description: "Delete a memory entry by ID",
    inputSchema: {
      type: "object",
      properties: {
        id: { type: "string", description: "UUID of the memory entry to delete" },
      },
      required: ["id"],
    },
  },
  {
    name: "memory_list",
    description: "List recent memory entries",
    inputSchema: {
      type: "object",
      properties: {
        limit: {
          type: "integer",
          description: "Maximum number of entries to return (default: 10)",
        },
        namespace: { type: "string", description: "Filter by namespace" },
      },
    },
  },
  {
    name: "memory_export",
    description: "Export all memory entries as a JSONL array",
    inputSchema: {
      type: "object",
      properties: {
        namespace: { type: "string", description: "Filter by namespace" },
        limit: { type: "integer", description: "Maximum entries to export" },
      },
    },
  },
  {
    name: "memory_gc",
    description: "Garbage collect expired memory entries (removes entries past their TTL)",
    inputSchema: {
      type: "object",
      properties: {},
    },
  },
  {
    name: "memory_status",
    description: "Get the current status of the memory engine",
    inputSchema: {
      type: "object",
      properties: {},
    },
  },
  {
    name: "memory_health",
    description:
      "Run health checks on the memory engine (Qdrant connectivity, engine status, embedding model)",
    inputSchema: {
      type: "object",
      properties: {},
    },
  },
  {
    name: "memory_get",
    description: "Retrieve a specific memory entry by its UUID",
    inputSchema: {
      type: "object",
      properties: {
        id: { type: "string", description: "UUID of the memory entry to retrieve" },
      },
      required: ["id"],
    },
  },
];

const RESOURCES = [
  {
    uri: "shabti://status",
    name: "Engine status",
    description: "Current shabti engine status and statistics",
    mimeType: "application/json",
  },
  {
    uri: "shabti://config",
    name: "Engine configuration",
    description: "Current engine configuration",
    mimeType: "application/json",
  },
];

let engine = null;
let engineInitAttempted = false;

function initEngine() {
  if (engine) return engine;
  if (engineInitAttempted) return null;
  // Synchronous fallback for initial attempt
  engineInitAttempted = true;
  try {
    engine = createEngine();
    logger.info("Engine initialized");
  } catch (err) {
    logger.warn("Engine not available, starting background retry", { error: err.message });
    // Start async retry in background
    createEngineWithRetry(createEngine)
      .then((eng) => {
        engine = eng;
        logger.info("Engine connected via retry");
      })
      .catch(() => {
        logger.error("Engine retry exhausted — tools will return errors");
      });
  }
  return engine;
}

function respond(id, result) {
  const msg = JSON.stringify({ jsonrpc: "2.0", id, result });
  process.stdout.write(msg + "\n");
}

function respondError(id, code, message) {
  const msg = JSON.stringify({ jsonrpc: "2.0", id, error: { code, message } });
  process.stdout.write(msg + "\n");
}

function handleInitialize(id) {
  respond(id, {
    protocolVersion: "2024-11-05",
    serverInfo: SERVER_INFO,
    capabilities: {
      tools: {},
      resources: {},
    },
  });
}

function handleToolsList(id) {
  respond(id, { tools: TOOLS });
}

function handleResourcesList(id) {
  respond(id, { resources: RESOURCES });
}

async function handleToolsCall(id, params) {
  const { name, arguments: args } = params;

  const eng = initEngine();

  if (name === "memory_status") {
    if (!eng) {
      return respond(id, {
        content: [
          {
            type: "text",
            text: JSON.stringify(
              { status: "unavailable", entry_count: 0, model_id: "unknown" },
              null,
              2,
            ),
          },
        ],
      });
    }
    const status = eng.status();
    return respond(id, {
      content: [
        {
          type: "text",
          text: JSON.stringify(
            {
              status: "ok",
              entry_count: status.entryCount,
              tier: status.tier,
              model_id: status.modelId,
              qdrant_url: status.qdrantUrl,
            },
            null,
            2,
          ),
        },
      ],
    });
  }

  if (name === "memory_store") {
    if (!eng) {
      return respondError(id, -32603, "Engine not available");
    }
    const content = args?.content;
    if (!content) {
      return respondError(id, -32602, "Missing required parameter: content");
    }
    try {
      const opts = {};
      if (args.namespace) opts.namespace = args.namespace;
      if (args.tags) opts.tags = args.tags;
      if (args.ttl) opts.ttlSeconds = args.ttl;
      const result = await eng.store(normalizeText(content), opts);
      return respond(id, {
        content: [
          {
            type: "text",
            text: JSON.stringify(
              { status: result.status, id: result.id || result.existingId },
              null,
              2,
            ),
          },
        ],
      });
    } catch (err) {
      return respondError(id, -32603, err.message);
    }
  }

  if (name === "memory_search") {
    if (!eng) {
      return respondError(id, -32603, "Engine not available");
    }
    const query = args?.query;
    if (!query) {
      return respondError(id, -32602, "Missing required parameter: query");
    }
    try {
      const limit = args.limit || 10;
      const queryObj = { text: normalizeText(query), limit };
      if (args.namespace) queryObj.namespace = args.namespace;
      const results = await eng.executeQuery(queryObj);
      const formatted = results.map((r) => ({
        content: r.content,
        score: r.score,
        id: r.id,
        namespace: r.namespace,
      }));
      return respond(id, {
        content: [
          {
            type: "text",
            text: JSON.stringify({ query, results: formatted }, null, 2),
          },
        ],
      });
    } catch (err) {
      return respondError(id, -32603, err.message);
    }
  }

  if (name === "memory_delete") {
    if (!eng) {
      return respondError(id, -32603, "Engine not available");
    }
    const entryId = args?.id;
    if (!entryId) {
      return respondError(id, -32602, "Missing required parameter: id");
    }
    try {
      await eng.delete(entryId);
      return respond(id, {
        content: [
          {
            type: "text",
            text: JSON.stringify({ deleted: true, id: entryId }, null, 2),
          },
        ],
      });
    } catch (err) {
      return respondError(id, -32603, err.message);
    }
  }

  if (name === "memory_list") {
    if (!eng) {
      return respondError(id, -32603, "Engine not available");
    }
    try {
      const limit = args?.limit || 10;
      const queryObj = { text: "*", limit };
      if (args?.namespace) queryObj.namespace = args.namespace;
      const results = await eng.executeQuery(queryObj);
      const entries = results.map((r) => ({
        id: r.id,
        content: r.content,
        score: r.score,
        namespace: r.namespace,
        createdAt: r.createdAt,
      }));
      return respond(id, {
        content: [
          {
            type: "text",
            text: JSON.stringify({ entries, count: entries.length }, null, 2),
          },
        ],
      });
    } catch (err) {
      return respondError(id, -32603, err.message);
    }
  }

  if (name === "memory_export") {
    if (!eng) {
      return respondError(id, -32603, "Engine not available");
    }
    try {
      const listOpts = {};
      if (args?.namespace) listOpts.namespace = args.namespace;
      if (args?.limit) listOpts.limit = args.limit;
      const entries = eng.listEntries(listOpts);
      const lines = entries.map((e) => JSON.stringify(e));
      return respond(id, {
        content: [
          {
            type: "text",
            text: JSON.stringify({ entries: entries.length, data: lines.join("\n") }, null, 2),
          },
        ],
      });
    } catch (err) {
      return respondError(id, -32603, err.message);
    }
  }

  if (name === "memory_gc") {
    if (!eng) {
      return respondError(id, -32603, "Engine not available");
    }
    try {
      const removed = await eng.gc();
      return respond(id, {
        content: [
          {
            type: "text",
            text: JSON.stringify({ removed }, null, 2),
          },
        ],
      });
    } catch (err) {
      return respondError(id, -32603, err.message);
    }
  }

  if (name === "memory_health") {
    const checks = [];
    const config = loadConfig();
    const qdrantUrl = config.qdrant_url.replace(/:\d+$/, ":6333");
    try {
      const res = await fetch(`${qdrantUrl}/healthz`, {
        signal: AbortSignal.timeout(3000),
      });
      checks.push({
        name: "qdrant",
        status: res.ok ? "ok" : "degraded",
        message: res.ok ? "Reachable" : `HTTP ${res.status}`,
      });
    } catch (err) {
      checks.push({ name: "qdrant", status: "error", message: err.message });
    }

    if (eng) {
      try {
        const status = eng.status();
        checks.push({
          name: "engine",
          status: "ok",
          message: `${status.entryCount} entries, tier: ${status.tier}`,
        });
      } catch (err) {
        checks.push({ name: "engine", status: "error", message: err.message });
      }
      try {
        const modelId = eng.modelId();
        checks.push({ name: "embedding", status: "ok", message: modelId });
      } catch (err) {
        checks.push({ name: "embedding", status: "error", message: err.message });
      }
    } else {
      checks.push({ name: "engine", status: "error", message: "Engine not available" });
    }

    const healthy = checks.every((c) => c.status === "ok");
    return respond(id, {
      content: [
        {
          type: "text",
          text: JSON.stringify({ healthy, checks }, null, 2),
        },
      ],
    });
  }

  if (name === "memory_get") {
    if (!eng) {
      return respondError(id, -32603, "Engine not available");
    }
    const entryId = args?.id;
    if (!entryId) {
      return respondError(id, -32602, "Missing required parameter: id");
    }
    try {
      const entry = await eng.get(entryId);
      return respond(id, {
        content: [
          {
            type: "text",
            text: JSON.stringify(entry, null, 2),
          },
        ],
      });
    } catch (err) {
      return respondError(id, -32603, err.message);
    }
  }

  respondError(id, -32601, `Unknown tool: ${name}`);
}

function handleResourcesRead(id, params) {
  const { uri } = params;
  const eng = initEngine();

  if (uri === "shabti://status") {
    if (!eng) {
      return respond(id, {
        contents: [
          {
            uri,
            mimeType: "application/json",
            text: JSON.stringify({ status: "unavailable", entry_count: 0 }),
          },
        ],
      });
    }
    const status = eng.status();
    return respond(id, {
      contents: [
        {
          uri,
          mimeType: "application/json",
          text: JSON.stringify({
            status: "ok",
            entry_count: status.entryCount,
            tier: status.tier,
            model_id: status.modelId,
          }),
        },
      ],
    });
  }

  if (uri === "shabti://config") {
    const config = loadConfig();
    return respond(id, {
      contents: [
        {
          uri,
          mimeType: "application/json",
          text: JSON.stringify(config),
        },
      ],
    });
  }

  respondError(id, -32602, `Unknown resource: ${uri}`);
}

async function handleRequest(line) {
  let req;
  try {
    req = JSON.parse(line);
  } catch (_) {
    return respondError(null, -32700, "Parse error");
  }

  const { id, method, params } = req;

  switch (method) {
    case "initialize":
      return handleInitialize(id);
    case "notifications/initialized":
      return; // no response needed for notifications
    case "tools/list":
      return handleToolsList(id);
    case "tools/call":
      return handleToolsCall(id, params || {});
    case "resources/list":
      return handleResourcesList(id);
    case "resources/read":
      return handleResourcesRead(id, params || {});
    default:
      return respondError(id, -32601, "Method not found");
  }
}

// stdio transport: read newline-delimited JSON from stdin
const rl = createInterface({ input: process.stdin, terminal: false });
rl.on("line", (line) => {
  const trimmed = line.trim();
  if (trimmed && trimmed.length <= 10_000_000) {
    // 10MB上限
    handleRequest(trimmed);
  }
});

async function shutdown() {
  logger.info("MCP server shutting down");
  rl.close();
  if (engine && engine.shutdown) {
    try {
      await engine.shutdown();
    } catch (_) {
      // best-effort
    }
  }
  process.exit(0);
}

rl.on("close", shutdown);
process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
