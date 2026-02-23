import { describe, it, expect, beforeAll, beforeEach, afterAll } from "vitest";
import { run } from "./helpers.js";
import { buildAgentCard } from "../src/a2a/agentCard.js";
import { dispatchRpc, resolveSkill, TaskStore } from "../src/a2a/server.js";

// Check if Qdrant is reachable (needed for engine-backed tests)
let qdrantAvailable = false;
try {
  const res = await fetch("http://localhost:6333/healthz", { signal: AbortSignal.timeout(2000) });
  qdrantAvailable = res.ok;
} catch {
  // Qdrant not available
}

// ================================================================
// CLI
// ================================================================

describe("A2A CLI", () => {
  it("shows help with --help", async () => {
    const { stdout, code } = await run(["a2a", "--help"]);
    expect(code).toBe(0);
    expect(stdout).toContain("A2A");
    expect(stdout).toContain("--port");
  });
});

// ================================================================
// Agent Card
// ================================================================

describe("Agent Card", () => {
  it("builds a valid Agent Card", () => {
    const card = buildAgentCard("http://localhost:3000/");
    expect(card.protocolVersion).toBe("0.3.0");
    expect(card.name).toBe("Shabti Memory Engine");
    expect(card.capabilities).toBeDefined();
    expect(card.capabilities.streaming).toBe(false);
    expect(card.skills).toBeInstanceOf(Array);
    expect(card.skills.length).toBe(9);

    const skillIds = card.skills.map((s) => s.id);
    expect(skillIds).toContain("memory_store");
    expect(skillIds).toContain("memory_search");
    expect(skillIds).toContain("memory_status");
    expect(skillIds).toContain("memory_get");
    expect(skillIds).toContain("memory_delete");
    expect(skillIds).toContain("memory_list");
    expect(skillIds).toContain("memory_export");
    expect(skillIds).toContain("memory_gc");
    expect(skillIds).toContain("memory_health");
  });

  it("includes version from package.json", () => {
    const card = buildAgentCard("http://localhost:3000/");
    expect(card.version).toMatch(/^\d+\.\d+\.\d+/);
  });

  it("uses provided baseUrl", () => {
    const card = buildAgentCard("http://example.com:4000/");
    expect(card.url).toBe("http://example.com:4000/");
  });
});

// ================================================================
// Skill Router
// ================================================================

