# Agent Café IPC 契约

## 目标

IPC 契约用于连接 Windows WPF、macOS SwiftUI 和 Rust sidecar。v1 固定使用 `stdio JSON-RPC 2.0`，确保两个原生 UI 共用同一业务核心。loopback local service 仅作为 v2 或高级模式预留。

MVP 1 只开放只读体检能力。`agentcafe doctor --json` 是 MVP 1 的唯一验收入口，其输出必须能由同一组 Rust core 对象生成，也必须能映射到 UI 只读页面。

`DiagnosticReport` 的机器可读 contract 由 `schemas/diagnostic-report.schema.json` 固定。文档中的 JSON 示例必须能通过该 schema 校验，Rust model 变更必须同步更新 schema。

## 传输与握手

- v1 由 UI 启动 Rust sidecar 子进程，通过 stdin / stdout 传输 JSON-RPC 2.0。
- sidecar 不监听任何端口。
- UI 启动 sidecar 时生成一次性启动 nonce。
- UI 必须先调用 `ipc.handshake`，校验协议版本、sidecar 版本、UI capability 和 nonce。
- 握手完成前，sidecar 必须拒绝除 `ipc.handshake` 之外的所有方法。

## 通用 Envelope

请求：

```json
{
  "jsonrpc": "2.0",
  "id": "request-id",
  "method": "runtime.list",
  "params": {}
}
```

成功响应必须使用顶层 `result`：

```json
{
  "jsonrpc": "2.0",
  "id": "request-id",
  "result": {}
}
```

错误响应必须使用整数 `error.code`。业务错误码放入 `error.data.code`：

```json
{
  "jsonrpc": "2.0",
  "id": "request-id",
  "error": {
    "code": -32001,
    "message": "Agent Cafe business error.",
    "data": {
      "code": "config_invalid",
      "retryable": false,
      "trace_id": "trace-id",
      "stage": "config.validate"
    }
  }
}
```

错误响应不得包含 secret 明文、prompt 正文、完整 transcript、tool payload、shell output、nonce 明文或私有协议帧。

## 错误码策略

| JSON-RPC code | 含义 |
| ---: | --- |
| `-32700` | Parse error。 |
| `-32600` | Invalid request。 |
| `-32601` | Method not found。 |
| `-32602` | Invalid params。 |
| `-32603` | Internal error。 |
| `-32001` | Agent Cafe business error，细分业务码放入 `error.data.code`。 |
| `-32002` | Timeout。 |
| `-32003` | Canceled。 |
| `-32004` | Handshake required / failed。 |

MVP 1 稳定业务码：

| 业务码 | 使用场景 |
| --- | --- |
| `handshake_failed` | 协议版本、capability、nonce 或 sidecar 版本校验失败。 |
| `runtime_not_found` | 指定 runtime 不存在或不可执行。 |
| `config_invalid` | TOML / JSON / YAML 解析或 schema 校验失败。 |
| `path_denied` | 请求路径不在允许扫描范围。 |
| `permission_denied` | 文件系统权限不足。 |
| `scan_canceled` | 用户取消扫描。 |
| `scan_timeout` | 扫描超过 timeout。 |
| `source_untrusted` | 项目配置来源未被 runtime 信任，只能做存在性摘要。 |
| `redaction_required` | 命中敏感字段，只返回脱敏摘要。 |
| `feature_not_in_mvp` | 调用了 MVP 2 draft 方法。 |

## 枚举

| 枚举 | 取值 |
| --- | --- |
| `runtime` | `codex`, `claude` |
| `scope` | `managed`, `system`, `user`, `profile`, `project`, `local`, `cli_override`, `plugin`, `unknown` |
| `status` | `available`, `missing`, `disabled`, `invalid`, `blocked`, `unknown` |
| `severity` | `info`, `low`, `medium`, `high`, `critical` |
| `format` | `toml`, `json`, `yaml`, `markdown`, `directory`, `command`, `unknown` |
| `transport` | `stdio`, `http`, `sse`, `websocket`, `unknown` |
| `validation_status` | `valid`, `invalid`, `partial`, `skipped`, `unknown` |
| `install_source` | `path`, `npm`, `brew`, `winget`, `app_bundle`, `manual`, `managed`, `unknown` |
| `trust_status` | `trusted`, `untrusted`, `managed`, `not_applicable`, `unknown` |
| `mvp_stage` | `mvp1_read_only`, `mvp2_write_draft`, `mvp3_ecosystem_draft`, `v2_draft` |

