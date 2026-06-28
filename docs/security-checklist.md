# Security Checklist

This checklist is the release gate companion for `docs/security-model.md`. It is intentionally written as a checkable list for PR review and MVP acceptance.

## MVP 1 Read-Only Gate

- [ ] `agentcafe doctor --json` does not write configuration files.
- [ ] No snapshot files or snapshot directories are created.
- [ ] MCP servers are not started.
- [ ] Hook commands, URLs, MCP tools, prompts, and agents are not executed.
- [ ] Plugin commands, installers, update scripts, and credential helpers are not executed.
- [ ] `mcp_servers[].connection_status` is limited to `not_tested`, `invalid`, or `unknown`.
- [ ] MCP `tool_count`, `resource_count`, and `template_count` remain `null` in MVP 1 reports.
- [ ] UI write/test actions are absent or disabled as MVP 2 draft.

## Redaction Gate

- [ ] API keys, tokens, cookies, Authorization headers, provider keys, passwords, private keys, and session values never appear in reports, logs, IPC responses, screenshots, or fixture expected reports.
- [ ] Secret-like findings use only field name, length, `sha256_12`, source summary, stable code, and redacted path.
- [ ] Parser errors are scrubbed before becoming user-visible messages.
- [ ] Full private paths are not copied into reports or logs.
- [ ] `trace_id` is safe to copy and does not encode secret, nonce, username, hostname, or full path data.

## High-Sensitivity Path Gate

- [ ] Codex sessions and raw rollout JSONL are skipped by default.
- [ ] Codex logs/goals/memories body fields are skipped by default.
- [ ] Codex auth, session, token, and account state are not read as plaintext.
- [ ] Claude project JSONL transcripts are skipped by default.
- [ ] Claude file-history and debug directories are skipped by default.
- [ ] Claude `primaryApiKey` value is never read, cached, logged, or reported.
- [ ] Claude account, OAuth, login state, and project private metadata in `~/.claude.json` are skipped or summarized without values.

## IPC Gate

- [ ] v1 sidecar uses stdio JSON-RPC and does not listen on any port.
- [ ] `ipc.handshake` is required before other methods.
- [ ] Nonce plaintext is not logged, reported, snapshotted, or returned after handshake.
- [ ] JSON-RPC top-level `error.code` is an integer; business code lives in `error.data.code`.
- [ ] Unsupported MVP 2 draft methods return `feature_not_in_mvp` without side effects.
- [ ] All IPC methods validate enum values, payload sizes, and path scope.

## Fixture And Test Gate

- [ ] `node tests/integration/validate-diagnostic-fixtures.mjs` passes.
- [ ] Codex only, Claude only, dual runtime, no runtime, malformed config, secret redaction, permission denied, and large scan scenarios are covered.
- [ ] Source fixtures may contain fake sentinel secrets, but expected report fixtures do not contain those sentinel values.
- [ ] Large scan performance evidence covers 100 plugins, 200 skills, 50 MCP servers, and 1000 config-related files.
- [ ] Timeout and canceled scans do not produce fake success reports.

## MVP 2 Write Gate

MVP 2 implementation work must not begin until these are designed, reviewed, and mapped to tests or explicit manual acceptance evidence:

- [ ] `config.diff` params/result schema.
- [ ] `config.apply` params/result schema.
- [ ] Snapshot manifest schema.
- [ ] Confirmation token model.
- [ ] Atomic write strategy per platform.
- [ ] Restore failure semantics.
- [ ] Snapshot retention and cleanup policy.
- [ ] Secret-free backup payload policy.
- [ ] File permission tests for macOS and Windows.
- [ ] Stable write failure codes: `diff_invalid`, `confirmation_required`, `source_changed`, `snapshot_failed`, `atomic_write_failed`, `revalidate_failed`, `restore_failed`, `path_denied`, `permission_denied`.
- [ ] Tests proving `config.apply` cannot run without a matching, unexpired `config.diff` confirmation token.
- [ ] Tests proving snapshot manifest and write failure reports contain no secret values or raw snapshot payload.