describe("resolveSkill", () => {
  it("resolves explicit skill from data part", () => {
    const parts = [{ kind: "data", data: { skill: "memory_store" } }];
    expect(resolveSkill(parts)).toBe("memory_store");
  });

  it("resolves store from text keywords", () => {
    expect(resolveSkill([{ kind: "text", text: "Remember this fact" }])).toBe("memory_store");
    expect(resolveSkill([{ kind: "text", text: "Save this note" }])).toBe("memory_store");
    expect(resolveSkill([{ kind: "text", text: "Please store this data" }])).toBe("memory_store");
  });

  it("resolves search from text keywords", () => {
    expect(resolveSkill([{ kind: "text", text: "Search for cats" }])).toBe("memory_search");
    expect(resolveSkill([{ kind: "text", text: "Find my notes" }])).toBe("memory_search");
    expect(resolveSkill([{ kind: "text", text: "Recall what I said" }])).toBe("memory_search");
    expect(resolveSkill([{ kind: "text", text: "Query the database" }])).toBe("memory_search");
  });

  it("resolves status from text keywords", () => {
    expect(resolveSkill([{ kind: "text", text: "What is the status?" }])).toBe("memory_status");
    expect(resolveSkill([{ kind: "text", text: "Show me health info" }])).toBe("memory_status");
    expect(resolveSkill([{ kind: "text", text: "Engine stats please" }])).toBe("memory_status");
  });

  it("resolves store from data with content field", () => {
    const parts = [{ kind: "data", data: { content: "hello world" } }];
    expect(resolveSkill(parts)).toBe("memory_store");
  });

  it("resolves search from data with query field", () => {
    const parts = [{ kind: "data", data: { query: "cats" } }];
    expect(resolveSkill(parts)).toBe("memory_search");
  });

  it("returns null for unrecognizable text", () => {
    expect(resolveSkill([{ kind: "text", text: "hello world" }])).toBeNull();
  });

  it("resolves delete from text keywords", () => {
    expect(resolveSkill([{ kind: "text", text: "Delete that entry" }])).toBe("memory_delete");
    expect(resolveSkill([{ kind: "text", text: "Remove old memories" }])).toBe("memory_delete");
  });

  it("resolves get from text keywords", () => {
    expect(resolveSkill([{ kind: "text", text: "Get entry details" }])).toBe("memory_get");
    expect(resolveSkill([{ kind: "text", text: "Retrieve the record" }])).toBe("memory_get");
  });

  it("resolves list from text keywords", () => {
    expect(resolveSkill([{ kind: "text", text: "List all memories" }])).toBe("memory_list");
  });

  it("resolves export from text keywords", () => {
    expect(resolveSkill([{ kind: "text", text: "Export the data" }])).toBe("memory_export");
    expect(resolveSkill([{ kind: "text", text: "Dump all entries" }])).toBe("memory_export");
  });

  it("resolves gc from text keywords", () => {
    expect(resolveSkill([{ kind: "text", text: "Run garbage collect" }])).toBe("memory_gc");
    expect(resolveSkill([{ kind: "text", text: "Cleanup expired entries" }])).toBe("memory_gc");
  });

  it("resolves data with id field to memory_get", () => {
    const parts = [{ kind: "data", data: { id: "some-uuid" } }];
    expect(resolveSkill(parts)).toBe("memory_get");
  });

  it("returns null for empty parts", () => {
    expect(resolveSkill([])).toBeNull();
  });

  it("prioritizes explicit skill over text routing", () => {
    const parts = [
      { kind: "data", data: { skill: "memory_status" } },
      { kind: "text", text: "Remember this" },
    ];
    expect(resolveSkill(parts)).toBe("memory_status");
  });
});

// ================================================================
// Task Store
// ================================================================

describe("TaskStore", () => {
  let store;

  beforeEach(() => {
    store = new TaskStore();
  });

  it("creates a task with correct initial state", () => {
    const task = store.create();
    expect(task.kind).toBe("task");
    expect(task.id).toBeDefined();
    expect(task.contextId).toBeDefined();
    expect(task.status.state).toBe("submitted");
    expect(task.artifacts).toEqual([]);
    expect(task.history).toEqual([]);
  });

  it("creates a task with provided contextId", () => {
    const task = store.create("ctx-123");
    expect(task.contextId).toBe("ctx-123");
  });

  it("retrieves a created task", () => {
    const task = store.create();
    const retrieved = store.get(task.id);
    expect(retrieved).toBe(task);
  });

  it("returns null for unknown task", () => {
    expect(store.get("nonexistent")).toBeNull();
  });

  it("clears all tasks", () => {
    store.create();
    store.create();
    store.clear();
    expect(store.get(store.create().id)).toBeDefined();
  });
});

// ================================================================
// JSON-RPC Dispatcher (without engine — error paths & routing)
// ================================================================

