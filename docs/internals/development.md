# Development

## Rust checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Offline checks can be useful in restricted environments:

```bash
cargo test --workspace --offline
```

## Manual smoke tests

```bash
cargo run -p rescope-cli -- snapshot --limit 5
cargo run -p rescope-cli -- snapshot --group parent --limit 5
cargo run -p rescope-cli -- live --once --json -
cargo run -p rescope-cli -- record --duration 5s --interval 1s --limit 5
```

## Docs

```bash
npm run docs:check
npm run docs:build
npm run docs:dev
```

The docs are built from `docs/` and deployed by `.github/workflows/pages.yml`.
