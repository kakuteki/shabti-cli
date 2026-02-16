# Shabti

Demo CLI tool — showcasing npm-publishable CLI structure.

## Install

```bash
npm install -g shabti
```

## Usage

```bash
shabti [command] [options]
```

## Commands

| Command        | Description                       |
| -------------- | --------------------------------- |
| `hello <name>` | Greet someone                     |
| `list`         | Show sample task list             |
| `spin`         | Demo async operation with spinner |

### hello

```bash
shabti hello World
# ✔ Hello, World!

shabti hello World --shout
# HELLO, WORLD!
```

### list

```bash
# Table output
shabti list

# JSON output
shabti list --json

# Filter by status
shabti list --filter done
```

### spin

```bash
# Default 2000ms spinner
shabti spin

# Custom duration
shabti spin --duration 500
```

## Development

```bash
# Install dependencies
npm install

# Run tests
npm test

# Run linter
npm run lint

# Format code
npm run format

# Run full CI pipeline
npm run ci
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

[Apache 2.0](LICENSE) — Copyright 2026 Kaga Hinata
