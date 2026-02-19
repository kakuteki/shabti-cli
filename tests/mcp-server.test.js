import { describe, it, expect } from "vitest";
import { spawn } from "child_process";
import { join } from "path";

const MCP_BIN = join(import.meta.dirname, "..", "src", "mcp", "server.js");

// Check if Qdrant is reachable (for tests that need the engine)
let qdrantAvailable = false;
try {
  const res = await fetch("http://localhost:6333/healthz", { signal: AbortSignal.timeout(2000) });
  qdrantAvailable = res.ok;
} catch {
  // Qdrant not available
}

/**
 * Send a JSON-RPC request to the MCP server and collect the response.
 */
function sendRequest(proc, request, timeoutMs = 10_000) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error("timeout")), timeoutMs);
    let buf = "";

    const onData = (chunk) => {
      buf += chunk.toString();
      // MCP stdio uses newline-delimited JSON
      const lines = buf.split("\n");
      for (let i = 0; i < lines.length - 1; i++) {
        const line = lines[i].trim();
        if (!line) continue;
        try {
          const parsed = JSON.parse(line);
          if (parsed.id === request.id) {
            clearTimeout(timeout);
            proc.stdout.off("data", onData);
            resolve(parsed);
            return;
          }
        } catch {
          // ignore partial lines
        }
      }
      buf = lines[lines.length - 1];
    };

    proc.stdout.on("data", onData);
    proc.stdin.write(JSON.stringify(request) + "\n");
  });
}

function spawnMcp(env = {}) {
  return spawn("node", [MCP_BIN], {
    stdio: ["pipe", "pipe", "pipe"],
    env: { ...process.env, FORCE_COLOR: "0", ...env },
  });
}

