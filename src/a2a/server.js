import { createServer } from "http";
import { randomUUID } from "crypto";
import { buildAgentCard } from "./agentCard.js";
import { createEngine } from "../core/engine.js";
import { logger } from "../utils/logger.js";

// ============================================================
// Task Store (in-memory)
// ============================================================

export class TaskStore {
  constructor() {
    this.tasks = new Map();
  }

  create(contextId) {
    const task = {
      kind: "task",
      id: randomUUID(),
      contextId: contextId || randomUUID(),
      status: { state: "submitted", timestamp: new Date().toISOString() },
      artifacts: [],
      history: [],
    };
    this.tasks.set(task.id, task);
    return task;
  }

  get(id) {
    return this.tasks.get(id) || null;
  }

  set(id, task) {
    this.tasks.set(id, task);
  }

  clear() {
    this.tasks.clear();
  }
}

// ============================================================
// Skill Handlers
// ============================================================

async function handleMemoryStore(engine, parts) {
  let content = null;
  let opts = {};

  for (const part of parts) {
    if (part.kind === "data" && part.data) {
      content = part.data.content || content;
      if (part.data.namespace) opts.namespace = part.data.namespace;
      if (part.data.tags) opts.tags = part.data.tags;
      if (part.data.ttl) opts.ttlSeconds = part.data.ttl;
    } else if (part.kind === "text" && part.text) {
      content = content || part.text;
    }
  }

  if (!content) {
    throw Object.assign(new Error("Missing content in message parts"), { code: -32602 });
  }

  const result = await engine.store(content, opts);
  return {
    artifactId: randomUUID(),
    name: "store_result",
    parts: [{ kind: "data", data: { status: result.status, id: result.id || result.existingId } }],
  };
}

async function handleMemorySearch(engine, parts) {
  let query = null;
  let limit = 10;

  for (const part of parts) {
    if (part.kind === "data" && part.data) {
      query = part.data.query || query;
      if (part.data.limit) limit = part.data.limit;
    } else if (part.kind === "text" && part.text) {
      query = query || part.text;
    }
  }

  if (!query) {
    throw Object.assign(new Error("Missing query in message parts"), { code: -32602 });
  }

  const results = await engine.executeQuery({ text: query, limit });
  const formatted = results.map((r) => ({
    id: r.id,
    content: r.content,
    score: r.score,
    namespace: r.namespace,
  }));

  return {
    artifactId: randomUUID(),
    name: "search_results",
    parts: [{ kind: "data", data: { query, results: formatted, count: formatted.length } }],
  };
}

function handleMemoryStatus(engine) {
  const status = engine.status();
  return {
    artifactId: randomUUID(),
    name: "status",
    parts: [
      {
        kind: "data",
        data: {
          status: "ok",
          entry_count: status.entryCount,
          tier: status.tier,
          model_id: status.modelId,
        },
      },
    ],
  };
}

// ============================================================
// Skill Router
// ============================================================

export function resolveSkill(parts) {
  // Check for explicit skill in data parts
  for (const part of parts) {
    if (part.kind === "data" && part.data?.skill) {
      return part.data.skill;
    }
  }

  // Text-based routing
  for (const part of parts) {
    if (part.kind === "text" && part.text) {
      const lower = part.text.toLowerCase();
      if (/\b(store|save|remember)\b/.test(lower)) return "memory_store";
      if (/\b(search|find|recall|query)\b/.test(lower)) return "memory_search";
      if (/\b(status|health|stats|info)\b/.test(lower)) return "memory_status";
    }
  }

  // Check for structured data that implies a skill
  for (const part of parts) {
    if (part.kind === "data" && part.data) {
      if (part.data.content) return "memory_store";
      if (part.data.query) return "memory_search";
    }
  }

  return null;
}

// ============================================================
// JSON-RPC Dispatcher (testable without HTTP)
// ============================================================

export async function dispatchRpc(engine, taskStore, method, params) {
  switch (method) {
    case "message/send":
      return handleMessageSend(engine, taskStore, params || {});
    case "tasks/get":
      return handleTasksGet(taskStore, params || {});
    case "tasks/cancel":
      return handleTasksCancel(taskStore, params || {});
    default:
      return { error: { code: -32601, message: `Method not found: ${method}` } };
  }
}

