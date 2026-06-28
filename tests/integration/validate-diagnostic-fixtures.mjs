#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../..");
const schemaPath = path.join(repoRoot, "schemas/diagnostic-report.schema.json");
const reportsDir = path.join(repoRoot, "fixtures/diagnostic/reports");

const schema = readJson(schemaPath);
const reports = fs
  .readdirSync(reportsDir)
  .filter((name) => name.endsWith(".json"))
  .sort();

let failed = false;

for (const reportName of reports) {
  const reportPath = path.join(reportsDir, reportName);
  const report = readJson(reportPath);
  const errors = [
    ...validateSchema(report, schema, "$"),
    ...validateMvp1Invariants(report),
    ...validateNoForbiddenLeakage(report)
  ];

  if (errors.length > 0) {
    failed = true;
    console.error(`\n${path.relative(repoRoot, reportPath)}`);
    for (const error of errors) {
      console.error(`  - ${error}`);
    }
  } else {
    console.log(`ok ${path.relative(repoRoot, reportPath)}`);
  }
}

if (reports.length === 0) {
  failed = true;
  console.error(`No diagnostic report fixtures found in ${path.relative(repoRoot, reportsDir)}`);
}

process.exit(failed ? 1 : 0);

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function validateSchema(value, node, location) {
  if (node.$ref) {
    return validateSchema(value, resolveRef(node.$ref), location);
  }

  if (node.anyOf) {
    const branchErrors = node.anyOf.map((branch) => validateSchema(value, branch, location));
    if (branchErrors.some((errors) => errors.length === 0)) {
      return [];
    }
    return [`${location} does not match any allowed schema branch`];
  }

  const errors = [];

  if (Object.hasOwn(node, "const") && value !== node.const) {
    errors.push(`${location} expected const ${JSON.stringify(node.const)}`);
  }

  if (node.enum && !node.enum.includes(value)) {
    errors.push(`${location} expected one of ${node.enum.map((item) => JSON.stringify(item)).join(", ")}`);
  }

  if (node.type && !matchesType(value, node.type)) {
    errors.push(`${location} expected type ${node.type}`);
    return errors;
  }

  if (typeof value === "string") {
    if (node.minLength !== undefined && value.length < node.minLength) {
      errors.push(`${location} shorter than minLength ${node.minLength}`);
    }
    if (node.maxLength !== undefined && value.length > node.maxLength) {
      errors.push(`${location} longer than maxLength ${node.maxLength}`);
    }
    if (node.pattern && !new RegExp(node.pattern).test(value)) {
      errors.push(`${location} does not match pattern ${node.pattern}`);
    }
    if (node.format === "date-time" && Number.isNaN(Date.parse(value))) {
      errors.push(`${location} is not a valid date-time`);
    }
    if (node.format === "uri") {
      try {
        new URL(value);
      } catch {
        errors.push(`${location} is not a valid uri`);
      }
    }
  }

  if (typeof value === "number") {
    if (node.minimum !== undefined && value < node.minimum) {
      errors.push(`${location} below minimum ${node.minimum}`);
    }
    if (node.maximum !== undefined && value > node.maximum) {
      errors.push(`${location} above maximum ${node.maximum}`);
    }
  }

  if (Array.isArray(value) && node.items) {
    value.forEach((item, index) => {
      errors.push(...validateSchema(item, node.items, `${location}[${index}]`));
    });
  }

  if (isPlainObject(value)) {
    const required = node.required ?? [];
    for (const key of required) {
      if (!Object.hasOwn(value, key)) {
        errors.push(`${location}.${key} is required`);
      }
    }

    const properties = node.properties ?? {};
    for (const [key, childValue] of Object.entries(value)) {
      if (Object.hasOwn(properties, key)) {
        errors.push(...validateSchema(childValue, properties[key], `${location}.${key}`));
      } else if (node.additionalProperties === false) {
        errors.push(`${location}.${key} is not allowed`);
      }
    }
  }

  return errors;
}

function resolveRef(ref) {
  const prefix = "#/$defs/";
  if (!ref.startsWith(prefix)) {
    throw new Error(`Unsupported $ref: ${ref}`);
  }
  const name = ref.slice(prefix.length);
  const resolved = schema.$defs?.[name];
  if (!resolved) {
    throw new Error(`Unknown $ref: ${ref}`);
  }
  return resolved;
}

function matchesType(value, type) {
  if (type === "array") return Array.isArray(value);
  if (type === "integer") return Number.isInteger(value);
  if (type === "null") return value === null;
  if (type === "object") return isPlainObject(value);
  return typeof value === type;
}

function isPlainObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function validateMvp1Invariants(report) {
  const errors = [];
  const riskCounts = { info: 0, low: 0, medium: 0, high: 0, critical: 0 };
  for (const finding of report.risk_findings) {
    riskCounts[finding.severity] += 1;
  }

  for (const [severity, expected] of Object.entries(riskCounts)) {
    const actual = report.summary.risk_count_by_severity[severity];
    if (actual !== expected) {
      errors.push(`summary.risk_count_by_severity.${severity} expected ${expected}, got ${actual}`);
    }
  }

  const availableRuntimes = report.runtimes.filter((runtime) => runtime.status === "available").length;
  const countChecks = [
    ["runtime_count", availableRuntimes],
    ["config_source_count", report.config_sources.length],
    ["plugin_count", report.plugins.length],
    ["skill_count", report.skills.length],
    ["mcp_server_count", report.mcp_servers.length],
    ["hook_count", report.hooks.length]
  ];

  for (const [field, expected] of countChecks) {
    if (report.summary[field] !== expected) {
      errors.push(`summary.${field} expected ${expected}, got ${report.summary[field]}`);
    }
  }

  for (const server of report.mcp_servers) {
    if (!["not_tested", "invalid", "unknown"].includes(server.connection_status)) {
      errors.push(`mcp_servers.${server.id}.connection_status must not imply an MVP 1 connection test`);
    }
    if (server.tool_count !== null || server.resource_count !== null || server.template_count !== null) {
      errors.push(`mcp_servers.${server.id} tool/resource/template counts must be null in MVP 1`);
    }
  }

  return errors;
}

function validateNoForbiddenLeakage(report) {
  const errors = [];
  const text = JSON.stringify(report);
  const forbiddenLiterals = [
    "AGENTCAFE_FIXTURE_SECRET",
    "Bearer ",
    "-----BEGIN",
    "primaryApiKey:",
    "tool payload body",
    "transcript body",
    "shell output body",
    "one-time-startup-nonce"
  ];

  for (const literal of forbiddenLiterals) {
    if (text.includes(literal)) {
      errors.push(`report contains forbidden literal ${JSON.stringify(literal)}`);
    }
  }

  return errors;
}
