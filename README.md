# capx

[![Crates.io](https://img.shields.io/crates/v/capx.svg)](https://crates.io/crates/capx)
[![GitHub stars](https://img.shields.io/github/stars/JungHoonGhae/capx)](https://github.com/JungHoonGhae/capx/stargazers)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/JungHoonGhae/capx/blob/main/LICENSE)

| [<img alt="GitHub Follow" src="https://img.shields.io/github/followers/JungHoonGhae?style=flat-square&logo=github&labelColor=black&color=24292f" width="156px" />](https://github.com/JungHoonGhae) | Follow [@JungHoonGhae](https://github.com/JungHoonGhae) on GitHub for more projects. |
| :-----| :----- |
| [<img alt="X link" src="https://img.shields.io/badge/Follow-%40lucas_ghae-000000?style=flat-square&logo=x&labelColor=black" width="156px" />](https://x.com/lucas_ghae) | Follow [@lucas_ghae](https://x.com/lucas_ghae) on X for updates. |

Unofficial CLI for [Capacities.io](https://capacities.io) — manage spaces, objects, tasks, and daily notes from your terminal.

> **Disclaimer**: This is an independent community CLI tool. It is not affiliated with, endorsed by, or sponsored by Capacities. Capacities is a trademark of its respective owners.

## About

Capacities.io is a powerful note-taking and knowledge management tool with a rich web interface. But sometimes you want to quickly capture a thought, create a task, or search your knowledge base without leaving the terminal. **capx** brings the Capacities API to your command line.

**What it does:**
- List and search spaces, objects, and types
- Create, update, and delete objects with properties
- Save weblinks and append to daily notes
- Create tasks with context linking
- Output as JSON for scripting and piping

## Support

If this tool helps you, consider supporting its maintenance:

<a href="https://www.buymeacoffee.com/lucas.ghae">
  <img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="50">
</a>

## Features

- 🔍 **Search** — Full-text search across your Capacities space
- 📝 **CRUD** — Create, read, update, and delete any object type
- 📋 **Tasks** — Create tasks with properties and context
- 📅 **Daily Notes** — Append to today's daily note
- 🔗 **Weblinks** — Save URLs with metadata
- 🏷️ **Properties** — Set key=value properties on objects
- 🔄 **Context** — Link entities as backlinks
- 📦 **JSON** — Machine-readable output for scripting

## Requirements

| Requirement | Version/Notes |
|-------------|---------------|
| Capacities API Token | [Get from Capacities settings](https://app.capacities.io/settings) |
| Rust | >= 1.70 (if building from source) |

## Installation

### Homebrew (macOS & Linux)

```sh
brew tap JungHoonGhae/capx
brew install capx
```

### Cargo

```sh
cargo install capx
```

### Binary

Download from [GitHub Releases](https://github.com/JungHoonGhae/capx/releases).

## Quick Start

```sh
# Set your API token
export CAP_TOKEN="your-token-here"

# List your spaces
capx spaces

# Search for content
capx search "meeting notes"

# Create a page
capx create Page "My New Page" -b "Some content here"

# Append to daily note
capx daily "Remember to review PR"
```

## Usage

### Spaces & Auth

```sh
capx spaces          # List all spaces
capx whoami          # Check authentication status
```

### Objects

```sh
capx types                          # List object types
capx ls                             # List objects
capx ls -t Page                     # Filter by type
capx get <uuid>                     # Get object by ID
capx create Page "Title" -b "Body"  # Create object
capx update <uuid> -t "New Title"   # Update object
capx rm <uuid>                      # Delete (soft)
capx undo <uuid>                    # Restore deleted
capx dup <uuid>                     # Duplicate
```

### Tasks & Notes

```sh
capx task "Buy groceries" -b "Milk, eggs"     # Create task
capx daily "Some text"                         # Append to daily note
capx link https://example.com -t "Example"     # Save weblink
```

### Context & Properties

```sh
capx context <uuid> <entity-or-search>         # Add backlink
capx create Page "Title" -p status=draft       # With properties
capx task "Review" --context "Project Name"    # Task with context
```

### Global Options

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--space-id <UUID>` | Override space (or set `CAP_SPACE_ID`) |
| `--token <TOKEN>` | Override token (or set `CAP_TOKEN`) |

## Documentation

| Resource | Link |
|----------|------|
| Crates.io | [crates.io/crates/capx](https://crates.io/crates/capx) |
| GitHub | [github.com/JungHoonGhae/capx](https://github.com/JungHoonGhae/capx) |
| Capacities API | [docs.capacities.io](https://docs.capacities.io) |

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT — See [LICENSE](https://github.com/JungHoonGhae/capx/blob/main/LICENSE) for details.