枚举 fallback 规则：

- 读取 runtime 配置时遇到未知值，输出对象字段使用 `unknown`，并在同一对象 `diagnostics` 或 `risk_findings` 中增加稳定业务码。
- IPC request params 中的枚举若不是表内值，返回 JSON-RPC `-32602`。
- `runtime` 不允许 fallback；未知 runtime 必须返回 `-32602`。

## 类型约束

通用约束：

- 所有 top-level collection 字段必须返回数组，不得省略；无数据时返回 `[]`。
- 所有时间戳使用 RFC 3339 UTC，例如 `2026-06-28T08:00:00Z`。
- 所有路径字段必须先脱敏；完整路径只允许留在 sidecar 内存中用于扫描，不进入 IPC response。
- `id` 字段必须在单次报告内稳定唯一，格式推荐 `<runtime>-<scope>-<kind>-<index-or-name>`。
- `trace_id` 必须存在，长度 8-128，允许 `[A-Za-z0-9._:-]`。
- `message` 和 `recommended_action` 最大 500 字符；不得包含原始 secret、prompt、transcript、tool payload 或 shell output。
- 列表默认最多返回 1000 项；超过时 `summary.truncated=true`，并在 `risk_findings` 增加 `scan_truncated` info finding。

必填与可空：

| 对象 | 必填字段 | 可为 null 字段 |
| --- | --- | --- |
| `RuntimeProfile` | `runtime`, `display_name`, `executable_path`, `version`, `install_source`, `path_status`, `status`, `detected_at`, `diagnostics` | `executable_path` |
| `ConfigSource` | `id`, `runtime`, `scope`, `path`, `priority`, `format`, `read_policy`, `display_policy`, `write_policy`, `validation_status`, `trust_status`, `mvp_stage`, `source_reference` | `source_reference` |
| `PluginItem` | `id`, `runtime`, `name`, `version`, `scope`, `source`, `path`, `enabled`, `validation_status`, `capabilities`, `risk_count` | `path` |
| `SkillItem` | `id`, `runtime`, `name`, `description`, `scope`, `path`, `validation_status`, `referenced_resource_count`, `risk_count` | `path` |
| `McpServerItem` | `id`, `runtime`, `scope`, `transport`, `command_or_url_summary`, `enabled`, `connection_status`, `tool_count`, `resource_count`, `template_count`, `validation_status`, `risk_count` | `tool_count`, `resource_count`, `template_count` |
| `HookItem` | `id`, `runtime`, `scope`, `event`, `matcher`, `handler_type`, `command_summary`, `enabled`, `trust_status`, `validation_status`, `risk_count` | `matcher`, `command_summary` |
| `RiskFinding` | `code`, `severity`, `source`, `runtime`, `path`, `message`, `redacted_evidence`, `recommended_action` | `runtime`, `path`, `redacted_evidence` |
| `ConflictFinding` | `runtime`, `key`, `winning_source_id`, `shadowed_source_ids`, `explanation` | none |

`summary.truncated` 是可选字段；未返回时视为 `false`。MVP 1 若任一列表超过 1000 项、任一阶段 timeout 后保留部分结果，或 scan 被实现层截断，必须返回 `summary.truncated=true`。

## 统一数据对象

### RuntimeProfile

```json
{
  "runtime": "codex",
  "display_name": "Codex",
  "executable_path": "/usr/local/bin/codex",
  "version": "0.138.0",
  "install_source": "npm",
  "path_status": "available",
  "status": "available",
  "detected_at": "2026-06-28T08:00:00Z",
  "diagnostics": []
}
```

### ConfigSource

```json
{
  "id": "codex-user-config",
  "runtime": "codex",
  "scope": "user",
  "path": "~/redacted/.codex/config.toml",
  "priority": 40,
  "format": "toml",
  "read_policy": "read_whitelisted_fields",
  "display_policy": "redacted_summary",
  "write_policy": "mvp2_draft_only",
  "validation_status": "valid",
  "trust_status": "not_applicable",
  "mvp_stage": "mvp1_read_only",
  "source_reference": "https://developers.openai.com/codex/config-basic"
}
```

