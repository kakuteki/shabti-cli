import { createServer } from "http";
import { randomUUID } from "crypto";
import { buildAgentCard } from "./agentCard.js";
import { createEngine } from "../core/engine.js";
import { logger } from "../utils/logger.js";
import { normalizeText } from "../utils/normalize.js";

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

  const result = await engine.store(normalizeText(content), opts);
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

  const results = await engine.executeQuery({ text: normalizeText(query), limit });
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

async function handleMemoryDelete(engine, parts) {
  let id = null;

  for (const part of parts) {
    if (part.kind === "data" && part.data?.id) {
      id = part.data.id;
    }
  }

  if (!id) {
    throw Object.assign(new Error("Missing id in message parts"), { code: -32602 });
  }

  await engine.delete(id);
  return {
    artifactId: randomUUID(),
    name: "delete_result",
    parts: [{ kind: "data", data: { deleted: true, id } }],
  };
}

async function handleMemoryGet(engine, parts) {
  let id = null;

  for (const part of parts) {
    if (part.kind === "data" && part.data?.id) {
      id = part.data.id;
    }
  }

  if (!id) {
    throw Object.assign(new Error("Missing id in message parts"), { code: -32602 });
  }

  const entry = await engine.get(id);
  return {
    artifactId: randomUUID(),
    name: "get_result",
    parts: [{ kind: "data", data: entry }],
  };
}

function handleMemoryList(engine, parts) {
  let opts = {};

  for (const part of parts) {
    if (part.kind === "data" && part.data) {
      if (part.data.limit) opts.limit = part.data.limit;
      if (part.data.namespace) opts.namespace = part.data.namespace;
    }
  }

  const entries = engine.listEntries(opts);
  return {
    artifactId: randomUUID(),
    name: "list_result",
    parts: [{ kind: "data", data: { entries, count: entries.length } }],
  };
}

function handleMemoryExport(engine) {
  const entries = engine.listEntries();
  const lines = entries.map((e) => JSON.stringify(e));
  return {
    artifactId: randomUUID(),
    name: "export_result",
    parts: [{ kind: "data", data: { entries: entries.length, data: lines.join("\n") } }],
  };
}

async function handleMemoryGc(engine) {
  const removed = await engine.gc();
  return {
    artifactId: randomUUID(),
    name: "gc_result",
    parts: [{ kind: "data", data: { removed } }],
  };
}

async function handleMemoryHealth(engine) {
  const checks = [];
  const status = engine.status();
  const qdrantUrl = status.qdrantUrl.replace(/:\d+$/, ":6333");

  try {
    const res = await fetch(`${qdrantUrl}/healthz`, { signal: AbortSignal.timeout(3000) });
    checks.push({
      name: "qdrant",
      status: res.ok ? "ok" : "degraded",
      message: res.ok ? "Reachable" : `HTTP ${res.status}`,
    });
  } catch (err) {
    checks.push({ name: "qdrant", status: "error", message: err.message });
  }

  checks.push({ name: "engine", status: "ok", message: `${status.entryCount} entries` });
  checks.push({ name: "embedding", status: "ok", message: `Model: ${engine.modelId()}` });

  const healthy = checks.every((c) => c.status === "ok");
  return {
    artifactId: randomUUID(),
    name: "health_result",
    parts: [{ kind: "data", data: { healthy, checks } }],
  };
}

// ============================================================
// Skill Router
// ============================================================

export function resolveSkill(parts) {
  // 破壊的操作: dataパートの明示的なskill指定のみ受け付ける
  const DESTRUCTIVE_SKILLS = new Set(["memory_delete", "memory_gc", "memory_export"]);

  // まず明示的なskill指定を探す
  for (const part of parts) {
    if (part.kind === "data" && part.data?.skill) {
      return part.data.skill;
    }
    // データフィールドによる暗黙のルーティング (非破壊的操作のみ)
    if (part.kind === "data" && part.data) {
      if (part.data.content !== undefined) return "memory_store";
      if (part.data.query !== undefined) return "memory_search";
      if (part.data.id !== undefined) return "memory_get";
    }
  }

  // テキストマッチは読み取り専用操作のみに制限
  // 破壊的スキルを先に評価してブロックすることで"dump all entries"等が
  // 誤ってmemory_listにマッチするのを防ぐ
  for (const part of parts) {
    if (part.kind === "text" && part.text) {
      const lower = part.text.toLowerCase();
      let candidate = null;
      if (/\b(store|save|remember)\b/.test(lower)) candidate = "memory_store";
      else if (/\b(search|find|recall|query)\b/.test(lower)) candidate = "memory_search";
      else if (/\b(delete|remove)\b/.test(lower)) candidate = "memory_delete";
      else if (/\b(export|dump)\b/.test(lower)) candidate = "memory_export";
      else if (/\b(gc|garbage|cleanup|clean)\b/.test(lower)) candidate = "memory_gc";
      else if (/\b(status|health|stats|info)\b/.test(lower)) candidate = "memory_status";
      else if (/\b(list|show)\b/.test(lower)) candidate = "memory_list";
      else if (/\b(get|fetch|retrieve)\b/.test(lower)) candidate = "memory_get";
      if (candidate !== null && !DESTRUCTIVE_SKILLS.has(candidate)) return candidate;
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
      case "memory_delete":
        artifact = await handleMemoryDelete(engine, message.parts);
        break;
      case "memory_get":
        artifact = await handleMemoryGet(engine, message.parts);
        break;
      case "memory_list":
        artifact = handleMemoryList(engine, message.parts);
        break;
      case "memory_export":
        artifact = handleMemoryExport(engine);
        break;
      case "memory_gc":
        artifact = await handleMemoryGc(engine);
        break;
      case "memory_health":
        artifact = await handleMemoryHealth(engine);
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