describe("dispatchRpc routing", () => {
  let store;

  beforeEach(() => {
    store = new TaskStore();
  });

  it("returns -32601 for unknown method", async () => {
    const res = await dispatchRpc(null, store, "nonexistent/method", {});
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32601);
  });

  it("message/send without parts returns error", async () => {
    const res = await dispatchRpc(null, store, "message/send", {
      message: { kind: "message", role: "user", messageId: "msg-1", parts: [] },
    });
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32602);
  });

  it("message/send without message returns error", async () => {
    const res = await dispatchRpc(null, store, "message/send", {});
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32602);
  });

  it("message/send with unrecognizable text returns -32005", async () => {
    const res = await dispatchRpc(null, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-1",
        parts: [{ kind: "text", text: "hello world" }],
      },
    });
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32005);
  });

  it("tasks/get without id returns error", async () => {
    const res = await dispatchRpc(null, store, "tasks/get", {});
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32602);
  });

  it("tasks/get for unknown task returns -32001", async () => {
    const res = await dispatchRpc(null, store, "tasks/get", { id: "nonexistent" });
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32001);
  });

  it("tasks/cancel for unknown task returns -32001", async () => {
    const res = await dispatchRpc(null, store, "tasks/cancel", { id: "nonexistent" });
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32001);
  });
});

// ================================================================
// JSON-RPC Dispatcher with mock engine (status only — no Qdrant)
// ================================================================

