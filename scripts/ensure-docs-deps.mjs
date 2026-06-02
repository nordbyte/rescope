import { access } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";

const root = process.cwd();
const vitepressBin = path.join(root, "docs", "node_modules", ".bin", process.platform === "win32" ? "vitepress.cmd" : "vitepress");

try {
  await access(vitepressBin);
  process.exit(0);
} catch {
  const result = spawnSync(
    "npm",
    [
      "ci",
      "--prefix",
      "docs",
      "--cache",
      "/tmp/rescope-npm-cache",
      "--no-audit",
      "--no-fund"
    ],
    { stdio: "inherit" }
  );
  process.exit(result.status === null ? 1 : result.status);
}