### PluginItem

```json
{
  "id": "gmail@openai-curated",
  "runtime": "codex",
  "name": "Gmail",
  "version": "unknown",
  "scope": "user",
  "source": "marketplace",
  "path": "~/redacted/.codex/plugins/gmail",
  "enabled": false,
  "validation_status": "partial",
  "capabilities": ["skills", "mcp_servers"],
  "risk_count": 1
}
```

### SkillItem

```json
{
  "id": "pdf",
  "runtime": "claude",
  "name": "pdf",
  "description": "Read and process PDF files.",
  "scope": "plugin",
  "path": "~/redacted/.claude/plugins/pdf/skills/pdf/SKILL.md",
  "validation_status": "valid",
  "referenced_resource_count": 2,
  "risk_count": 0
}
```

### McpServerItem

```json
{
  "id": "github",
  "runtime": "claude",
  "scope": "project",
  "transport": "http",
  "command_or_url_summary": "https://mcp.example.com/**",
  "enabled": true,
  "connection_status": "not_tested",
  "tool_count": null,
  "resource_count": null,
  "template_count": null,
  "validation_status": "valid",
  "risk_count": 0
}
```

### HookItem

```json
{
  "id": "codex-user-pretooluse-0",
  "runtime": "codex",
  "scope": "user",
  "event": "PreToolUse",
  "matcher": "^Bash$",
  "handler_type": "command",
  "command_summary": "/usr/bin/python3 .../pre_tool_use_policy.py",
  "enabled": true,
  "trust_status": "trusted",
  "validation_status": "valid",
  "risk_count": 1
}
```

### RiskFinding

```json
{
  "code": "secret_like_value",
  "severity": "high",
  "source": "config",
  "runtime": "claude",
  "path": "~/redacted/.claude/settings.json",
  "message": "A secret-like field is present and will not be displayed.",
  "redacted_evidence": {
    "field": "env.ANTHROPIC_API_KEY",
    "length": 32,
    "sha256_12": "3b7d9f1a0c4e",
    "source_summary": "user settings"
  },
  "recommended_action": "Move the value to the runtime's supported secret mechanism and keep it out of shared project files."
}
```

### ConflictFinding

```json
{
  "runtime": "codex",
  "key": "model",
  "winning_source_id": "codex-project-config",
  "shadowed_source_ids": ["codex-user-config"],
  "explanation": "Project config has higher precedence than user config when the project is trusted."
}
```

### DiagnosticReport

```json
{
  "schema_version": "agentcafe.diagnostic.v1",
  "trace_id": "trace-20260628-0001",
  "generated_at": "2026-06-28T08:00:00Z",
  "runtimes": [],
  "config_sources": [],
  "plugins": [],
  "skills": [],
  "mcp_servers": [],
  "hooks": [],
  "conflicts": [],
  "risk_findings": [],
  "summary": {
    "runtime_count": 0,
    "config_source_count": 0,
    "plugin_count": 0,
    "skill_count": 0,
    "mcp_server_count": 0,
    "hook_count": 0,
    "risk_count_by_severity": {
      "info": 0,
      "low": 0,
      "medium": 0,
      "high": 0,
      "critical": 0
    },
    "overall_status": "missing",
    "truncated": false
  },
  "redaction_notice": "Agent Cafe returns field names, counts, lengths, hashes, and redacted paths only. Secrets, prompts, transcripts, tool payloads, shell output, and nonce values are omitted."
}
```

## MVP 1 方法

### `ipc.handshake`

Params:

```json
{
  "protocol_version": "1.0",
  "ui_name": "AgentCafe.Windows",
  "ui_version": "0.1.0",
  "ui_platform": "windows",
  "ui_capabilities": ["runtime.list", "config.scan", "risk.scan"],
  "nonce": "<one-time-startup-nonce>"
}
```

Result:

```json
{
  "protocol_version": "1.0",
  "sidecar_version": "0.1.0",
  "accepted_capabilities": ["runtime.list", "config.scan", "risk.scan"],
  "trace_id": "trace-id"
}
```

Failure example:

