import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const fixtureDir = path.join(repoRoot, "fixtures", "diagnostic", "reports");
const requiredTopLevel = [
  "schema_version",
  "trace_id",
  "generated_at",
  "runtimes",
  "config_sources",
  "plugins",
  "skills",
  "mcp_servers",
  "hooks",
  "conflicts",
  "risk_findings",
  "summary",
  "redaction_notice"
];

const requiredSections = [
  "总览",
  "Codex助手",
  "Claude助手",
  "MCP",
  "Plugins",
  "Skills",
  "风险",
  "备份",
  "诊断详情"
];

const forbiddenWritableMethods = [
  ["config", "diff"],
  ["config", "apply"],
  ["backup", "create"],
  ["backup", "restore"],
  ["mcp", "test"],
  ["plugin", "enable"],
  ["plugin", "disable"],
  ["skill", "create"]
];

for (const file of fs.readdirSync(fixtureDir).filter((name) => name.endsWith(".json"))) {
  const fullPath = path.join(fixtureDir, file);
  const report = JSON.parse(fs.readFileSync(fullPath, "utf8"));
  for (const key of requiredTopLevel) {
    assert(key in report, `${file} is missing ${key}`);
  }
  assert(report.schema_version === "agentcafe.diagnostic.v1", `${file} has an unsupported schema`);
  assert(Array.isArray(report.runtimes), `${file} runtimes must be an array`);
  assert(Array.isArray(report.config_sources), `${file} config_sources must be an array`);
  assert(Array.isArray(report.plugins), `${file} plugins must be an array`);
  assert(Array.isArray(report.skills), `${file} skills must be an array`);
  assert(Array.isArray(report.mcp_servers), `${file} mcp_servers must be an array`);
  assert(Array.isArray(report.risk_findings), `${file} risk_findings must be an array`);
}

const windowsSource = readTree(path.join(repoRoot, "apps", "windows-wpf"));
const macSource = readTree(path.join(repoRoot, "apps", "macos", "Sources", "AgentCafeMac"));

for (const section of requiredSections) {
  assert(windowsSource.includes(section), `Windows UI is missing section ${section}`);
  assert(macSource.includes(section), `macOS UI is missing section ${section}`);
}

for (const source of [windowsSource, macSource]) {
  assert(source.includes("ipc.handshake"), "UI must perform ipc.handshake");
  assert(source.includes("doctor.run"), "UI must call doctor.run");
  for (const [namespace, action] of forbiddenWritableMethods) {
    const method = `${namespace}.${action}`;
    assert(!source.includes(`\"${method}\"`), `UI must not call writable method ${method}`);
  }
}

console.log("Native UI contracts validated.");

function readTree(directory) {
  let output = "";
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      output += readTree(fullPath);
    } else if (/\.(cs|xaml|swift)$/.test(entry.name)) {
      output += fs.readFileSync(fullPath, "utf8");
    }
  }
  return output;
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
