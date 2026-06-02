#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const child_process = require("child_process");

function targetTriple() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === "linux" && arch === "x64") return "x86_64-unknown-linux-gnu";
  if (platform === "linux" && arch === "arm64") return "aarch64-unknown-linux-gnu";
  if (platform === "darwin" && arch === "x64") return "x86_64-apple-darwin";
  if (platform === "darwin" && arch === "arm64") return "aarch64-apple-darwin";
  if (platform === "win32" && arch === "x64") return "x86_64-pc-windows-msvc";

  throw new Error(`Unsupported platform/arch: ${platform}/${arch}`);
}

function candidatePaths() {
  const triple = targetTriple();
  const exe = process.platform === "win32" ? "rescope.exe" : "rescope";
  const here = path.resolve(__dirname, "..");

  return [
    process.env.RESCOPE_BINARY,
    path.join(here, "vendor", triple, exe),
    path.resolve(here, "..", "..", "target", "release", exe),
    path.resolve(here, "..", "..", "target", "debug", exe)
  ].filter(Boolean);
}

let candidates;
try {
  candidates = candidatePaths();
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

const binary = candidates.find((candidate) => fs.existsSync(candidate));

if (!binary) {
  console.error("rescope native binary not found.");
  console.error(
    "Build it with `cargo build -p rescope-cli --release` or install a published npm package that includes the native binary."
  );
  console.error("Checked:");
  for (const candidate of candidates) {
    console.error(`- ${candidate}`);
  }
  process.exit(1);
}

const result = child_process.spawnSync(binary, process.argv.slice(2), {
  stdio: "inherit"
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status === null ? 1 : result.status);
