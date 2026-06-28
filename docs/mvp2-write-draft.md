# MVP 2 写入设计冻结规格

本文档冻结 MVP 2 写入能力的 contract，不进入 MVP 1 实现范围。MVP 1 可以保留方法名，但所有写入、测试和备份方法必须返回 `feature_not_in_mvp`，不得产生副作用。

MVP 2 进入实现前，必须先冻结本文的 schema、确认模型和失败语义，并让 `docs/security-checklist.md` 的 MVP 2 Write Gate 全部映射到测试或人工验收证据。

机器可校验 contract：

- `schemas/config-diff.schema.json`
- `schemas/config-apply.schema.json`
- `schemas/snapshot-manifest.schema.json`

验收 fixtures：

- `fixtures/mvp2/diff/*.json`
- `fixtures/mvp2/apply/*.json`
- `fixtures/mvp2/snapshot/*.json`

校验入口：

```sh
node tests/integration/validate-mvp2-fixtures.mjs
```

## Non-Goals For MVP 1

MVP 1 不实现：

- `config.diff` 的真实 diff 计算。
- `config.apply`。
- snapshot 创建、读取、删除或 restore。
- MCP 连接测试。
- Hook dry-run。
- Plugin install、update、enable、disable、remove。
- Skill create/edit。

## Draft IPC Surface

MVP 2 实现前必须冻结以下 schema：

| Method | MVP 1 behavior | MVP 2 intent |
| --- | --- | --- |
| `config.diff` | `feature_not_in_mvp` | 生成只读 dry-run diff，不写文件。 |
| `config.apply` | `feature_not_in_mvp` | 用户确认后 snapshot、atomic write、re-validate。 |
| `backup.list` | `feature_not_in_mvp` | 列出脱敏 snapshot manifest。 |
| `backup.create` | `feature_not_in_mvp` | 写入前创建 snapshot。 |
| `backup.restore` | `feature_not_in_mvp` | 从 snapshot 恢复并报告 restore 状态。 |
| `mcp.test` | `feature_not_in_mvp` | 用户触发的连接测试，带 timeout 和脱敏输出。 |
| `plugin.enable` / `plugin.disable` | `feature_not_in_mvp` | 修改 plugin enabled state。 |
| `skill.create` | `feature_not_in_mvp` | 创建 Skill skeleton 和 frontmatter。 |

## Design Freeze Checklist

MVP 2 开发开始前必须冻结并通过 fixtures 校验：

- `config.diff` params/result schema：`schemas/config-diff.schema.json` 和 `fixtures/mvp2/diff/*.json`。
- `config.apply` result schema：`schemas/config-apply.schema.json` 和 `fixtures/mvp2/apply/*.json`。
- Snapshot manifest schema：`schemas/snapshot-manifest.schema.json` 和 `fixtures/mvp2/snapshot/*.json`。
- Confirmation token 模型：diff-scoped、expires_at-scoped、apply 必填。
- Restore 失败语义：`restore_failed` 必须显式返回，不得伪造成成功。
- Atomic write 策略：snapshot 成功后才能写入；atomic write 失败保留 restore path。
- Snapshot retention 和 cleanup 策略：默认 20 个或 30 天。
- Secret-free backup payload 策略：manifest 和报告不得包含 payload 内容或 secret 原文。
- macOS / Windows 文件权限测试策略：进入实现前补平台测试。

冻结要求：

- schema 示例必须能被机器校验。
- 所有失败路径必须返回稳定业务码和 `trace_id`。
- 所有用户可见 path、diff value、snapshot manifest 字段必须是脱敏或摘要。
- 写入实现前必须有 dry-run 测试和 redaction regression 测试。

## `config.diff` Draft Params

```json
{
  "runtime": "codex",
  "source_id": "codex-user-config",
  "path": "~/redacted/.codex/config.toml",
  "format": "toml",
  "intent": "mcp_server_upsert",
  "changes": [
    {
      "op": "set",
      "field": "mcp_servers.github.command",
      "value": {
        "kind": "command_summary",
        "command": "npx",
        "args_count": 2
      }
    }
  ],
  "client_nonce_hash": "7d9f1a0c4e2b"
}
```

Params rules:

- `path` must be a sidecar-resolved allowed target, not trusted solely from UI input.
- `value` must be typed; raw secret-bearing string values are not accepted for diff preview.
- `client_nonce_hash` is a correlation value only; nonce plaintext must not be logged or returned.

## `config.diff` Draft Result

Result 必须满足 `schemas/config-diff.schema.json`。

```json
{
  "trace_id": "trace-id",
  "target": {
    "runtime": "codex",
    "source_id": "codex-user-config",
    "path": "~/redacted/.codex/config.toml",
    "format": "toml"
  },
  "changes": [
    {
      "op": "set",
      "field": "mcp_servers.github.command",
      "before": {
        "kind": "missing"
      },
      "after": {
        "kind": "redacted_summary",
        "summary": "command:npx args:2"
      },
      "risk_codes": []
    }
  ],
  "requires_snapshot": true,
  "requires_confirmation": true,
  "would_execute_commands": false,
  "redaction_notice": "Diff values are summarized; secrets and command output are omitted."
}
```

Rules:

- Diff result must not include raw secret values.
- Diff preview must not execute MCP servers, hooks, plugins, credential helpers, or install scripts.
- `before` and `after` must use typed redacted summaries, not arbitrary strings.
- `would_execute_commands` must remain `false` for config edits; command execution belongs only to explicit MVP 2 test workflows.

## `config.apply` Draft Params

```json
{
  "diff_id": "diff-20260628-0001",
  "confirmation_token": "confirm-token-derived-from-diff",
  "expected_source_hash_12": "2f6a19d2c0be"
}
```

Apply rules:

