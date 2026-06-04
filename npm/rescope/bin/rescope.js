#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const child_process = require("child_process");

function targetTriple(platform = process.platform, arch = process.arch) {
  if (platform === "linux" && arch === "x64") return "x86_64-unknown-linux-gnu";
  if (platform === "linux" && arch === "arm64") return "aarch64-unknown-linux-gnu";
  if (platform === "darwin" && arch === "x64") return "x86_64-apple-darwin";
  if (platform === "darwin" && arch === "arm64") return "aarch64-apple-darwin";
  if (platform === "win32" && arch === "x64") return "x86_64-pc-windows-msvc";

  throw new Error(`Unsupported platform/arch: ${platform}/${arch}`);
}

function candidatePaths(env = process.env, platform = process.platform, arch = process.arch) {
  const triple = targetTriple(platform, arch);
  const exe = platform === "win32" ? "rescope.exe" : "rescope";
  const here = path.resolve(__dirname, "..");
  const optionalPackage = optionalPackageName(platform, arch);

  return [
    env.RESCOPE_BINARY,
    optionalPackage && resolveOptionalPackageBinary(optionalPackage, exe),
    path.join(here, "vendor", triple, exe),
    path.resolve(here, "..", "..", "target", "release", exe),
    path.resolve(here, "..", "..", "target", "debug", exe)
  ].filter(Boolean);
}

function optionalPackageName(platform = process.platform, arch = process.arch) {
  if (platform === "linux" && arch === "x64") return "@rescope/rescope-linux-x64";
  if (platform === "linux" && arch === "arm64") return "@rescope/rescope-linux-arm64";
  if (platform === "darwin" && arch === "x64") return "@rescope/rescope-darwin-x64";
  if (platform === "darwin" && arch === "arm64") return "@rescope/rescope-darwin-arm64";
  if (platform === "win32" && arch === "x64") return "@rescope/rescope-win32-x64";

  return null;
}

function resolveOptionalPackageBinary(packageName, exe) {
  try {
    const packageJson = require.resolve(`${packageName}/package.json`);
    return path.join(path.dirname(packageJson), "bin", exe);
  } catch {
    return null;
  }
}

function main() {
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

  const child = child_process.spawn(binary, process.argv.slice(2), {
    stdio: "inherit"
  });

  child.on("error", (error) => {
    console.error(error.message);
    process.exit(1);
  });

  for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"]) {
    process.on(signal, () => {
      if (!child.killed) {
        child.kill(signal);
      }
    });
  }

  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }
    process.exit(code === null ? 1 : code);
  });
}

if (require.main === module) {
  main();
}

module.exports = {
  candidatePaths,
  optionalPackageName,
  targetTriple
};
