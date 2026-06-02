# rescope npm package

This package provides the `rescope` command for npm-based installs.

The JavaScript wrapper does not collect metrics. It locates and executes the native Rust binary for the current platform, passing all CLI arguments through unchanged.

For local development, build the binary first:

```bash
cargo build -p rescope-cli
node bin/rescope.js --help
```
