# Claude Source Fixtures

Claude source fixtures should model the input side of MVP 1 scanner tests. They may contain fake sentinel values to prove redaction, but expected diagnostic reports must never include those values.

Recommended source scenarios:

| Scenario | Files to model | Expected report fixture |
| --- | --- | --- |
| Claude only | `settings.json` with whitelisted fields and a skill frontmatter summary. | `fixtures/diagnostic/reports/claude-only.json` |
| Secret redaction | MCP headers/env and `config.json` primary key presence. | `fixtures/diagnostic/reports/secret-redaction.json` |
| Permission denied | Managed settings path exists but cannot be read. | `fixtures/diagnostic/reports/permission-denied.json` |
| Dual runtime | User settings, project settings, plugin metadata, skill metadata, MCP metadata, hook metadata. | `fixtures/diagnostic/reports/dual-runtime.json` |

Scanner tests must assert that Claude high-sensitivity paths are skipped by default:

- `~/.claude/projects/**/*.jsonl`
- `~/.claude/file-history`
- `~/.claude/debug`
- transcript bodies
- tool payloads
- shell output bodies
- `~/.claude/config.json.primaryApiKey` value
- account, login, OAuth, and project private state in `~/.claude.json`

MVP 1 may run `claude --version` for version detection only. It must not execute plugin commands, hook commands, MCP servers, API key helpers, install scripts, or any write operation.