```json
{
  "jsonrpc": "2.0",
  "id": "request-id",
  "error": {
    "code": -32004,
    "message": "Handshake failed.",
    "data": {
      "code": "handshake_failed",
      "retryable": false,
      "trace_id": "trace-id",
      "stage": "ipc.handshake"
    }
  }
}
```

### `runtime.list`

Params:

```json
{
  "include_diagnostics": true
}
```

Result:

```json
{
  "runtimes": [
    {
      "runtime": "codex",
      "display_name": "Codex",
      "executable_path": "/usr/local/bin/codex",
      "version": "0.138.0",
      "install_source": "npm",
      "path_status": "available",
      "status": "available",
      "detected_at": "2026-06-28T08:00:00Z",
      "diagnostics": []
    }
  ],
  "trace_id": "trace-id"
}
```

Error example: `runtime_not_found` only applies to `runtime.probe`; `runtime.list` returns missing runtime entries instead of failing when neither runtime exists.

### `runtime.probe`

Params:

```json
{
  "runtime": "claude",
  "candidate_paths": []
}
```

Result:

```json
{
  "runtime": {
    "runtime": "claude",
    "display_name": "Claude Code",
    "executable_path": null,
    "version": "unknown",
    "install_source": "unknown",
    "path_status": "missing",
    "status": "missing",
    "detected_at": "2026-06-28T08:00:00Z",
    "diagnostics": [
      {
        "code": "runtime_not_found",
        "message": "claude was not found on PATH or known install locations."
      }
    ]
  },
  "trace_id": "trace-id"
}
```

### `config.scan`

Params:

```json
{
  "runtimes": ["codex", "claude"],
  "workspace_root": "/Users/example/project",
  "include_experimental_detect_only": true,
  "include_untrusted_project_sources": true
}
```

Result:

```json
{
  "config_sources": [],
  "conflicts": [
    {
      "runtime": "codex",
      "key": "model",
      "winning_source_id": "codex-project-config",
      "shadowed_source_ids": ["codex-user-config"],
      "explanation": "Project config has higher precedence than user config when the project is trusted."
    }
  ],
  "trace_id": "trace-id"
}
```

`config.scan.result.conflicts` 必须原样或经去重后进入 `DiagnosticReport.conflicts`。如果 conflict key 本身疑似 secret 字段，只能返回字段名和 source id，不得返回值。

Error example:

```json
{
  "jsonrpc": "2.0",
  "id": "request-id",
  "error": {
    "code": -32001,
    "message": "Configuration source is invalid.",
    "data": {
      "code": "config_invalid",
      "retryable": false,
      "trace_id": "trace-id",
      "stage": "config.scan"
    }
  }
}
```

### `config.validate`

Params:

```json
{
  "source_id": "codex-user-config",
  "runtime": "codex",
  "path": "~/redacted/.codex/config.toml",
  "format": "toml"
}
```

Result:

```json
{
  "validation_status": "valid",
  "errors": [],
  "warnings": [],
  "trace_id": "trace-id"
}
```

### `plugin.list`

Params:

```json
{
  "runtimes": ["codex", "claude"],
  "include_disabled": true
}
```

Result:

```json
{
  "plugins": [],
  "trace_id": "trace-id"
}
```

### `skill.list`

Params:

```json
{
  "runtimes": ["codex", "claude"],
  "include_plugin_skills": true
}
```

Result:

```json
{
  "skills": [],
  "trace_id": "trace-id"
}
```

### `mcp.list`

Params:

```json
{
  "runtimes": ["codex", "claude"],
  "include_plugin_servers": true,
  "test_connections": false
}
```

Result:

```json
{
  "mcp_servers": [],
  "trace_id": "trace-id"
}
```

`test_connections` must remain `false` in MVP 1. Connection tests are user-triggered MVP 2 draft behavior. In `agentcafe.diagnostic.v1`, `McpServerItem.connection_status` is limited to `not_tested`, `invalid`, or `unknown`; `tool_count`, `resource_count`, and `template_count` remain `null` because MVP 1 does not perform MCP connection tests.

### `risk.scan`

Params:

```json
{
  "runtimes": ["codex", "claude"],
  "source_ids": [],
  "include_secret_like_scan": true
}
```

Result:

```json
{
  "risk_findings": [],
  "trace_id": "trace-id"
}
```

