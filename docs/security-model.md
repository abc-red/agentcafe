# Agent Café 安全模型

## 安全目标

Agent Café 会接触 `.codex`、`.claude`、项目配置、Plugins、Skills、MCP 和 Hooks 等本机敏感配置面。安全目标是：

- 最小读取。
- 默认脱敏。
- 本机优先。
- 写入可回滚。
- 不把 runtime 私密数据变成日志、报告、snapshot 或 IPC 泄露。

## 禁止读取、展示或上传的内容

默认不得读取、展示、上传或写入报告：

- API key、token、cookie、runtime 登录态。
- prompt 正文、完整 transcript、tool payload、shell output。
- Codex / Claude 私有协议帧。
- workspace secret、证书私钥、调试日志全文。
- `~/.claude/config.json.primaryApiKey` 或等价 Claude API key 字段。
- Codex auth、session、token、account state、logs DB 正文、goals / memories 正文、raw rollout JSONL。

如必须为排障引用敏感相关信息，只能使用不可逆 hash、长度、字段名、错误码、trace id 或前后缀脱敏摘要。

## Redaction 格式

所有 IPC response、日志、诊断报告和 UI 风险证据必须使用统一脱敏对象，不得返回原值：

```json
{
  "field": "mcp_servers.github.env.GITHUB_TOKEN",
  "length": 40,
  "sha256_12": "3b7d9f1a0c4e",
  "source_summary": "user config",
  "path": "~/redacted/.codex/config.toml"
}
```

规则：

- `field` 只记录字段路径或配置 key，不记录值。
- `length` 记录原值字符长度；二进制或不可解码内容记录 byte length。
- `sha256_12` 使用 SHA-256 后取前 12 个 hex 字符，只用于同一次本机诊断内比对。
- `source_summary` 只能是来源类型，例如 `user config`、`project mcp config`、`plugin manifest`。
- `path` 必须是脱敏路径，不得包含 workspace secret、token、session id 或随机 nonce。

## Secret-like 检测类别

MVP 1 `SafetyScanner` 至少检测以下类别：

| 类别 | 示例字段或模式 | 默认 severity |
| --- | --- | --- |
| API key | `api_key`, `apikey`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` | `high` |
| token | `token`, `access_token`, `refresh_token`, `session_token` | `high` |
| cookie | `cookie`, `set-cookie` | `high` |
| Authorization header | `Authorization`, `Bearer ...`, `X-API-Key` | `high` |
| env secret | MCP / plugin / hook env 中的 `*_KEY`, `*_TOKEN`, `*_SECRET` | `high` |
| private key | PEM markers such as private key headers | `critical` |
| known provider key pattern | 常见 provider key 前缀或长度特征 | `high` |
| password-like field | `password`, `passwd`, `client_secret` | `high` |

检测结果只返回 redaction 对象。扫描器不得把命中的原始值写入 panic、debug log、snapshot、fixture report 或 IPC response。

## Codex 高敏文件红线

- `~/.codex/sessions` 和 raw rollout JSONL 默认不读取明文。
- Codex logs DB 默认不读取正文；只允许读取错误码、级别、计数、时间戳等脱敏摘要。
- goals / memories DB 默认不读取正文；`objective`、`raw_memory`、`rollout_summary` 等文本字段不得进入 UI、日志、报告、snapshot 或 IPC response。
- auth / session / token / account state 默认不读明文。
- MCP env secret、plugin secret、runtime 登录态只显示“已配置 / 缺失 / 来源摘要”，不显示值。
- SQLite metadata 仅允许读取配置或状态需要的白名单字段；prompt、preview、first_user_message、log body、memory summary 默认不读取或不展示。

## Claude 高敏文件红线

- `~/.claude/config.json` 默认不读取明文；其中 `primaryApiKey` 永不展示、永不写入日志、永不进入 IPC response 或 snapshot。
- `~/.claude.json` 只允许白名单字段级读取，例如 plugin / MCP / settings 状态摘要；账号、登录态、项目私密 metadata 必须脱敏或跳过。
- `~/.claude/stats-cache.json` 仅允许读取聚合计数和模型名摘要，不读取 prompt、路径或会话正文。

## 默认不扫描路径

默认不扫描：

- `~/.codex/sessions`
- `~/.claude/projects/**/*.jsonl`
- `~/.claude/file-history`
- `~/.claude/debug`
- 完整 conversation / transcript 存储

这些路径可能包含 prompt、完整 transcript、tool payload、shell output、workspace 内容或私有协议帧。MVP 不依赖这些路径实现配置管理能力。

## 配置白名单原则

Agent Café 只读取完成配置管理所需的白名单路径和字段。

白名单读取应满足：

- 读取前明确 runtime、scope 和用途。
- 只解析必要字段。
- 未知字段默认保留但不展示敏感原文。
- 解析失败返回明确错误码，不导致 sidecar panic。

## Snapshot 与 Restore

snapshot 只能包含配置白名单。

不得进入 snapshot：

- API key、token、cookie、runtime 登录态。
- prompt 正文、完整 transcript、tool payload、shell output。
- debug logs、file-history、session JSONL。

## Snapshot 存储位置与生命周期

- MVP 默认不允许用户选择任意 snapshot 目录。
- macOS 默认目录：`~/Library/Application Support/AgentCafe/snapshots`。
- Windows 默认目录：`%LOCALAPPDATA%\AgentCafe\snapshots`。
- snapshot 不得写入项目目录、`.codex`、`.claude`、Git 工作区或常见云同步目录。
- snapshot 文件权限必须限制为当前用户读写。
- 默认 retention：保留最近 20 个或最近 30 天，支持本地清理。
- snapshot manifest 只记录脱敏摘要、hash、时间戳、变更路径摘要和 restore 状态。
- restore 前必须验证目标路径仍在允许范围内；restore 失败不得伪造成成功。

写入流程必须是：

```text
validate input
  -> diff preview
  -> create snapshot
  -> atomic write
  -> re-validate
  -> success or restore path