async function handleMessageSend(engine, taskStore, params) {
  const message = params?.message;
  if (!message || !message.parts || message.parts.length === 0) {
    return { error: { code: -32602, message: "Missing message or parts" } };
  }

  const skill = resolveSkill(message.parts);
  if (!skill) {
    return {
      error: { code: -32005, message: "Could not determine which skill to invoke from message" },
    };
  }

  const task = taskStore.create(message.contextId);

  // Store the incoming message in history
  task.history.push(message);

  try {
    task.status = { state: "working", timestamp: new Date().toISOString() };

    let artifact;
    switch (skill) {
      case "memory_store":
        artifact = await handleMemoryStore(engine, message.parts);
        break;
      case "memory_search":
        artifact = await handleMemorySearch(engine, message.parts);
        break;
      case "memory_status":
        artifact = handleMemoryStatus(engine);
        break;
      default:
        task.status = { state: "failed", timestamp: new Date().toISOString() };
        taskStore.set(task.id, task);
        return { error: { code: -32004, message: `Unknown skill: ${skill}` } };
    }

    task.artifacts = [artifact];
    task.status = { state: "completed", timestamp: new Date().toISOString() };
  } catch (err) {
    task.status = {
      state: "failed",
      timestamp: new Date().toISOString(),
      message: {
        kind: "message",
        role: "agent",
        messageId: randomUUID(),
        parts: [{ kind: "text", text: err.message }],
      },
    };
    taskStore.set(task.id, task);
    if (err.code) return { error: { code: err.code, message: err.message } };
    return { error: { code: -32603, message: err.message } };
  }

  taskStore.set(task.id, task);
  return { result: task };
}

function handleTasksGet(taskStore, params) {
  const id = params?.id;
  if (!id) return { error: { code: -32602, message: "Missing task id" } };

  const task = taskStore.get(id);
  if (!task) return { error: { code: -32001, message: `Task not found: ${id}` } };

  // Apply historyLength filter
  if (params.historyLength !== undefined) {
    const result = { ...task };
    const n = params.historyLength;
    result.history = n === 0 ? [] : task.history.slice(-n);
    return { result };
  }

  return { result: task };
}

function handleTasksCancel(taskStore, params) {
  const id = params?.id;
  if (!id) return { error: { code: -32602, message: "Missing task id" } };

  const task = taskStore.get(id);
  if (!task) return { error: { code: -32001, message: `Task not found: ${id}` } };

  const terminal = ["completed", "failed", "canceled", "rejected"];
  if (terminal.includes(task.status.state)) {
    return {
      error: { code: -32002, message: `Task is already in terminal state: ${task.status.state}` },
    };
  }

  task.status = { state: "canceled", timestamp: new Date().toISOString() };
  taskStore.set(task.id, task);
  return { result: task };
}

// ============================================================
// HTTP Server
// ============================================================

function readBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on("data", (c) => chunks.push(c));
    req.on("end", () => resolve(Buffer.concat(chunks).toString()));
    req.on("error", reject);
  });
}

export function startA2AServer(port = 3000) {
  let engine = null;
  let engineError = null;

  function getEngine() {
    if (engine) return engine;
    if (engineError) throw engineError;
    try {
      engine = createEngine();
      logger.info("A2A engine initialized");
    } catch (err) {
      logger.error("A2A engine initialization failed", { error: err.message });
      engineError = err;
      throw err;
    }
    return engine;
  }

  const baseUrl = `http://localhost:${port}/`;
  const agentCard = buildAgentCard(baseUrl);
  const taskStore = new TaskStore();

  const server = createServer(async (req, res) => {
    // CORS headers
    res.setHeader("Access-Control-Allow-Origin", "*");
    res.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
    res.setHeader("Access-Control-Allow-Headers", "Content-Type");

    if (req.method === "OPTIONS") {
      res.writeHead(204);
      return res.end();
    }

    // Agent Card discovery
    if (req.method === "GET" && req.url === "/.well-known/agent-card.json") {
      res.writeHead(200, { "Content-Type": "application/json" });
      return res.end(JSON.stringify(agentCard, null, 2));
    }

    // JSON-RPC endpoint
    if (req.method === "POST" && (req.url === "/" || req.url === "")) {
      let body;
      try {
        body = await readBody(req);
      } catch (_) {
        res.writeHead(400);
        return res.end();
      }

      let rpc;
      try {
        rpc = JSON.parse(body);
      } catch (_) {
        res.writeHead(200, { "Content-Type": "application/json" });
        return res.end(
          JSON.stringify({
            jsonrpc: "2.0",
            id: null,
            error: { code: -32700, message: "Parse error" },
          }),
        );
      }

      const { id, method, params } = rpc;
      let response;

      try {
        response = await dispatchRpc(getEngine(), taskStore, method, params);
      } catch (err) {
        response = { error: { code: -32603, message: `Engine error: ${err.message}` } };
      }

      res.writeHead(200, { "Content-Type": "application/json" });
      if (response.error) {
        return res.end(JSON.stringify({ jsonrpc: "2.0", id, error: response.error }));
      }
      return res.end(JSON.stringify({ jsonrpc: "2.0", id, result: response.result }));
    }

    res.writeHead(404);
    res.end("Not Found");
  });

  server.listen(port, "127.0.0.1", () => {
    console.log(`\n  Shabti A2A server listening on ${baseUrl}`);
    console.log(`  Agent Card: ${baseUrl}.well-known/agent-card.json`);
    console.log(`  Skills: ${agentCard.skills.map((s) => s.id).join(", ")}\n`);
  });

  async function shutdown() {
    console.log("\n  Shutting down A2A server...");
    server.close();
    if (engine && engine.shutdown) {
      try {
        await engine.shutdown();
      } catch (_) {
        // best-effort
      }
    }
    process.exit(0);
  }

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);

  return server;
}