## `agentcafe doctor --json`

MVP 1 golden sample:

```json
{
  "schema_version": "agentcafe.diagnostic.v1",
  "trace_id": "trace-20260628-0001",
  "generated_at": "2026-06-28T08:00:00Z",
  "runtimes": [
    {
      "runtime": "codex",
      "display_name": "Codex",
      "executable_path": "/usr/local/bin/codex",
      "version": "0.138.0",
      "install_source": "npm",
      "path_status": "available",
      "status": "available",
      "detected_at": "2026-06-28T08:00:00Z",
      "diagnostics": []
    },
    {
      "runtime": "claude",
      "display_name": "Claude Code",
      "executable_path": null,
      "version": "unknown",
      "install_source": "unknown",
      "path_status": "missing",
      "status": "missing",
      "detected_at": "2026-06-28T08:00:00Z",
      "diagnostics": [
        {
          "code": "runtime_not_found",
          "message": "claude was not found on PATH or known install locations."
        }
      ]
    }
  ],
  "config_sources": [
    {
      "id": "codex-user-config",
      "runtime": "codex",
      "scope": "user",
      "path": "~/redacted/.codex/config.toml",
      "priority": 40,
      "format": "toml",
      "read_policy": "read_whitelisted_fields",
      "display_policy": "redacted_summary",
      "write_policy": "mvp2_draft_only",
      "validation_status": "valid",
      "trust_status": "not_applicable",
      "mvp_stage": "mvp1_read_only",
      "source_reference": "https://developers.openai.com/codex/config-basic"
    }
  ],
  "plugins": [],
  "skills": [],
  "mcp_servers": [],
  "hooks": [],
  "conflicts": [
    {
      "runtime": "codex",
      "key": "model",
      "winning_source_id": "codex-user-config",
      "shadowed_source_ids": [],
      "explanation": "Only one source defines this key in the sample report."
    }
  ],
  "risk_findings": [
    {
      "code": "secret_like_value",
      "severity": "high",
      "source": "config",
      "runtime": "codex",
      "path": "~/redacted/.codex/config.toml",
      "message": "A secret-like field is present and will not be displayed.",
      "redacted_evidence": {
        "field": "mcp_servers.github.env.GITHUB_TOKEN",
        "length": 40,
        "sha256_12": "3b7d9f1a0c4e",
        "source_summary": "user config"
      },
      "recommended_action": "Move the value to the runtime's supported secret mechanism and keep it out of shared project files."
    }
  ],
  "summary": {
    "runtime_count": 1,
    "config_source_count": 1,
    "plugin_count": 0,
    "skill_count": 0,
    "mcp_server_count": 0,
    "hook_count": 0,
    "risk_count_by_severity": {
      "info": 0,
      "low": 0,
      "medium": 0,
      "high": 1,
      "critical": 0
    },
    "overall_status": "available",
    "truncated": false
  },
  "redaction_notice": "Agent Cafe returns field names, counts, lengths, hashes, and redacted paths only. Secrets, prompts, transcripts, tool payloads, shell output, and nonce values are omitted."
}
```

### Doctor 聚合流程

`agentcafe doctor --json` 必须按以下顺序执行，并将部分失败降级为诊断项，而不是整体崩溃：

1. 生成 `trace_id` 和 `generated_at`。
2. 执行 `runtime.list`，生成 Codex / Claude 两个 runtime 条目；缺失 runtime 返回 `status=missing`。
3. 对可用 runtime 执行 `config.scan`；解析失败生成 `ConfigSource.validation_status=invalid` 和 `RiskFinding(code=config_invalid)`。
4. 执行 `plugin.list`、`skill.list`、`mcp.list`、hook metadata scan；每个子扫描独立 timeout。
5. 执行 `risk.scan`；只使用已读取的白名单字段和 metadata。
6. 聚合 `summary` 计数；若任何子扫描 timeout / permission denied / parse error，`summary.overall_status` 至少为 `blocked` 或 `invalid`。
7. 输出 `DiagnosticReport`，并在 CLI 层用 `schemas/diagnostic-report.schema.json` 校验；校验失败返回非 0 exit code，且 stderr 只包含稳定错误码和 trace id。

分段 timeout 默认值：

