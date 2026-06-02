# Installation

`rescope` ships as a native Rust binary. The npm package is a launcher that finds and executes that binary; it does not collect metrics in JavaScript.

## From source

```bash
git clone https://github.com/nordbyte/rescope.git
cd rescope
cargo build -p rescope-cli --release
./target/release/rescope --help
```

For development, use the debug binary:

```bash
cargo run -p rescope-cli -- snapshot --limit 10
```

## With Cargo

When published to crates.io:

```bash
cargo install rescope
```

## With npm

When published to npm:

```bash
npm install -g rescope
rescope --help
```

For local npm-wrapper testing:

```bash
cargo build -p rescope-cli
cd npm/rescope
node bin/rescope.js snapshot --limit 10
```

## Platforms

The project targets:

- Linux x86_64
- Linux aarch64
- macOS x86_64
- macOS aarch64
- Windows x86_64

Linux x86_64 is the primary MVP platform. Platform-specific metric gaps are reported as `unknown`, `n/a` or `0` rather than crashing.
