# capx

[![Crates.io](https://img.shields.io/crates/v/capx.svg)](https://crates.io/crates/capx)
[![GitHub stars](https://img.shields.io/github/stars/JungHoonGhae/capx)](https://github.com/JungHoonGhae/capx/stargazers)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/JungHoonGhae/capx/blob/main/LICENSE)

| [<img alt="GitHub Follow" src="https://img.shields.io/github/followers/JungHoonGhae?style=flat-square&logo=github&labelColor=black&color=24292f" width="156px" />](https://github.com/JungHoonGhae) | Follow [@JungHoonGhae](https://github.com/JungHoonGhae) on GitHub for more projects. |
| :-----| :----- |
| [<img alt="X link" src="https://img.shields.io/badge/Follow-%40lucas_ghae-000000?style=flat-square&logo=x&labelColor=black" width="156px" />](https://x.com/lucas_ghae) | Follow [@lucas_ghae](https://x.com/lucas_ghae) on X for updates. |

Unofficial CLI for [Capacities.io](https://capacities.io) — the first tool that gives you **full read/write access** to your Capacities data from the terminal.

> **Disclaimer**: This is an independent community CLI tool. It is not affiliated with, endorsed by, or sponsored by Capacities. Capacities is a trademark of its respective owners.

## Why capx?

### The problem: Capacities has no real API

The [official Capacities API](https://api.capacities.io/docs) is in a very early stage. It offers only **5 endpoints**:

| Official API | What it can do |
|---|---|
| `GET /spaces` | List spaces |
| `GET /space-info` | List object types |
| `POST /lookup` | Search by title |
| `POST /save-weblink` | Save a URL |
| `POST /save-to-daily-note` | Append to daily note |

That's it. **No reading object content. No creating objects. No updating. No deleting.** You can put data in, but you can't get it back out.

### Existing tools hit the same wall

Community tools and MCP servers built on the official API inherit every limitation — they can only use those same 5 endpoints:

- **Can't read** — No access to object content, notes, or page bodies
- **Can't create** — No general object creation (only weblinks and daily notes)
- **Can't update or delete** — No mutation of existing data
- **Can't list objects** — No way to browse what's in a space
- **Requires Pro subscription** — The official API is paywalled
- **Strict rate limits** — 5 requests per 60 seconds on most endpoints

### capx solves this

**capx** uses the same internal Portal API that the Capacities desktop app uses. This gives you full access to everything the app can do:

| capx | Official API | MCP Servers |
|:----:|:------------:|:-----------:|
| List & search objects | Search only | Search only |
| **Read object content** | — | — |
| **Create any object type** | Weblinks only | Weblinks only |
| **Update objects** | — | — |
| **Delete / restore** | — | — |
| **Duplicate objects** | — | — |
| **Manage properties** | — | — |
| **Create tasks** | — | — |
| **Context linking** | — | — |
| **Markdown body** | — | — |
| Auto-auth from desktop app | Manual API token | Manual API token |
| No Pro subscription needed | Pro required | Pro required |
| No rate limits | 5 req/60s | 5 req/60s |

## Support

If this tool helps you, consider supporting its maintenance:

<a href="https://www.buymeacoffee.com/lucas.ghae">
  <img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="50">
</a>

## Features

- 🔍 **Search** — Search objects by title across your Capacities space
- 📖 **Read** — Get full object content rendered as markdown
- 📝 **Create** — Create any object type (Page, Person, Book, custom types...)
- ✏️ **Update** — Modify title, description, body, and properties
- 🗑️ **Delete & Restore** — Soft-delete and undo
- 📋 **Tasks** — Create tasks with status, priority, and context
- 📅 **Daily Notes** — Append to today's daily note
- 🔗 **Weblinks** — Save URLs with metadata
- 🏷️ **Properties** — Set key=value properties (dates, selects, strings)
- 🔄 **Context** — Link entities as backlinks by UUID or search term
- 📦 **JSON** — Machine-readable output for scripting and piping
- 🔐 **Auto-auth** — Token extracted from Capacities desktop app automatically

## Requirements

| Requirement | Version/Notes |
|-------------|---------------|
| Capacities Desktop App | macOS — logged in (token auto-extracted from cookies) |
| Rust | >= 1.70 (if building from source) |

## Installation

### Homebrew (macOS & Linux)

```sh
brew tap JungHoonGhae/capx
brew install capx
```

> **Note for Linux users**: Auto-auth is macOS-only (reads from Capacities desktop app cookies). On Linux, set `CAP_TOKEN` or pass `--token <TOKEN>` manually.

### Cargo

```sh
cargo install capx
```

### Binary

Download from [GitHub Releases](https://github.com/JungHoonGhae/capx/releases).

## Quick Start

```sh
# Auth is automatic — just have Capacities desktop app logged in

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
capx trash                          # List trashed items
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
