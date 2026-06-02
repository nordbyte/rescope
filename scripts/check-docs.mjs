import { access, readdir, readFile } from "node:fs/promises";
import path from "node:path";

const root = process.cwd();
const docsDir = path.join(root, "docs");
const requiredFiles = [
  "index.md",
  "start/install.md",
  "start/quickstart.md",
  "start/core-concepts.md",
  "start/troubleshooting.md",
  "guides/live.md",
  "guides/recording.md",
  "guides/filters-grouping.md",
  "guides/exports.md",
  "guides/privacy.md",
  "commands/index.md",
  "commands/snapshot.md",
  "commands/live.md",
  "commands/record.md",
  "commands/help-version.md",
  "reference/options.md",
  "reference/metrics.md",
  "reference/output-formats.md",
  "reference/exit-codes.md",
  "internals/architecture.md",
  "internals/development.md",
  "internals/github-pages.md"
];
const commandPages = ["snapshot", "live", "record", "help-version"];

let failed = false;

for (const file of requiredFiles) {
  await expectFile(path.join(docsDir, file), `missing docs/${file}`);
}

const commandIndex = await readFile(path.join(docsDir, "commands", "index.md"), "utf8");
for (const page of commandPages) {
  if (!commandIndex.includes(`](${page}.md)`)) {
    fail(`docs/commands/index.md does not link ${page}.md`);
  }
}

const options = await readFile(path.join(docsDir, "reference", "options.md"), "utf8");
for (const option of [
  "--all",
  "--normalize-cpu",
  "--json",
  "--csv",
  "--group",
  "--cmd",
  "--cmd-regex",
  "--config",
  "--exe",
  "--exe-regex",
  "--name-regex",
  "--parent",
  "--parent-name",
  "--parent-regex",
  "--profile",
  "--min-cpu",
  "--min-ram",
  "--min-io",
  "--invert",
  "--show-command"
]) {
  const optionPattern = new RegExp(`\`${escapeRegExp(option)}(?:\`|\\s)`);
  if (!optionPattern.test(options)) {
    fail(`docs/reference/options.md is missing ${option}`);
  }
}

const docsFiles = await listMarkdown(docsDir);
for (const file of docsFiles) {
  const source = await readFile(file, "utf8");
  for (const match of source.matchAll(/\]\(([^)]+\.md(?:#[^)]+)?)\)/g)) {
    const target = match[1].split("#")[0];
    const resolved = path.resolve(path.dirname(file), target);
    if (!resolved.startsWith(docsDir)) {
      fail(`${relative(file)} links outside docs: ${match[1]}`);
      continue;
    }
    await expectFile(resolved, `${relative(file)} links missing page ${match[1]}`);
  }
}

if (failed) {
  process.exit(1);
}

console.log(`checked ${docsFiles.length} docs pages`);

async function expectFile(file, message) {
  try {
    await access(file);
  } catch {
    fail(message);
  }
}

async function listMarkdown(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "node_modules" || entry.name === ".vitepress") continue;
      files.push(...await listMarkdown(full));
    } else if (entry.isFile() && entry.name.endsWith(".md")) {
      files.push(full);
    }
  }
  return files;
}

function relative(file) {
  return path.relative(root, file).replaceAll(path.sep, "/");
}

function fail(message) {
  failed = true;
  console.error(`docs check failed: ${message}`);
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
