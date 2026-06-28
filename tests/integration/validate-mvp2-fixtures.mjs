#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../..");

const groups = [
  {
    name: "diff",
    schemaPath: "schemas/config-diff.schema.json",
    fixtureDir: "fixtures/mvp2/diff",
    invariants: validateDiff
  },
  {
    name: "apply",
    schemaPath: "schemas/config-apply.schema.json",
    fixtureDir: "fixtures/mvp2/apply",
    invariants: validateApply
  },
  {
    name: "snapshot",
    schemaPath: "schemas/snapshot-manifest.schema.json",
    fixtureDir: "fixtures/mvp2/snapshot",
    invariants: validateSnapshot
  }
];

let failed = false;

for (const group of groups) {
  const schema = readJson(path.join(repoRoot, group.schemaPath));
  const dir = path.join(repoRoot, group.fixtureDir);
  const fixtures = fs
    .readdirSync(dir)
    .filter((name) => name.endsWith(".json"))
    .sort();

  if (fixtures.length === 0) {
    failed = true;
    console.error(`No ${group.name} fixtures found in ${group.fixtureDir}`);
    continue;
  }

  for (const fixtureName of fixtures) {
    const fixturePath = path.join(dir, fixtureName);
    const fixture = readJson(fixturePath);
    const errors = [
      ...validateSchema(fixture, schema, schema, "$"),
      ...group.invariants(fixture),
      ...validateNoForbiddenLeakage(fixture)
    ];

    if (errors.length > 0) {
      failed = true;
      console.error(`\n${path.relative(repoRoot, fixturePath)}`);
      for (const error of errors) {
        console.error(`  - ${error}`);
      }
    } else {
      console.log(`ok ${path.relative(repoRoot, fixturePath)}`);
    }
  }
}

process.exit(failed ? 1 : 0);

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function validateSchema(value, node, rootSchema, location) {
  if (node.$ref) {
    return validateSchema(value, resolveRef(node.$ref, rootSchema), rootSchema, location);
  }

  if (node.anyOf) {
    const branchErrors = node.anyOf.map((branch) => validateSchema(value, branch, rootSchema, location));
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
      errors.push(...validateSchema(item, node.items, rootSchema, `${location}[${index}]`));
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
        errors.push(...validateSchema(childValue, properties[key], rootSchema, `${location}.${key}`));
      } else if (node.additionalProperties === false) {
        errors.push(`${location}.${key} is not allowed`);
      }
    }
  }

  return errors;
}

function resolveRef(ref, rootSchema) {
  const prefix = "#/$defs/";
  if (!ref.startsWith(prefix)) {
    throw new Error(`Unsupported $ref: ${ref}`);
  }
  const name = ref.slice(prefix.length);
  const resolved = rootSchema.$defs?.[name];
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

function validateDiff(fixture) {
  const errors = [];

  if (new Date(fixture.expires_at) <= new Date(fixture.generated_at)) {
    errors.push("expires_at must be after generated_at");
  }

  if (fixture.would_execute_commands !== false) {
    errors.push("config.diff must not execute commands");
  }

  if (fixture.status === "ready_for_confirmation" && fixture.error !== null) {
    errors.push("ready_for_confirmation diff must not include error");
  }

  if (fixture.status !== "ready_for_confirmation" && fixture.error === null) {
    errors.push("blocked or invalid diff must include error");
  }

  if (!fixture.requires_snapshot || !fixture.requires_confirmation) {
    errors.push("MVP2 write diffs must require snapshot and confirmation");
  }

  return errors;
}

function validateApply(fixture) {
  const errors = [];

  if (fixture.status === "applied") {
    if (!fixture.snapshot_id) {
      errors.push("applied result must include snapshot_id");
    }
    if (!fixture.restore_available) {
      errors.push("applied result must keep restore_available true");
    }
    if (fixture.error !== null) {
      errors.push("applied result must not include error");
    }
  } else if (fixture.error === null) {
    errors.push("non-applied result must include stable error");
  }

  if (fixture.status === "blocked" && fixture.snapshot_id !== null) {
    errors.push("blocked pre-write result must not include snapshot_id");
  }

  if (fixture.status === "restore_failed" && fixture.error?.code !== "restore_failed") {
    errors.push("restore_failed status must include restore_failed error code");
  }

  return errors;
}

function validateSnapshot(fixture) {
  const errors = [];

  if (fixture.storage_policy.location_kind !== "app_private_data") {
    errors.push("snapshot storage must use app_private_data");
  }

  if (fixture.storage_policy.permissions !== "current_user_read_write") {
    errors.push("snapshot storage must be current_user_read_write");
  }

  if (fixture.restore_status === "restore_failed" && fixture.restore_error === null) {
    errors.push("restore_failed manifest must include restore_error");
  }

  if (fixture.restore_status !== "restore_failed" && fixture.restore_error !== null) {
    errors.push("non-failed restore manifest must not include restore_error");
  }

  for (const item of fixture.items) {
    if (item.path.includes("/payload") || item.path.includes("\\payload")) {
      errors.push(`snapshot item ${item.source_id} path must describe source path, not payload path`);
    }
  }

  return errors;
}

function validateNoForbiddenLeakage(fixture) {
  const errors = [];
  const text = JSON.stringify(fixture);
  const forbiddenLiterals = [
    "AGENTCAFE_FIXTURE_SECRET",
    "Bearer ",
    "-----BEGIN",
    "primaryApiKey:",
    "tool payload body",
    "transcript body",
    "shell output body",
    "one-time-startup-nonce",
    "snapshot payload content"
  ];

  for (const literal of forbiddenLiterals) {
    if (text.includes(literal)) {
      errors.push(`fixture contains forbidden literal ${JSON.stringify(literal)}`);
    }
  }

  return errors;
}
