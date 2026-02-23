import { readFileSync } from "fs";

const { version } = JSON.parse(
  readFileSync(new URL("../../package.json", import.meta.url), "utf8"),
);

export function buildAgentCard(baseUrl) {
  return {
    protocolVersion: "0.3.0",
    name: "Shabti Memory Engine",
    description: "A2A-compatible memory agent providing storage, search, and status operations",
    url: baseUrl,
    version,
    capabilities: {
      streaming: false,
      pushNotifications: false,
      stateTransitionHistory: false,
    },
    defaultInputModes: ["text", "application/json"],
    defaultOutputModes: ["application/json"],
    skills: [
      {
        id: "memory_store",
        name: "Store Memory",
        description: "Persist a memory entry with content and optional metadata tags",
        tags: ["memory", "store", "save", "remember"],
        inputModes: ["text", "application/json"],
        outputModes: ["application/json"],
      },
      {
        id: "memory_search",
        name: "Search Memories",
        description: "Search stored memories using semantic similarity queries",
        tags: ["memory", "search", "recall", "find", "query"],
        inputModes: ["text", "application/json"],
        outputModes: ["application/json"],
      },
      {
        id: "memory_status",
        name: "Memory Status",
        description: "Get status information about the memory engine",
        tags: ["memory", "status", "health", "stats"],
        inputModes: ["text"],
        outputModes: ["application/json"],
      },
      {
        id: "memory_get",
        name: "Get Memory",
        description: "Retrieve a specific memory entry by its ID",
        tags: ["memory", "get", "retrieve", "fetch"],
        inputModes: ["application/json"],
        outputModes: ["application/json"],
      },
      {
        id: "memory_delete",
        name: "Delete Memory",
        description: "Delete a specific memory entry by its ID",
        tags: ["memory", "delete", "remove"],
        inputModes: ["application/json"],
        outputModes: ["application/json"],
      },
      {
        id: "memory_list",
        name: "List Memories",
        description: "List stored memory entries with optional filters",
        tags: ["memory", "list", "entries"],
        inputModes: ["text", "application/json"],
        outputModes: ["application/json"],
      },
      {
        id: "memory_export",
        name: "Export Memories",
        description: "Export all memory entries as JSONL data",
        tags: ["memory", "export", "dump", "backup"],
        inputModes: ["text", "application/json"],
        outputModes: ["application/json"],
      },
      {
        id: "memory_gc",
        name: "Garbage Collect",
        description: "Remove expired memory entries and reclaim storage",
        tags: ["memory", "gc", "garbage", "cleanup"],
        inputModes: ["text"],
        outputModes: ["application/json"],
      },
      {
        id: "memory_health",
        name: "Health Check",
        description: "Check the health of the memory engine and its dependencies",
        tags: ["memory", "health", "diagnostics"],
        inputModes: ["text"],
        outputModes: ["application/json"],
      },
    ],
  };
}
