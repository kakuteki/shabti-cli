# Shabti

Agent Memory OS — semantic memory for AI agents.

A Rust-powered memory engine with Node.js CLI that provides semantic search, deduplication, time-decay scoring, and graph-based memory linking for AI agents. Integrates with Claude Code, Cursor, and other tools via MCP and A2A protocols.

## Install

```bash
npm install -g shabti
```

### Prerequisites

Shabti requires a running Qdrant instance for vector storage.

**Option 1: Docker**

```bash
docker run -d --name qdrant -p 6333:6333 -p 6334:6334 qdrant/qdrant
```

**Option 2: Native binary (no Docker)**

Download the binary for your platform from [Qdrant releases](https://github.com/qdrant/qdrant/releases) and run it directly.

```bash
# Check for locally installed qdrant
shabti config setup --detect
```

**Verify the connection:**

```bash
shabti config setup --check
```

## Quick Start

```bash
# Store a memory
shabti store "Rust is a systems programming language"

# Store with namespace and TTL (auto-expires after 1 hour)
shabti store "temp note" --namespace work --ttl 3600

# Search memories
shabti search "systems programming"

# Search with score explanation and graph expansion
shabti search "programming" --explain --follow-links 2

# Check engine status
shabti status
```

## Commands

| Command           | Description                         |
| ----------------- | ----------------------------------- |
| `store <content>` | Store a memory entry                |
| `search <query>`  | Search memory entries               |
| `get <id>`        | Retrieve a memory entry by ID       |
| `delete <id>`     | Delete a memory entry by ID         |
| `status`          | Show engine status                  |
| `export`          | Export memories as JSONL            |
| `import <file>`   | Import memories from JSONL          |
| `snapshot`        | Manage storage snapshots            |
| `config`          | Manage configuration                |
| `gc`              | Garbage collect expired entries     |
| `health`          | Run health checks on engine         |
| `a2a`             | Start A2A protocol server           |
| `chat`            | Interactive chat with OpenAI        |
| `mcp-config`      | Print MCP server configuration JSON |

### store

```bash
shabti store "Tokyo is the capital of Japan"
shabti store "meeting at 3pm" --namespace work
shabti store "buy groceries" --tags "todo,personal"
shabti store "temporary note" --ttl 3600    # expires in 1 hour
```

### search

```bash
shabti search "capital of Japan"
shabti search "meeting" --namespace work --limit 5
shabti search "programming" --explain           # show score breakdown
shabti search "AI" --follow-links 2             # expand via graph links
shabti search "recent events" --min-score 0.5
shabti search "query" --json                    # JSON output
```

### get

```bash
shabti get <uuid>
shabti get <uuid> --json
```

### delete

```bash
shabti delete <uuid>
```

### export / import

```bash
# Export all entries to stdout
shabti export

# Export to file with namespace filter
shabti export --namespace work --output backup.jsonl

# Import from JSONL file
shabti import backup.jsonl

# Import with namespace override and dry-run
shabti import data.jsonl --namespace imported --dry-run
```

### snapshot

```bash
shabti snapshot create
shabti snapshot list
shabti snapshot restore <id>
```

### config

```bash
shabti config show
shabti config show --json
shabti config set qdrant_url http://localhost:6334
shabti config setup           # show Qdrant setup instructions
shabti config setup --check   # test Qdrant connection
```

### gc

```bash
shabti gc    # remove entries past their TTL
```

## Interactive Mode (REPL)

Run `shabti` with no arguments in a terminal to enter interactive mode:

```
$ shabti

  [info] Memory engine connected (/remember, /recall available)
  [info] Interactive mode — model: gpt-4o-mini

you> /remember Tokyo is the capital of Japan
  [ok] Remembered: a1b2c3d4-...

you> /recall capital
  0.9234  Tokyo is the capital of Japan

you> /help
  /help        Show this help message
  /exit        Exit the REPL
  /clear       Clear conversation history
  /model       Show or switch the model
  /history     Show conversation history
  /remember    Store a memory
  /recall      Search memories
```

Requires `OPENAI_API_KEY` in `.env` for chat functionality. Memory commands (`/remember`, `/recall`) work with the local Qdrant engine.

## MCP Server

Shabti includes an MCP (Model Context Protocol) server for integration with Claude Code, Cursor, and other MCP-compatible tools.

```bash
# Start the MCP server (stdio transport)
npx shabti-mcp
```

### Claude Code / Cursor configuration

Generate the MCP settings JSON:

```bash
shabti mcp-config
```

Or manually add to your MCP settings:

```json
{
  "mcpServers": {
    "shabti-memory": {
      "command": "npx",
      "args": ["shabti-mcp"]
    }
  }
}
```

### MCP Tools

| Tool            | Description                                                 |
| --------------- | ----------------------------------------------------------- |
| `memory_store`  | Store a memory entry with optional namespace, tags, and TTL |
| `memory_search` | Search memories by semantic similarity                      |
| `memory_delete` | Delete a memory entry by ID                                 |
| `memory_list`   | List recent memory entries                                  |
| `memory_export` | Export entries as JSONL                                     |
| `memory_gc`     | Garbage collect expired entries                             |
| `memory_status` | Get engine status                                           |

### MCP Resources

| URI               | Description                  |
| ----------------- | ---------------------------- |
| `shabti://status` | Engine status and statistics |
| `shabti://config` | Current configuration        |

## A2A Protocol Server

Shabti supports Google's [Agent-to-Agent (A2A) protocol](https://google.github.io/A2A/) v0.3.0 for inter-agent communication.

```bash
# Start the A2A server (default port 3000)
shabti a2a

# Start on a custom port
shabti a2a --port 4000
```

### Agent Card

Discoverable at `GET /.well-known/agent-card.json`.

### Skills

| Skill           | Description                          |
| --------------- | ------------------------------------ |
| `memory_store`  | Store a memory entry via A2A message |
| `memory_search` | Semantic search via A2A message      |
| `memory_status` | Engine status via A2A message        |

### JSON-RPC Methods

| Method         | Description                      |
| -------------- | -------------------------------- |
| `message/send` | Send a message to invoke a skill |
| `tasks/get`    | Retrieve task status and results |
| `tasks/cancel` | Cancel a running task            |

### CORS

The A2A server sets `Access-Control-Allow-Origin: *` to allow requests from any origin. This is intentional for local development and inter-agent communication where the caller's origin is not known in advance. For production deployments behind a reverse proxy, restrict the origin at the proxy level.

## Node.js API

```javascript
import { ShabtiEngine } from "shabti";

const engine = new ShabtiEngine({
  qdrantUrl: "http://localhost:6334",
  collectionName: "my-app",
  dataDir: "./data",
});

// Store
await engine.store("Rust is a systems programming language", {
  namespace: "tech",
  tags: ["rust", "programming"],
  ttlSeconds: 86400, // expires in 24 hours
});

// Search
const results = await engine.executeQuery({
  text: "systems programming",
  limit: 10,
  withExplanation: true,
});

for (const r of results) {
  console.log(`${r.score.toFixed(4)}  ${r.content}`);
}

// Export
const entries = engine.listEntries({ namespace: "tech", limit: 100 });

// Garbage collect
const removed = await engine.gc();

await engine.shutdown();
```

## Configuration

### Environment Variables

| Variable            | Description                                           | Default                 |
| ------------------- | ----------------------------------------------------- | ----------------------- |
| `SHABTI_QDRANT_URL` | Override Qdrant URL                                   | `http://localhost:6334` |
| `SHABTI_LOG_LEVEL`  | Log level: `debug`, `info`, `warn`, `error`, `silent` | `info`                  |
| `SHABTI_LOG_JSON`   | Set to `1` for JSON-formatted log output              | —                       |
| `SHABTI_A2A_PORT`   | A2A server port (standalone mode)                     | `3000`                  |
| `OPENAI_API_KEY`    | Required for chat/REPL mode                           | —                       |

### Config File

Stored at `~/.shabti/config.json`. Manage via `shabti config` commands.

## Architecture

```
shabti (npm CLI + Node.js API)
  ├── src/commands/      12 CLI commands (Commander.js)
  ├── src/repl/          Interactive REPL with slash commands
  ├── src/mcp/           MCP server (stdio, 7 tools, 2 resources)
  ├── src/a2a/           A2A server (HTTP, JSON-RPC 2.0)
  ├── src/core/          Engine factory, session, retry logic
  ├── src/utils/         Style, validation, structured logger
  └── native.cjs         NAPI-RS bridge
       └── crates/
            ├── shabti-engine     Orchestration, store/search/gc
            ├── shabti-core       Data models, scoring, dedup, query DSL
            ├── shabti-embedding  fastembed-rs (MultilingualE5Small, 384-dim)
            ├── shabti-index      Qdrant vector DB client
            ├── shabti-storage    Append-only log, event store, snapshots
            ├── shabti-graph      k-NN memory link graph
            └── shabti-napi       NAPI-RS bindings
```

## Benchmarks

See [BENCHMARKS.md](BENCHMARKS.md) for detailed results.

| Metric     | Target   | Measured    |
| ---------- | -------- | ----------- |
| Recall@10  | >= 0.85  | **1.000**   |
| Insert p99 | <= 100ms | **60.22ms** |
| Search p99 | <= 100ms | **58.71ms** |

## Development

```bash
npm install          # install JS dependencies
cargo build          # build Rust workspace
npm test             # run JS tests (vitest) — 129 tests
cargo test           # run Rust tests — 26 test modules
npm run lint         # eslint
npm run ci           # lint + format check + audit + coverage
```

## License

[Apache 2.0](LICENSE) — Copyright 2026 Kaga Hinata
