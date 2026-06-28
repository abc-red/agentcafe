# Agent Cafe MVP 1 UI 产品规格

## 目标

MVP 1 UI 是 `agentcafe doctor --json` 的原生可视化层。它帮助用户理解本机 Codex / Claude Code 配置生态的健康状态、来源、风险和下一步处理建议。

MVP 1 不提供写入、启用、禁用、安装、删除、restore、MCP 连接测试或 Hook dry-run。所有页面只能展示 Rust sidecar 返回的 `DiagnosticReport`。

## 信息架构

MVP 1 页面：

| 页面 | 数据来源 | 主要内容 |
| --- | --- | --- |
| 总览 | `summary`, `runtimes`, `risk_findings`, `conflicts` | runtime 状态、风险计数、配置来源计数、冲突计数、最近一次扫描时间。 |
| Codex助手 | filtered `runtime=codex` | Codex runtime、config sources、plugins、skills、MCP、hooks、risks。 |
| Claude助手 | filtered `runtime=claude` | Claude runtime、settings sources、plugins、skills、MCP、hooks、risks。 |
| MCP | `mcp_servers` | server name、runtime、scope、transport、enabled、connection_status、risk_count。 |
| Plugins | `plugins` | plugin name、runtime、source、enabled、capabilities、validation_status、risk_count。 |
| Skills | `skills` | skill name、runtime、scope、description、validation_status、referenced_resource_count。 |
| Hooks / Risks | `hooks`, `risk_findings` | hook metadata、severity、source、redacted evidence、recommended action。 |
| 诊断详情 | full `DiagnosticReport` | schema_version、trace_id、generated_at、redaction_notice、错误码和原始脱敏 JSON。 |

导航顺序必须保持一致：总览、Codex助手、Claude助手、MCP、Plugins、Skills、Hooks / Risks、诊断详情。

## 交互规则

- Refresh：用户点击刷新时，UI 重新启动 doctor scan 或重新调用 sidecar 聚合入口；刷新期间保留上一份报告，显示 loading overlay。
- Cancel：用户取消 scan 时，UI 显示 canceled 状态；不得把上一份报告伪装成最新结果。
- Retry：仅对 `permission_denied`、`scan_timeout`、`sidecar_missing`、`sidecar_version_mismatch`、`sidecar crash` 显示 retry 操作。
- Reveal：只对用户明确点击的单个 source path 执行平台 reveal；完整路径不得写入日志、报告或剪贴板。
- Copy trace id：只复制 `trace_id`，不复制完整报告。
- Export：MVP 1 不提供导出报告。诊断详情只显示已脱敏 JSON。

## 页面状态

所有页面必须支持以下状态：

| 状态 | 触发条件 | UI 行为 |
| --- | --- | --- |
| loading | sidecar 启动、handshake 或 scan 进行中 | 显示进度状态和取消入口；不得冻结窗口。 |
| empty | 当前列表为空且没有错误 | 显示短空状态，例如 "No MCP servers found." |
| no runtime | `runtimes` 中对应 runtime `status=missing` | 显示缺失状态、检测路径摘要和安装建议入口占位。 |
| single runtime | 只有 Codex 或 Claude 可用 | 总览显示一个可用 runtime 和一个 missing runtime。 |
| dual runtime | Codex 和 Claude 都可用 | 总览显示双 runtime 健康摘要。 |
| parse error | `config_invalid` | 显示 source、path、error code、trace_id；不显示原始文件片段。 |
| permission denied | `permission_denied` | 显示脱敏路径、scope、trace_id 和重试入口。 |
| untrusted source | `source_untrusted` | 显示存在性和说明；不展示字段详情。 |
| timeout | `scan_timeout` or JSON-RPC `-32002` | 标记本次诊断 incomplete，保留已返回的部分结果。 |
| canceled | `scan_canceled` or JSON-RPC `-32003` | 显示 canceled 状态，不当作失败。 |
| sidecar missing | UI 找不到 sidecar executable | 显示 `sidecar_missing`，允许重新定位或重试。 |
| sidecar version mismatch | handshake version fail | 显示 `sidecar_version_mismatch` 和 UI/sidecar version。 |
| sidecar crash | scan 中子进程退出 | 显示 degraded 状态和 restart sidecar 操作。 |

## 总览

总览首屏内容：

- Runtime health：Codex、Claude Code 两个状态行。
- Counts：config sources、plugins、skills、MCP servers、hooks。
- Conflicts：配置冲突计数和最高优先级冲突摘要。
- Risk summary：按 `critical/high/medium/low/info` 计数。
- Last scan：`generated_at`、`trace_id`。
- Redaction notice：短文本说明所有敏感值已脱敏。

总览不得展示配置正文、完整 command output、prompt、transcript、tool payload 或 secret 原值。

## Runtime 页面

`Codex助手` 与 `Claude助手` 使用同一布局：