| 阶段 | 默认 timeout |
| --- | ---: |
| runtime detection | 3s |
| config scan | 10s |
| plugin / skill inventory | 10s |
| MCP metadata inventory | 5s |
| hook metadata inventory | 5s |
| risk scan | 10s |
| full doctor | 30s |

部分失败规则：

- permission denied：保留脱敏 path、source、`permission_denied` finding，继续扫描其他来源。
- malformed config：该 source 标记 `invalid`，继续扫描其他来源。
- runtime missing：不扫描该 runtime 的 config/plugin/skill/MCP/hooks。
- timeout：保留已完成结果，`summary.truncated=true`，增加 `scan_timeout` finding。
- canceled：返回 `scan_canceled`，不得输出伪成功报告。

## MVP 2 Draft 方法

以下方法保留名称，但在 MVP 1 中必须返回 `feature_not_in_mvp`，不得执行写入或外部连接测试：

- `config.diff`
- `config.apply`
- `plugin.inspect`
- `plugin.enable`
- `plugin.disable`
- `skill.validate`
- `skill.create`
- `mcp.test`
- `backup.list`
- `backup.create`
- `backup.restore`

MVP 2 实现前必须另行冻结这些方法的 params/result schema、写入确认模型、snapshot manifest 和 restore 失败语义。当前冻结入口见 `docs/mvp2-write-draft.md`，机器可校验 schema 为：

- `schemas/config-diff.schema.json`
- `schemas/config-apply.schema.json`
- `schemas/snapshot-manifest.schema.json`

MVP 2 contract fixtures 位于：

- `fixtures/mvp2/diff/*.json`
- `fixtures/mvp2/apply/*.json`
- `fixtures/mvp2/snapshot/*.json`

### MVP 2 Wire Contract Preview

MVP 2 写入实现启用时，以下方法使用标准 JSON-RPC `result` envelope，并继续使用整数 JSON-RPC `error.code` 与业务码 `error.data.code`：

| Method | Params | Result | Stable failure codes |
| --- | --- | --- | --- |
| `config.diff` | target source, intent, typed changes, client nonce hash | `agentcafe.config_diff.v1` | `diff_invalid`, `path_denied`, `permission_denied` |
| `config.apply` | `diff_id`, `confirmation_token`, `expected_source_hash_12` | `agentcafe.config_apply.v1` | `confirmation_required`, `source_changed`, `snapshot_failed`, `atomic_write_failed`, `revalidate_failed`, `path_denied`, `permission_denied` |
| `backup.list` | optional filters | snapshot manifest summaries | `path_denied`, `permission_denied` |
| `backup.create` | allowlisted source ids and reason | `agentcafe.snapshot.v1` | `snapshot_failed`, `path_denied`, `permission_denied` |
| `backup.restore` | `snapshot_id`, source ids, confirmation token | `agentcafe.config_apply.v1` | `restore_failed`, `source_changed`, `path_denied`, `permission_denied` |

MVP 2 写入 contract invariants:

- `config.apply` must reject missing, expired, mismatched, or reused confirmation tokens before any write.
- Snapshot creation must complete before any write.
- Atomic write failure must report `atomic_write_failed` and keep restore evidence when a snapshot exists.
- Restore failure must report `restore_failed`; it must never be represented as success.
- Result payloads must not contain raw secrets, prompt text, transcript text, tool payloads, shell output, nonce plaintext, or snapshot payload content.
- MVP 2 config write methods must not start MCP servers, execute Hook commands, execute Plugin commands, or call credential helpers.

## Timeout 与取消

- UI 调用必须设置 timeout。
- sidecar 内部调用外部 CLI、MCP server 或文件扫描也必须设置分段 timeout。
- 长耗时扫描必须支持取消；取消后返回 `scan_canceled`，不得伪造成成功。
- MVP 1 不允许通过 IPC 自动执行 MCP server、plugin command 或 hook command。

## 脱敏要求

IPC response 默认只能返回：

- 字段名。
- 状态。
- 错误码。
- hash。
- 长度。
- 脱敏路径。
- 计数。

不得返回：

- API key、token、cookie。
- prompt 正文。
- 完整 transcript。
- tool payload。
- shell output 全文。
- nonce 明文。
- Codex / Claude 私有协议帧。