describe("MCP server", () => {
  it("responds to initialize with server info and capabilities", async () => {
    const proc = spawnMcp();
    try {
      const res = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      expect(res.jsonrpc).toBe("2.0");
      expect(res.id).toBe(1);
      expect(res.result.protocolVersion).toBe("2024-11-05");
      expect(res.result.serverInfo.name).toBe("shabti-memory");
      expect(res.result.capabilities).toHaveProperty("tools");
    } finally {
      proc.kill();
    }
  });

  it("lists tools with memory_store, memory_search, memory_status", async () => {
    const proc = spawnMcp();
    try {
      // Must initialize first
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      const res = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 2,
        method: "tools/list",
        params: {},
      });

      expect(res.result.tools).toBeInstanceOf(Array);
      const names = res.result.tools.map((t) => t.name);
      expect(names).toContain("memory_store");
      expect(names).toContain("memory_search");
      expect(names).toContain("memory_status");
    } finally {
      proc.kill();
    }
  });

  it("lists resources with shabti:// URIs", async () => {
    const proc = spawnMcp();
    try {
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      const res = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 2,
        method: "resources/list",
        params: {},
      });

      expect(res.result.resources).toBeInstanceOf(Array);
      const uris = res.result.resources.map((r) => r.uri);
      expect(uris).toContain("shabti://status");
      expect(uris).toContain("shabti://config");
    } finally {
      proc.kill();
    }
  });

  it("returns error for unknown method", async () => {
    const proc = spawnMcp();
    try {
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      const res = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 2,
        method: "nonexistent/method",
        params: {},
      });

      expect(res.error).toBeDefined();
      expect(res.error.code).toBe(-32601);
    } finally {
      proc.kill();
    }
  });

  it.skipIf(!qdrantAvailable)("tools/call memory_status returns engine status", async () => {
    const proc = spawnMcp();
    try {
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      const res = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 2,
        method: "tools/call",
        params: {
          name: "memory_status",
          arguments: {},
        },
      });

      expect(res.result).toBeDefined();
      expect(res.result.content).toBeInstanceOf(Array);
      expect(res.result.content[0].type).toBe("text");
      const output = JSON.parse(res.result.content[0].text);
      expect(output).toHaveProperty("entry_count");
      expect(output).toHaveProperty("model_id");
    } finally {
      proc.kill();
    }
  });

  it.skipIf(!qdrantAvailable)("tools/call memory_store stores content and returns id", async () => {
    const proc = spawnMcp();
    try {
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      const res = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 2,
        method: "tools/call",
        params: {
          name: "memory_store",
          arguments: { content: `MCP test memory entry ${Date.now()}` },
        },
      });

      expect(res.result).toBeDefined();
      expect(res.result.content).toBeInstanceOf(Array);
      const output = JSON.parse(res.result.content[0].text);
      expect(output.status).toBe("stored");
      expect(output.id).toBeDefined();
    } finally {
      proc.kill();
    }
  });

  it.skipIf(!qdrantAvailable)("tools/call memory_search returns results array", async () => {
    const proc = spawnMcp();
    try {
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      // Store something first
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 2,
        method: "tools/call",
        params: {
          name: "memory_store",
          arguments: { content: "Tokyo is the capital of Japan" },
        },
      });

      // Search
      const res = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 3,
        method: "tools/call",
        params: {
          name: "memory_search",
          arguments: { query: "capital of Japan", limit: 5 },
        },
      });

      expect(res.result).toBeDefined();
      expect(res.result.content).toBeInstanceOf(Array);
      const output = JSON.parse(res.result.content[0].text);
      expect(output.results).toBeInstanceOf(Array);
      expect(output.results.length).toBeGreaterThan(0);
      expect(output.results[0]).toHaveProperty("content");
      expect(output.results[0]).toHaveProperty("score");
    } finally {
      proc.kill();
    }
  });

  it.skipIf(!qdrantAvailable)("tools/call memory_delete removes an entry", async () => {
    const proc = spawnMcp();
    try {
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      // Store first
      const storeRes = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 2,
        method: "tools/call",
        params: {
          name: "memory_store",
          arguments: { content: `delete test ${Date.now()}` },
        },
      });
      const storeOutput = JSON.parse(storeRes.result.content[0].text);
      const entryId = storeOutput.id;
      expect(entryId).toBeDefined();

      // Delete
      const deleteRes = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 3,
        method: "tools/call",
        params: {
          name: "memory_delete",
          arguments: { id: entryId },
        },
      });

      expect(deleteRes.result).toBeDefined();
      const deleteOutput = JSON.parse(deleteRes.result.content[0].text);
      expect(deleteOutput.deleted).toBe(true);
    } finally {
      proc.kill();
    }
  });

  it.skipIf(!qdrantAvailable)("tools/call memory_list returns recent entries", async () => {
    const proc = spawnMcp();
    try {
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      const res = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 2,
        method: "tools/call",
        params: {
          name: "memory_list",
          arguments: { limit: 5 },
        },
      });

      expect(res.result).toBeDefined();
      const output = JSON.parse(res.result.content[0].text);
      expect(output.entries).toBeInstanceOf(Array);
    } finally {
      proc.kill();
    }
  });

  it("tools/list includes memory_delete and memory_list", async () => {
    const proc = spawnMcp();
    try {
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      const res = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 2,
        method: "tools/list",
        params: {},
      });

      const names = res.result.tools.map((t) => t.name);
      expect(names).toContain("memory_delete");
      expect(names).toContain("memory_list");
      expect(names).toContain("memory_export");
      expect(names).toContain("memory_gc");
    } finally {
      proc.kill();
    }
  });

  it.skipIf(!qdrantAvailable)("tools/call memory_gc removes expired entries", async () => {
    const proc = spawnMcp();
    try {
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      const res = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 2,
        method: "tools/call",
        params: {
          name: "memory_gc",
          arguments: {},
        },
      });

      expect(res.result).toBeDefined();
      const output = JSON.parse(res.result.content[0].text);
      expect(output).toHaveProperty("removed");
      expect(typeof output.removed).toBe("number");
    } finally {
      proc.kill();
    }
  });

  it.skipIf(!qdrantAvailable)("tools/call memory_export returns entries", async () => {
    const proc = spawnMcp();
    try {
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      // Store something first
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 2,
        method: "tools/call",
        params: {
          name: "memory_store",
          arguments: { content: `Export test ${Date.now()}` },
        },
      });

      const res = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 3,
        method: "tools/call",
        params: {
          name: "memory_export",
          arguments: {},
        },
      });

      expect(res.result).toBeDefined();
      const output = JSON.parse(res.result.content[0].text);
      expect(output.entries).toBeGreaterThan(0);
      expect(typeof output.data).toBe("string");
      // Data should contain JSONL
      const firstLine = JSON.parse(output.data.split("\n")[0]);
      expect(firstLine).toHaveProperty("id");
      expect(firstLine).toHaveProperty("content");
      expect(firstLine).toHaveProperty("namespace");
    } finally {
      proc.kill();
    }
  });

  it.skipIf(!qdrantAvailable)("tools/call memory_store with ttl sets expiry", async () => {
    const proc = spawnMcp();
    try {
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      const res = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 2,
        method: "tools/call",
        params: {
          name: "memory_store",
          arguments: { content: `TTL test ${Date.now()}`, ttl: 3600 },
        },
      });

      expect(res.result).toBeDefined();
      const output = JSON.parse(res.result.content[0].text);
      expect(output.status).toBe("stored");
      expect(output.id).toBeDefined();
    } finally {
      proc.kill();
    }
  });

  it("tools/call with unavailable engine returns graceful error", { timeout: 30_000 }, async () => {
    const proc = spawnMcp({ SHABTI_QDRANT_URL: "http://localhost:19999" });
    try {
      await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test", version: "1.0" },
        },
      });

      // memory_store should return error, not crash
      // Engine init with unreachable Qdrant can take 10+ seconds
      const storeRes = await sendRequest(
        proc,
        {
          jsonrpc: "2.0",
          id: 2,
          method: "tools/call",
          params: {
            name: "memory_store",
            arguments: { content: "test" },
          },
        },
        20_000,
      );
      expect(storeRes.error).toBeDefined();
      expect(storeRes.error.code).toBe(-32603);

      // memory_status should return unavailable status, not crash
      const statusRes = await sendRequest(proc, {
        jsonrpc: "2.0",
        id: 3,
        method: "tools/call",
        params: {
          name: "memory_status",
          arguments: {},
        },
      });
      expect(statusRes.result).toBeDefined();
      const output = JSON.parse(statusRes.result.content[0].text);
      expect(output.status).toBe("unavailable");
    } finally {
      proc.kill();
    }
  });

  it.skipIf(!qdrantAvailable)(
    "resources/read shabti://status returns JSON",
    { timeout: 15_000 },
    async () => {
      const proc = spawnMcp();
      try {
        await sendRequest(proc, {
          jsonrpc: "2.0",
          id: 1,
          method: "initialize",
          params: {
            protocolVersion: "2024-11-05",
            capabilities: {},
            clientInfo: { name: "test", version: "1.0" },
          },
        });

        const res = await sendRequest(proc, {
          jsonrpc: "2.0",
          id: 2,
          method: "resources/read",
          params: { uri: "shabti://status" },
        });

        expect(res.result).toBeDefined();
        expect(res.result.contents).toBeInstanceOf(Array);
        expect(res.result.contents[0].uri).toBe("shabti://status");
        const data = JSON.parse(res.result.contents[0].text);
        expect(data).toHaveProperty("entry_count");
      } finally {
        proc.kill();
      }
    },
  );
});
