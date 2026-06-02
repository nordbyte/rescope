const assert = require("assert");

const { candidatePaths, optionalPackageName, targetTriple } = require("../bin/rescope.js");

assert.strictEqual(targetTriple("linux", "x64"), "x86_64-unknown-linux-gnu");
assert.strictEqual(targetTriple("linux", "arm64"), "aarch64-unknown-linux-gnu");
assert.strictEqual(targetTriple("darwin", "x64"), "x86_64-apple-darwin");
assert.strictEqual(targetTriple("darwin", "arm64"), "aarch64-apple-darwin");
assert.strictEqual(targetTriple("win32", "x64"), "x86_64-pc-windows-msvc");
assert.throws(() => targetTriple("freebsd", "x64"), /Unsupported platform\/arch/);

assert.strictEqual(optionalPackageName("linux", "x64"), "@rescope/rescope-linux-x64");
assert.strictEqual(optionalPackageName("win32", "x64"), "@rescope/rescope-win32-x64");
assert.strictEqual(optionalPackageName("freebsd", "x64"), null);

const candidates = candidatePaths(
  { RESCOPE_BINARY: "/tmp/custom-rescope" },
  "linux",
  "x64"
);
assert.strictEqual(candidates[0], "/tmp/custom-rescope");
assert(candidates.some((candidate) => candidate.includes("x86_64-unknown-linux-gnu")));

const windowsCandidates = candidatePaths({}, "win32", "x64");
assert(windowsCandidates.every((candidate) => candidate.endsWith("rescope.exe")));
