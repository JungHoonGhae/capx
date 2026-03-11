# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-03-10

### Changed

- Renamed project from `capx` to `capacities-cli` (crates.io name); CLI command remains `capx`
- Repository moved to `JungHoonGhae/capacities-cli`

### Added

- New commands: `export`, `edit`, `doctor`, `auth`, `completions`
- 7 new block types: checklist, bullet list, numbered list, quote, callout, toggle, image
- Entity UUID preservation in wiki-links: `[[Name|uuid]]`
- Structured error types (`CapxError`) with JSON serialization support (`--json` flag)
- Cross-platform auth: macOS, Linux, and Windows cookie DB extraction
- `--appversion` / `CAP_APPVERSION` and `--portal-url` / `CAP_PORTAL_URL` config overrides
- 29 new unit tests for `blocks.rs` edge cases (141 total: 133 unit + 8 integration)
- `cargo-audit` added to CI security checks

### Fixed

- Removed dangerous `unwrap()` calls in `api.rs`
- Added 30-second request timeout
- Automatic retry on HTTP 429 and 5xx responses
- Improved API error messages with contextual endpoint details
- CI: hardened Doppler token fetch, suppress stderr to prevent leakage
- CI: skip Homebrew tap update when token is not configured

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

[0.2.0]: https://github.com/JungHoonGhae/capacities-cli/releases/tag/v0.2.0
[0.1.1]: https://github.com/JungHoonGhae/capx/releases/tag/v0.1.1
[0.1.0]: https://github.com/JungHoonGhae/capx/releases/tag/v0.1.0
