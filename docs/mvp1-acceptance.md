# MVP 1 验收标准

本文档把 MVP 1 的“只读体检”验收收敛成可执行清单。MVP 1 完成的含义是：`agentcafe doctor --json` 或等价 sidecar IPC 输出稳定、可校验、已脱敏，并能被原生 UI 直接映射。

## Completion Gate

MVP 1 release candidate 必须同时满足：

- `agentcafe doctor --json` 输出通过 `schemas/diagnostic-report.schema.json`。
- `fixtures/diagnostic/reports/*.json` 对应场景均有自动化测试或人工验收证据。
- UI 只消费 `DiagnosticReport`，不自行解析 `.codex`、`.claude`、plugin manifest、MCP config 或 hook config。
- 没有配置写入、snapshot、restore、MCP 连接测试、Hook dry-run、Plugin command 执行。
- 报告、日志、IPC response、验收截图均不包含 secret、prompt、transcript、tool payload、shell output、nonce 明文或完整私有路径。

## CLI Contract

`agentcafe doctor --json` 必须返回完整 `DiagnosticReport`：

- 必填字段：`schema_version`、`trace_id`、`generated_at`、`runtimes`、`config_sources`、`plugins`、`skills`、`mcp_servers`、`hooks`、`conflicts`、`risk_findings`、`summary`、`redaction_notice`。
- `schema_version` 固定为 `agentcafe.diagnostic.v1`。
- `trace_id` 必须在 CLI stderr、IPC error data、UI 错误状态中可关联。
- `summary` 计数必须与同一报告中的数组内容一致。
- `summary.truncated=true` 只用于 timeout 或明确截断；不得伪装成完整扫描。

CLI 失败规则：

- schema 校验失败：非 0 exit code，stderr 只包含稳定错误码和 `trace_id`。
- canceled：返回 `scan_canceled`，不得输出伪成功报告。
- sidecar crash：UI 显示 degraded/retry；不得把上一份报告标记为最新。

## Risk Levels

风险等级用于排序、视觉权重和用户文案，不等价于自动修复权限。

| Severity | 使用条件 | UI 文案方向 |
| --- | --- | --- |
| `critical` | 配置无法解析、private key 明文风险、会阻断可信诊断的结构性问题。 | 需要先处理，否则诊断结果不完整或不可信。 |
| `high` | API key、token、cookie、Authorization header、provider key、password-like 字段。 | 敏感信息不能展示；建议迁移到 runtime 支持的 secret 机制。 |
| `medium` | permission denied、危险 hook metadata、untrusted project source、会影响部分扫描的 timeout。 | 解释影响范围，提供 retry/reveal/人工检查入口。 |
| `low` | 非阻断配置冲突、未知字段计数、兼容性提示。 | 帮助用户理解来源和优先级。 |
| `info` | 大扫描摘要、未安装 runtime、未测试连接等中性状态。 | 说明状态，不渲染成失败。 |

## Fixture Coverage

MVP 1 必须覆盖：

- Codex only。
- Claude only。
- 双 runtime。
- 无 runtime。
- malformed TOML / JSON / YAML。
- secret redaction。
- 权限不足。
- sidecar missing。
- sidecar version mismatch。
- scan timeout / canceled。
- large scan：100 plugins、200 skills、50 MCP servers、1k 配置相关文件。

当前报告样本位于 `fixtures/diagnostic/reports/`。`golden-sample.json` 是 `docs/ipc-contract.md` 中 `agentcafe doctor --json` golden sample 的文件版本；其中 `large-scan.json` 是紧凑 smoke fixture，性能验收必须生成完整规模样本并记录耗时、取消和 UI 滚动表现。

## UI Acceptance

UI 验收必须证明：

- 导航顺序为：总览、Codex助手、Claude助手、MCP、Plugins、Skills、Hooks / Risks、诊断详情。
- `not_tested` 的 MCP server 不显示为失败，也不提供 Test action。
- Plugins 不提供 install、update、enable、disable、remove；如入口存在，必须禁用并标明 MVP 2 draft。
- 诊断详情只显示已脱敏 JSON，支持复制 `trace_id`，不支持导出完整报告。
- Refresh 保留上一份报告但显示 loading overlay；Cancel 后显示 canceled，不把旧报告伪装为新结果。
- 所有错误状态显示 `trace_id` 和稳定错误码。

## Security Acceptance

安全验收必须证明：

- secret-like 命中只输出 `redacted_evidence`：字段名、长度、`sha256_12`、来源摘要。
- 默认不读取 transcript、debug logs、session JSONL、file-history、prompt 正文、tool payload 或 shell output。
- `~/.claude/config.json.primaryApiKey` 只允许存在性检测，不读明文、不缓存、不进报告。
- v1 sidecar 使用 stdio JSON-RPC，不监听端口。
- MVP 2 draft 方法在 MVP 1 中返回 `feature_not_in_mvp`，不产生副作用。

## Verification Commands

在代码工程初始化前，至少运行：

```sh
node tests/integration/validate-diagnostic-fixtures.mjs
```

Rust CLI 实现后，MVP 1 release candidate 必须补充：

```sh
agentcafe doctor --json > /tmp/agentcafe-doctor.json
```

并用同一 schema validator 校验真实输出。
