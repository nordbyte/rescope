# rescope npm package

This package provides the `rescope` command for npm-based installs.

The JavaScript wrapper does not collect metrics. It locates and executes the native Rust binary for the current platform, passing all CLI arguments through unchanged.

Lookup order:

1. `RESCOPE_BINARY`
2. optional native package such as `@rescope/rescope-linux-x64`
3. `vendor/<target-triple>/rescope`
4. local `../../target/release/rescope`
5. local `../../target/debug/rescope`

Native package metadata is scaffolded in `npm/native` for the supported Linux, macOS and Windows targets. Those packages are intended for platform-specific binary publishing.

For local development, build the binary first:

```bash
cargo build -p rescope-cli
node bin/rescope.js --help
npm test
```

Example:

```bash
node bin/rescope.js snapshot --group user --limit 10
```