- Runtime card：display_name、version、executable_path 脱敏摘要、install_source、status。
- Config sources table：scope、path、priority、format、validation_status、trust_status。
- Ecosystem tabs：Plugins、Skills、MCP、Hooks。
- Risks list：当前 runtime 的 risk findings。

`path` 默认显示脱敏路径。用户选择 reveal 时，只允许调用平台 reveal 对话，不把完整路径复制到日志或报告。

## MCP 页面

表格列：

- Name
- Runtime
- Scope
- Transport
- Command / URL summary
- Enabled
- Connection status
- Validation
- Risk count

MVP 1 中 `connection_status` 只能是 `not_tested`、`invalid`、`unknown`。不得提供自动连接测试按钮。MVP 2 再增加用户触发的 Test action。

排序与过滤：

- 默认排序：runtime、scope、name。
- 支持过滤：runtime、scope、transport、validation_status、risk_count > 0。
- `not_tested` 不渲染为失败；只显示“未测试”状态。

## Plugins 页面

表格列：

- Name
- Runtime
- Version
- Source
- Scope
- Enabled
- Capabilities
- Validation
- Risk count

MVP 1 不提供 install、update、enable、disable、remove 操作。若设计稿需要操作入口，必须显示 disabled 状态并标注为 MVP 2 draft。

排序与过滤：

- 默认排序：runtime、source、name。
- 支持过滤：runtime、enabled、capability、validation_status、risk_count > 0。
- capability chip 只来自 `PluginItem.capabilities`，UI 不自行推断。

## Skills 页面

表格列：

- Name
- Runtime
- Scope
- Description
- Validation
- Referenced resources
- Risk count

Skill 详情只能展示 frontmatter 摘要和引用资源路径摘要，不展示引用资源正文。

排序与过滤：

- 默认排序：runtime、scope、name。
- 支持过滤：runtime、scope、validation_status、risk_count > 0。
- description 超过两行时折叠；展开后仍不得加载引用资源正文。

## Hooks / Risks 页面

Hooks 表格列：

- Event
- Runtime
- Scope
- Matcher
- Handler type
- Command / endpoint summary
- Trust
- Validation
- Risk count

Risks 列表字段：

- Severity
- Code
- Runtime
- Source
- Path
- Message
- Redacted evidence
- Recommended action

Severity 视觉排序：`critical`、`high`、`medium`、`low`、`info`。同级按 runtime、source、path 排序。`redacted_evidence` 必须使用字段名、长度、hash 和来源摘要，不得展示 secret 原值。

错误详情层级：

- 列表行只显示 severity、code、message、source。
- 展开行显示 runtime、path、redacted evidence、recommended action、trace id。
- `critical` 和 `high` 默认展开第一条，其余默认折叠。

## 诊断详情

诊断详情用于支持工程排障：

- 显示 `schema_version`、`trace_id`、`generated_at`。
- 显示 redaction notice。
- 显示 pretty-printed `DiagnosticReport`，但必须来自已脱敏 JSON。
- 支持复制 trace id。
- MVP 1 不支持导出完整报告到用户选择路径；如需要保存报告，必须另行做安全审查。

## Doctor JSON 映射

UI 必须从 `DiagnosticReport` 直接映射：

| JSON 字段 | UI 页面 |
| --- | --- |
| `schema_version` | 诊断详情 |
| `trace_id` | 所有错误状态、诊断详情 |
| `generated_at` | 总览、诊断详情 |
| `runtimes` | 总览、Codex助手、Claude助手 |
| `config_sources` | Runtime 页面 config sources table |
| `plugins` | Plugins 页面和 runtime tabs |
| `skills` | Skills 页面和 runtime tabs |
| `mcp_servers` | MCP 页面和 runtime tabs |
| `hooks` | Hooks / Risks 页面和 runtime tabs |
| `conflicts` | 总览、Runtime 页面 config sources table |
| `risk_findings` | 总览 risk summary、Hooks / Risks 页面 |
| `summary` | 总览 counts 和 overall status |
| `redaction_notice` | 总览、诊断详情 |

UI 不应自行重新解析 `.codex`、`.claude`、plugin manifest、MCP config 或 hook config。若需要刷新，必须重新调用 sidecar / doctor。

## 验收场景

UI 风险等级、标题、message 和 recommended action 的默认文案见 `docs/mvp1-risk-copy.md`。UI 可以按平台微调语气，但不得把 `not_tested` 渲染为失败，不得把 MVP 2 draft 操作渲染成可点击成功路径。

MVP 1 UI smoke 必须覆盖：

- 无 Codex / 无 Claude。
- Codex only。
- Claude only。
- 双 runtime。
- malformed TOML / JSON / YAML。
- secret redaction。
- 权限不足。
- sidecar missing。
- sidecar version mismatch。
- scan timeout。
- large scan：100 plugins、200 skills、50 MCP servers、1k 配置相关文件。

验收截图不得包含 API key、token、cookie、Authorization header、prompt、transcript、tool payload、shell output、nonce 明文或完整私有路径。