describe("dispatchRpc with mock engine", () => {
  let store;
  const mockEngine = {
    status: () => ({
      entryCount: 42,
      tier: "local",
      modelId: "mock-model",
      qdrantUrl: "http://localhost:6334",
    }),
    modelId: () => "mock-model",
    delete: async () => {},
    get: async (id) => ({
      id,
      content: "mock content",
      score: 1.0,
      namespace: "default",
      createdAt: 1700000000,
    }),
    executeQuery: async () => [
      { id: "id-1", content: "entry 1", score: 0.9, namespace: "default" },
    ],
    listEntries: () => [
      {
        id: "id-1",
        content: "entry 1",
        namespace: "default",
        tags: [],
        keywords: [],
        sessionId: "",
        createdAt: 1700000000,
        lifecycleState: "Active",
        originType: "User",
        modelId: "mock",
      },
    ],
    gc: async () => 3,
  };

  beforeEach(() => {
    store = new TaskStore();
  });

  it("message/send memory_status returns completed task", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-status-1",
        parts: [{ kind: "data", data: { skill: "memory_status" } }],
      },
    });

    expect(res.result).toBeDefined();
    expect(res.result.kind).toBe("task");
    expect(res.result.status.state).toBe("completed");
    expect(res.result.artifacts).toBeInstanceOf(Array);
    expect(res.result.artifacts.length).toBe(1);

    const data = res.result.artifacts[0].parts[0].data;
    expect(data.status).toBe("ok");
    expect(data.entry_count).toBe(42);
    expect(data.tier).toBe("local");
    expect(data.model_id).toBe("mock-model");
  });

  it("message/send memory_status via text routing", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-status-2",
        parts: [{ kind: "text", text: "What is the engine status?" }],
      },
    });

    expect(res.result).toBeDefined();
    expect(res.result.status.state).toBe("completed");
  });

  it("tasks/get returns previously created task", async () => {
    const sendRes = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-get-1",
        parts: [{ kind: "data", data: { skill: "memory_status" } }],
      },
    });
    const taskId = sendRes.result.id;

    const getRes = await dispatchRpc(mockEngine, store, "tasks/get", { id: taskId });
    expect(getRes.result).toBeDefined();
    expect(getRes.result.id).toBe(taskId);
    expect(getRes.result.status.state).toBe("completed");
  });

  it("tasks/get with historyLength filters history", async () => {
    const sendRes = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-hist-1",
        parts: [{ kind: "data", data: { skill: "memory_status" } }],
      },
    });
    const taskId = sendRes.result.id;

    const getRes = await dispatchRpc(mockEngine, store, "tasks/get", {
      id: taskId,
      historyLength: 0,
    });
    expect(getRes.result.history).toEqual([]);
  });

  it("tasks/cancel returns -32002 for completed task", async () => {
    const sendRes = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-cancel-1",
        parts: [{ kind: "data", data: { skill: "memory_status" } }],
      },
    });
    const taskId = sendRes.result.id;

    const cancelRes = await dispatchRpc(mockEngine, store, "tasks/cancel", { id: taskId });
    expect(cancelRes.error).toBeDefined();
    expect(cancelRes.error.code).toBe(-32002);
  });

  it("task has history with the incoming message", async () => {
    const message = {
      kind: "message",
      role: "user",
      messageId: "msg-hist-2",
      parts: [{ kind: "data", data: { skill: "memory_status" } }],
    };
    const res = await dispatchRpc(mockEngine, store, "message/send", { message });

    expect(res.result.history.length).toBe(1);
    expect(res.result.history[0].messageId).toBe("msg-hist-2");
  });

  it("message/send with unknown skill returns -32004", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-unk-1",
        parts: [{ kind: "data", data: { skill: "nonexistent_skill" } }],
      },
    });
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32004);
  });

  it("message/send memory_store without content returns -32602", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-err-1",
        parts: [{ kind: "data", data: { skill: "memory_store" } }],
      },
    });
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32602);
  });

  it("message/send memory_search without query returns -32602", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-err-2",
        parts: [{ kind: "data", data: { skill: "memory_search" } }],
      },
    });
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32602);
  });

  it("message/send with engine error returns -32603", async () => {
    const brokenEngine = {
      ...mockEngine,
      store: () => {
        throw new Error("engine boom");
      },
    };
    const res = await dispatchRpc(brokenEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-err-3",
        parts: [{ kind: "data", data: { skill: "memory_store", content: "test" } }],
      },
    });
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32603);
  });

  it("tasks/cancel on submitted task succeeds", async () => {
    // Create a task directly in submitted state
    const task = store.create();
    const res = await dispatchRpc(mockEngine, store, "tasks/cancel", { id: task.id });
    expect(res.result).toBeDefined();
    expect(res.result.status.state).toBe("canceled");
  });

  it("tasks/cancel without id returns -32602", async () => {
    const res = await dispatchRpc(mockEngine, store, "tasks/cancel", {});
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32602);
  });

  it("message/send memory_delete removes entry and returns completed", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-del-1",
        parts: [{ kind: "data", data: { skill: "memory_delete", id: "test-uuid" } }],
      },
    });
    expect(res.result).toBeDefined();
    expect(res.result.status.state).toBe("completed");
    const data = res.result.artifacts[0].parts[0].data;
    expect(data.deleted).toBe(true);
    expect(data.id).toBe("test-uuid");
  });

  it("message/send memory_delete without id returns -32602", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-del-2",
        parts: [{ kind: "data", data: { skill: "memory_delete" } }],
      },
    });
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32602);
  });

  it("message/send memory_get retrieves entry", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-get-2",
        parts: [{ kind: "data", data: { skill: "memory_get", id: "test-uuid" } }],
      },
    });
    expect(res.result).toBeDefined();
    expect(res.result.status.state).toBe("completed");
    const data = res.result.artifacts[0].parts[0].data;
    expect(data.id).toBe("test-uuid");
    expect(data.content).toBe("mock content");
  });

  it("message/send memory_get without id returns -32602", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-get-3",
        parts: [{ kind: "data", data: { skill: "memory_get" } }],
      },
    });
    expect(res.error).toBeDefined();
    expect(res.error.code).toBe(-32602);
  });

  it("message/send memory_list returns entries", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-list-1",
        parts: [{ kind: "data", data: { skill: "memory_list", limit: 5 } }],
      },
    });
    expect(res.result).toBeDefined();
    expect(res.result.status.state).toBe("completed");
    const data = res.result.artifacts[0].parts[0].data;
    expect(data.entries).toBeInstanceOf(Array);
    expect(typeof data.count).toBe("number");
  });

  it("message/send memory_export returns JSONL data", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-export-1",
        parts: [{ kind: "data", data: { skill: "memory_export" } }],
      },
    });
    expect(res.result).toBeDefined();
    expect(res.result.status.state).toBe("completed");
    const data = res.result.artifacts[0].parts[0].data;
    expect(typeof data.entries).toBe("number");
    expect(typeof data.data).toBe("string");
  });

  it("message/send memory_gc returns removed count", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-gc-1",
        parts: [{ kind: "data", data: { skill: "memory_gc" } }],
      },
    });
    expect(res.result).toBeDefined();
    expect(res.result.status.state).toBe("completed");
    const data = res.result.artifacts[0].parts[0].data;
    expect(data.removed).toBe(3);
  });

  it("message/send memory_health returns health checks", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-health-1",
        parts: [{ kind: "data", data: { skill: "memory_health" } }],
      },
    });
    expect(res.result).toBeDefined();
    expect(res.result.status.state).toBe("completed");
    const data = res.result.artifacts[0].parts[0].data;
    expect(typeof data.healthy).toBe("boolean");
    expect(data.checks).toBeInstanceOf(Array);
  });

  it("message/send preserves contextId from message", async () => {
    const res = await dispatchRpc(mockEngine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-ctx-1",
        contextId: "my-context",
        parts: [{ kind: "data", data: { skill: "memory_status" } }],
      },
    });
    expect(res.result.contextId).toBe("my-context");
  });
});

