# capx

Unofficial CLI for [Capacities.io](https://capacities.io).

## Install

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

## Authentication

Set your Capacities API token:

```sh
export CAP_TOKEN="your-token-here"
```

Optionally set a default space:

```sh
export CAP_SPACE_ID="your-space-uuid"
```

## Usage

```sh
# List spaces
capx spaces

# Check auth
capx whoami

# Search
capx search "query"

# List object types
capx types

# List objects (optionally filter by type)
capx ls
capx ls -t Page

# Get object by ID
capx get <uuid>

# Create an object
capx create Page "My Title" -d "Description" -b "Body text"

# Update an object
capx update <uuid> -t "New Title"

# Delete / restore
capx rm <uuid>
capx undo <uuid>

# Save a weblink
capx link https://example.com

# Append to daily note
capx daily "Some text"

# Create a task
capx task "Buy groceries" -b "Milk, eggs"

# Add context to an object
capx context <uuid> <entity-uuid-or-search-term>
```

### Global Options

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--space-id <UUID>` | Override space (or set `CAP_SPACE_ID`) |
| `--token <TOKEN>` | Override token (or set `CAP_TOKEN`) |

## License

MIT
