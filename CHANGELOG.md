# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.1] - 2026-03-07

### Added

- Daily note CRUD commands:
  - `capx daily get` — read daily note
  - `capx daily delete` — delete by marker / last N blocks
  - `capx daily set` — replace daily note content

### Fixed

- Allows repairing incorrectly appended daily note blocks (e.g., literal `\\n` sequences).

## [0.1.0] - 2026-03-03

### Added

- Initial release
- `spaces` — List all Capacities spaces
- `whoami` — Check authentication status
- `search` — Full-text search across a space
- `types` — List available object types (structures)
- `ls` — List objects with optional type filtering
- `get` — Retrieve objects by ID (formatted or raw)
- `create` — Create objects with properties and context
- `update` — Update object title, description, body, or properties
- `rm` — Soft-delete objects
- `undo` — Restore soft-deleted objects
- `dup` — Duplicate objects
- `trash` — List trashed items
- `link` — Save weblinks with metadata
- `daily` — Append text to today's daily note
- `task` — Create tasks with properties and context linking
- `context` — Add backlink entities to existing objects
- `--json` flag for machine-readable output
- `--space-id` and `--token` global overrides
- Auto-detect first space when `CAP_SPACE_ID` is not set

[0.1.1]: https://github.com/JungHoonGhae/capx/releases/tag/v0.1.1
[0.1.0]: https://github.com/JungHoonGhae/capx/releases/tag/v0.1.0
