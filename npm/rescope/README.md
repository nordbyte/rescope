# rescope npm package

This package provides the `rescope` command for npm-based installs.

The JavaScript wrapper does not collect metrics. It locates and executes the native Rust binary for the current platform, passing all CLI arguments through unchanged.

Lookup order:

1. `RESCOPE_BINARY`
2. optional native package such as `@rescope/rescope-linux-x64`
3. `vendor/<target-triple>/rescope`
4. local `../../target/release/rescope`
5. local `../../target/debug/rescope`

For local development, build the binary first:

```bash
cargo build -p rescope-cli
node bin/rescope.js --help
```

Example:

```bash
node bin/rescope.js snapshot --group user --limit 10
```