```

MVP 1 不创建 snapshot。MVP 2 实现前必须补充 snapshot manifest schema、文件权限检查、restore 失败语义和 retention 测试。

## IPC 安全

v1 sidecar 由 UI 作为子进程启动，通过 stdio 通信，不监听任何端口。sidecar 启动后必须先执行 `ipc.handshake`，校验协议版本、sidecar 版本、UI capability 和一次性启动 nonce；握手完成前不得接受其他方法。nonce 明文不得进入日志、报告、snapshot 或普通 IPC response。

IPC response 不得返回：

- secret 明文。
- 完整 transcript。
- tool payload。
- shell output。
- nonce 明文。
- Codex / Claude 私有协议帧。

IPC request 必须校验：

- method 是否支持。
- 参数类型。
- 枚举值。
- payload 大小。
- 路径是否在允许范围内。

所有 IPC 调用必须有 timeout。长耗时任务必须支持取消或明确返回 `timeout` / `canceled`。

MVP 1 中以下行为必须返回 `feature_not_in_mvp`，不得执行副作用：

- `config.apply`
- `plugin.enable`
- `plugin.disable`
- `skill.create`
- `mcp.test`
- `backup.create`
- `backup.restore`

## MCP / Hook 测试规则

MVP 1 只允许静态分析 MCP、Hook 和 Plugin metadata，不得启动 MCP server、不得执行 hook command、不得调用 hook HTTP endpoint、不得调用 plugin command。

MVP 2 若开放测试能力，必须满足：

- 必须由用户显式触发，不能由扫描自动触发。
- 必须设置 wall-clock timeout 和 idle timeout。
- 必须提供取消入口。
- 默认不得执行 destructive command、外部网络 POST、credential helper 或 plugin install/update 脚本。
- 只记录状态、稳定错误码、trace id、tool/resource/template 计数和脱敏 endpoint/command 摘要。
- 不记录完整 tool payload、shell output、HTTP body、headers 明文或 MCP protocol frame。
- 测试失败不得伪造成成功；必须返回 `timeout`、`permission_denied`、`connection_failed`、`auth_required` 或其他稳定业务码。

## Local Service 边界

MVP 默认不启用 loopback local service。如果 Rust core 在 v2 或高级模式中以 local service 运行：

- 只能监听 loopback。
- 不暴露公网或局域网端口。
- 必须使用一次性 session token 或等价握手凭据。
- 必须绑定启动方上下文，不能接受未授权本机进程调用。
- 不执行未经用户确认的外部命令。
- 不把 MCP / Hook / Plugin 测试结果中的敏感输出写入日志。

## 日志与诊断

允许记录：

- `trace_id`。
- 稳定错误码。
- runtime 名称。
- 配置来源类型。
- 脱敏路径摘要。
- hash、长度、计数、时间戳。
- method 名称。
- stage 名称。
- duration_ms。

禁止记录：

- API key、token、cookie。
- prompt 正文。
- 完整 transcript。
- tool payload。
- shell output 全文。
- nonce 明文。
- Codex / Claude 私有协议帧。

日志记录必须结构化，推荐字段：

```json
{
  "trace_id": "trace-id",
  "level": "warn",
  "method": "config.scan",
  "stage": "parse",
  "runtime": "codex",
  "source_kind": "user_config",
  "path": "~/redacted/.codex/config.toml",
  "code": "config_invalid",
  "duration_ms": 18
}
```

不得把 arbitrary exception string 原样透传到用户报告；需要先做 secret-like scrub，再映射为稳定错误码和简短 message。

## 安全验收

- 构造含 API key、token、cookie 的 fixture，确认不会进入日志、报告、snapshot、IPC response。
- 构造含 Authorization header、private key、provider key pattern、env secret 的 fixture，确认只输出 redaction 对象。
- 构造含 prompt、transcript、tool payload 的 fixture，确认默认扫描会跳过。
- MVP 1 扫描不得启动 MCP server、执行 Hook、执行 Plugin command 或创建 snapshot。
- MVP 2 MCP / Hook 测试必须有 timeout，且不自动执行危险命令。
- v1 sidecar 不监听任何端口。
- loopback local service 如未来启用，必须使用一次性 session token 或等价握手凭据，并确认不监听非 loopback 地址。