- `confirmation_token` must be generated from the exact `config.diff` result and expire after a short window.
- `expected_source_hash_12` must match the target source immediately before write.
- A mismatched hash returns `source_changed` and does not write.
- Missing or invalid confirmation returns `confirmation_required` and does not write.

`config.apply` result 必须满足 `schemas/config-apply.schema.json`。成功 result 见 `fixtures/mvp2/apply/apply-success.json`；失败 result 至少覆盖 `source_changed`、`atomic_write_failed` 和 `restore_failed`。

`config.apply` 不接受 UI 自行构造的 target/path。sidecar 必须通过 `diff_id` 查找已冻结 diff，并在写入前重新校验 source hash。

## `backup.list` Draft Result

```json
{
  "trace_id": "trace-id",
  "snapshots": [
    {
      "snapshot_id": "snap-20260628-0001",
      "created_at": "2026-06-28T08:00:00Z",
      "reason": "config.apply",
      "item_count": 1,
      "restore_status": "not_restored"
    }
  ]
}
```

Rules:

- List result returns manifest summaries only.
- It must not return snapshot payload content.
- Full manifest details must still satisfy `schemas/snapshot-manifest.schema.json`.

## `backup.create` Draft Result

`backup.create` returns a full snapshot manifest satisfying `schemas/snapshot-manifest.schema.json`.

Rules:

- MVP 2 apply flow creates snapshot automatically; manual `backup.create` remains a protected expert action.
- Snapshot target paths must be sidecar-resolved allowlisted sources.
- Snapshot payload storage must use app private data dir.

## `backup.restore` Draft Result

`backup.restore` returns an apply-style result satisfying `schemas/config-apply.schema.json`, with `status` set to `restored` or `restore_failed`.

Rules:

- Restore preflight must verify target path remains allowlisted.
- Restore failure must keep `restore_available=true` when another retry may be possible.
- Restore failure must never be returned as `applied` or `restored`.

## Snapshot Manifest Draft

Manifest 必须满足 `schemas/snapshot-manifest.schema.json`。

```json
{
  "schema_version": "agentcafe.snapshot.v1",
  "snapshot_id": "snap-20260628-0001",
  "created_at": "2026-06-28T08:00:00Z",
  "trace_id": "trace-id",
  "reason": "config.apply",
  "items": [
    {
      "runtime": "codex",
      "source_id": "codex-user-config",
      "path": "~/redacted/.codex/config.toml",
      "format": "toml",
      "content_sha256_12": "2f6a19d2c0be",
      "byte_length": 842
    }
  ],
  "restore_status": "not_restored"
}
```

Manifest rules:

- Manifest stores redacted path, hash, length, timestamps, source ids, and restore state only.
- Snapshot payload storage must live under the app private data directory.
- Snapshot must not be written into project directories, Git worktrees, `.codex`, `.claude`, or cloud sync folders.
- Payload file permissions must be current-user read/write only.

Restore states:

- `not_restored`
- `restore_succeeded`
- `restore_failed`
- `restore_partially_succeeded`

Restore failure must never be reported as success.

## Apply Flow

```text
validate request
  -> compute redacted diff
  -> require explicit user confirmation
  -> create snapshot in private app data dir
  -> atomic write to target
  -> re-validate target
  -> return success or restore-ready failure
```

Failure semantics:

- Diff failure returns stable code and `trace_id`; no writes.
- Snapshot failure aborts before write.
- Atomic write failure returns stable code and keeps snapshot manifest.
- Re-validation failure returns stable code and offers restore path.
- Restore failure must return `restore_failed`; it must never be reported as success.

Stable write failure codes:

- `diff_invalid`
- `confirmation_required`
- `source_changed`
- `snapshot_failed`
- `atomic_write_failed`
- `revalidate_failed`
- `restore_failed`
- `path_denied`
- `permission_denied`

Failure fixture coverage:

- `fixtures/mvp2/diff/permission-denied.json`
- `fixtures/mvp2/apply/source-changed.json`
- `fixtures/mvp2/apply/atomic-write-failed.json`
- `fixtures/mvp2/apply/restore-failed.json`
- `fixtures/mvp2/snapshot/restore-failed-manifest.json`

## Confirmation Model

MVP 2 write confirmation must include:

- Runtime and source id.
- Redacted target path.
- Diff summary.
- Risk findings introduced or resolved.
- Snapshot location policy summary.
- Explicit user action from UI.

The sidecar must reject `config.apply` without a confirmation token generated from the matching `config.diff` result.

Confirmation token rules:

- Token is generated only after a schema-valid diff with `status=ready_for_confirmation`.
- Token binds to `diff_id`, `expected_source_hash_12`, target source id, target path, and change list digest.
- Token expires at `expires_at`.
- Token plaintext must not enter logs, reports, fixtures, or snapshot manifests.
- Applying with a missing, expired, mismatched, or reused token returns `confirmation_required` or `diff_invalid` and performs no writes.

## Test Evidence Required Before Implementation

Before MVP 2 implementation begins:

- `node tests/integration/validate-mvp2-fixtures.mjs` passes.
- All MVP 2 fixture JSON files pass `jq empty`.
- Security scan finds no sentinel secret or raw payload text in `fixtures/mvp2`, `schemas`, or docs examples.
- Existing `cargo test` still passes and MVP 1 sidecar still returns `feature_not_in_mvp` for MVP 2 draft methods.

## MVP 1.5 Optional Internal Stage

If the team wants an intermediate stage after MVP 1:

- Implement `config.diff` dry-run only.
- Do not implement `config.apply`.
- Do not create snapshots.
- Do not execute MCP, Hook, Plugin, credential helper, or install commands.
- Treat the feature as internal validation, not a public MVP 1 user workflow.
