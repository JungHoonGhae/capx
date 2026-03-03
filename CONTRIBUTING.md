# Contributing to capx

Thanks for your interest in contributing to capx!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/<your-username>/capx.git`
3. Create a branch: `git checkout -b my-feature`
4. Make your changes
5. Run checks: `cargo fmt --check && cargo clippy -- -D warnings && cargo build`
6. Commit and push
7. Open a Pull Request

## Development

### Prerequisites

- Rust 1.70+
- A Capacities.io API token (for integration testing)

### Build

```sh
cargo build
```

### Lint

```sh
cargo fmt --check
cargo clippy -- -D warnings
```

## Guidelines

- Run `cargo fmt` before committing
- Ensure `cargo clippy -- -D warnings` passes
- Keep PRs focused on a single change
- Add tests for new functionality where applicable

## Reporting Issues

- Use [GitHub Issues](https://github.com/JungHoonGhae/capx/issues)
- Include your OS, Rust version, and steps to reproduce

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
