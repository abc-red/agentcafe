# Diagnostic Fixtures

This directory contains MVP 1 diagnostic contract fixtures for `agentcafe doctor --json`.

## Report Fixtures

`reports/*.json` are already-redacted `DiagnosticReport` examples. They are intended for:

- CLI contract tests.
- sidecar IPC result tests.
- UI smoke tests.
- security regression checks for redaction.

Required MVP 1 report scenarios:

| Fixture | Purpose |
| --- | --- |
| `golden-sample.json` | File version of the `agentcafe doctor --json` golden sample from `docs/ipc-contract.md`. |
| `codex-only.json` | Codex available, Claude missing, static MCP metadata only. |
| `claude-only.json` | Claude available, Codex missing, skill inventory summary. |
| `dual-runtime.json` | Both runtimes available, config conflict, plugin, skills, MCP, hook risk. |
| `no-runtime.json` | Neither runtime available; scanner still returns a valid report. |
| `malformed-config.json` | Parse failure is represented as invalid source plus `config_invalid` finding. |
| `secret-redaction.json` | Secret-like fields become redacted evidence only. |
| `permission-denied.json` | Permission failure is degraded into a finding; scan does not crash. |
| `large-scan.json` | Compact smoke fixture for the large scan UI state. Performance tests must generate the full scale case. |

## Large Scan Scale

The full large scan performance fixture must expand to at least:

- 100 plugins.
- 200 skills.
- 50 MCP servers.
- 1000 config-related files.

The compact `large-scan.json` keeps this repository readable and should not be used as the only performance proof.

## Redaction Rule

Report fixtures must never contain real or fake secret values, prompt bodies, transcript bodies, tool payloads, shell output bodies, nonce values, or full private paths. Source fixtures may contain fake sentinel secrets when needed, but expected reports must only include field names, lengths, hashes, counts, stable codes, and redacted paths.

Validate all report fixtures with:

```sh
node tests/integration/validate-diagnostic-fixtures.mjs
```