// ================================================================
// Engine-backed tests (require Qdrant)
// ================================================================

describe.skipIf(!qdrantAvailable)("dispatchRpc with real engine", () => {
  let store;
  let engine;

  beforeAll(async () => {
    const { createEngine } = await import("../src/core/engine.js");
    engine = createEngine();
  });

  afterAll(async () => {
    if (engine?.shutdown) await engine.shutdown();
  });

  beforeEach(() => {
    store = new TaskStore();
  });

  it("message/send memory_store with data part", async () => {
    const res = await dispatchRpc(engine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-store-1",
        parts: [
          {
            kind: "data",
            data: { skill: "memory_store", content: `A2A test entry ${Date.now()}` },
          },
        ],
      },
    });

    expect(res.result).toBeDefined();
    expect(res.result.kind).toBe("task");
    expect(res.result.status.state).toBe("completed");
    const data = res.result.artifacts[0].parts[0].data;
    expect(data.status).toBe("stored");
    expect(data.id).toBeDefined();
  });

  it("message/send memory_store with text part", async () => {
    const res = await dispatchRpc(engine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-store-2",
        parts: [{ kind: "text", text: `Remember: A2A text test ${Date.now()}` }],
      },
    });

    expect(res.result).toBeDefined();
    expect(res.result.status.state).toBe("completed");
    const data = res.result.artifacts[0].parts[0].data;
    expect(data.status).toBe("stored");
  });

  it("message/send memory_search with data part", async () => {
    const res = await dispatchRpc(engine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-search-1",
        parts: [{ kind: "data", data: { skill: "memory_search", query: "A2A test", limit: 5 } }],
      },
    });

    expect(res.result).toBeDefined();
    expect(res.result.status.state).toBe("completed");
    const data = res.result.artifacts[0].parts[0].data;
    expect(data.results).toBeInstanceOf(Array);
    expect(typeof data.count).toBe("number");
  });

  it("message/send memory_search with text part", async () => {
    const res = await dispatchRpc(engine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-search-2",
        parts: [{ kind: "text", text: "search for A2A test entries" }],
      },
    });

    expect(res.result).toBeDefined();
    expect(res.result.status.state).toBe("completed");
  });

  it("message/send memory_status returns engine status", async () => {
    const res = await dispatchRpc(engine, store, "message/send", {
      message: {
        kind: "message",
        role: "user",
        messageId: "msg-status-real",
        parts: [{ kind: "data", data: { skill: "memory_status" } }],
      },
    });

    expect(res.result).toBeDefined();
    expect(res.result.status.state).toBe("completed");
    const data = res.result.artifacts[0].parts[0].data;
    expect(data.status).toBe("ok");
    expect(typeof data.entry_count).toBe("number");
  });
});
