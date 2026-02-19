# Shabti

Agent Memory OS — semantic memory for AI agents.

A Rust-powered memory engine with Node.js CLI that provides semantic search, deduplication, time-decay scoring, and graph-based memory linking for AI agents.

## Install

```bash
npm install -g shabti
```

### Prerequisites

Shabti requires a running Qdrant instance for vector storage:

```bash
docker run -d --name qdrant -p 6333:6333 -p 6334:6334 qdrant/qdrant
```

Verify the connection:

```bash
shabti config setup --check
```

## Quick Start

```bash
# Store a memory
shabti store "Rust is a systems programming language"

# Search memories
shabti search "systems programming"

# Search with score explanation
shabti search "programming" --explain

# Check engine status
shabti status
```

## Commands

| Command           | Description                  |
| ----------------- | ---------------------------- |
| `store <content>` | Store a memory entry         |
| `search <query>`  | Search memory entries        |
| `status`          | Show engine status           |
| `snapshot`        | Manage storage snapshots     |
| `config`          | Manage configuration         |
| `chat`            | Interactive chat with OpenAI |

### store

```bash
shabti store "Tokyo is the capital of Japan"
shabti store "meeting at 3pm" --namespace work
shabti store "buy groceries" --tags "todo,personal"
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

## Interactive Mode (REPL)

Run `shabti` with no arguments in a terminal to enter interactive mode:

```
$ shabti
  shabti v2.0.0

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

## Node.js API

```javascript
import { ShabtiEngine } from "shabti";

const engine = new ShabtiEngine({
  qdrantUrl: "http://localhost:6334",
  collectionName: "my-app",
  dataDir: "./data",
});

// Store
await engine.store("Rust is a systems programming language", {});

// Search
const results = await engine.executeQuery({
  text: "systems programming",
  limit: 10,
  withExplanation: true,
});

for (const r of results) {
  console.log(`${r.score.toFixed(4)}  ${r.content}`);
  if (r.explanation) {
    console.log(
      `  sim=${r.explanation.semanticSimilarity.toFixed(3)} ` +
        `decay=${r.explanation.timeDecayFactor.toFixed(3)} ` +
        `boost=${r.explanation.accessBoostFactor.toFixed(3)}`,
    );
  }
}

await engine.shutdown();
```

## Architecture

```
shabti (npm CLI + Node.js API)
  └── shabti-napi (Rust ↔ Node.js FFI via NAPI-RS)
       └── shabti-engine (orchestration layer)
            ├── shabti-embedding (fastembed-rs, MultilingualE5Small 384-dim)
            ├── shabti-index (Qdrant vector DB client)
            ├── shabti-storage (append-only log, event store, snapshots)
            ├── shabti-graph (k-NN memory link graph)
            └── shabti-core (data models, scoring, dedup, query DSL)
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
npm test             # run JS tests (vitest)
cargo test           # run Rust tests
npm run lint         # eslint
cargo clippy         # Rust linter
```

## License

[Apache 2.0](LICENSE) — Copyright 2026 Kaga Hinata
