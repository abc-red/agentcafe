# Codex Source Fixtures

Codex source fixtures should model the input side of MVP 1 scanner tests. They may contain fake sentinel values to prove redaction, but expected diagnostic reports must never include those values.

Recommended source scenarios:

| Scenario | Files to model | Expected report fixture |
| --- | --- | --- |
| Codex only | `config.toml` with whitelisted fields and MCP metadata. | `fixtures/diagnostic/reports/codex-only.json` |
| Malformed config | Invalid TOML syntax. | `fixtures/diagnostic/reports/malformed-config.json` |
| Secret redaction | MCP env fields such as `GITHUB_TOKEN = "AGENTCAFE_FIXTURE_SECRET_*"`. | `fixtures/diagnostic/reports/secret-redaction.json` |
| Project conflict | User and trusted project configs both define `model`. | `fixtures/diagnostic/reports/dual-runtime.json` |

Scanner tests must assert that Codex high-sensitivity paths are skipped by default:

- `~/.codex/sessions`
- raw rollout JSONL
- logs DB body fields
- goals / memories body fields
- auth / session / token / account state

MVP 1 may run `codex --version` for version detection only. It must not execute plugin commands, hook commands, MCP servers, install scripts, credential helpers, or any write operation.
