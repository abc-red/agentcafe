# Agent Café 开发计划

## Summary

`Agent Café`（中文名：`Agent咖啡`）是独立原生桌面项目，目标是提供 `Codex助手` / `Claude助手` 的本机配置管理能力。v1 技术路线固定为 Windows `WPF + C#`、macOS `SwiftUI + AppKit fallback`、Core `Rust sidecar`，UI 通过 `stdio JSON-RPC 2.0` 调用 Rust。开发顺序先收敛文档契约，再实现 Rust core / sidecar，再接 Windows 与 macOS 原生 UI，最后进入 MVP 验收。

v1 不启用 `loopback local service`、托盘常驻、菜单栏常驻、自动升级、团队策略远程下发或云同步。这些能力全部延后到 v2 或高级模式。

## Phase 0：文档契约收敛

目标：把文档修到可执行状态，避免实现阶段反复猜边界。

交付物：

- `docs/ipc-contract.md` 固定 v1 为 `stdio JSON-RPC 2.0`。
- JSON-RPC 顶层 `error.code` 使用整数，业务码放入 `error.data.code`。
- `docs/ipc-contract.md` 为 MVP 1 方法补齐 params/result/error 示例，并提供 `agentcafe doctor --json` golden sample。
- `docs/config-sources.md` 固定 Codex / Claude 配置来源矩阵、scope、priority、读取策略和敏感字段策略。
- `docs/security-model.md` 覆盖 Codex / Claude 高敏文件红线、redaction 格式、secret-like 类别、结构化日志白名单和 MVP 1 禁止副作用清单。
- `docs/ui-product-spec.md` 固定 MVP 1 页面、状态、风险展示和 doctor JSON 到 UI 的映射。
- `docs/mvp1-acceptance.md` 固定 MVP 1 completion gate、CLI contract、fixture coverage、UI acceptance 和安全验收。
- `docs/mvp1-risk-copy.md` 固定 MVP 1 风险等级和用户可见文案方向。
- snapshot 私有目录、权限、retention、禁止 Git / 云同步目录策略明确。
- `docs/mvp-roadmap.md` 覆盖 MVP 1 诊断验收和 MVP 2 写入验收证据。
- `docs/mvp2-write-draft.md` 作为 MVP 2 预研，不进入 MVP 1 实现。
- `docs/security-checklist.md` 作为 MVP 1 release gate 和 MVP 2 写入准入检查清单。

验收：

- 文档 review 无剩余阻塞问题。
- `README.md`、`docs/architecture.md`、`docs/ipc-contract.md` 对 v1 IPC 描述一致。
- `docs/ipc-contract.md`、`docs/config-sources.md`、`docs/ui-product-spec.md` 对 MVP 1 只读边界描述一致。
- `fixtures/diagnostic/reports/*.json` 通过 `node tests/integration/validate-diagnostic-fixtures.mjs`。
- `docs/security-checklist.md` 的 MVP 1 read-only、redaction、IPC、fixture gate 均能映射到测试或人工验收证据。
- `rg` 检查确认无真实 secret、无字符串业务码作为 JSON-RPC 顶层 `error.code`。

## Phase 1：Rust Core 与 Sidecar 基线

目标：实现只读能力和标准 IPC，先不做写入。

交付物：

- 建立 Rust workspace：
  - `core/agentcafe-core`
  - `core/agentcafe-sidecar`
  - `core/agentcafe-cli`
- 实现核心模型：
  - `RuntimeProfile`
  - `ConfigSource`
  - `RiskFinding`
  - `PluginItem`
  - `SkillItem`
  - `McpServerItem`
  - `HookItem`
  - `ConflictFinding`
  - `DiagnosticReport`
  - `AgentCafeError`
- 实现 `stdio JSON-RPC 2.0` sidecar：
  - `ipc.handshake`
  - `runtime.list`
  - `runtime.probe`
  - `config.scan`
  - `config.validate`
  - `plugin.list`
  - `skill.list`
  - `mcp.list`
  - `risk.scan`
- 实现只读扫描：
  - Codex 配置白名单。
  - Claude 配置白名单。
  - Plugins / Skills / MCP / Hooks metadata。
  - 默认跳过 sessions、transcript、debug logs、file-history、raw rollout。

验收：

- `agentcafe doctor --json` 输出 runtime 状态、配置来源计数、风险计数、错误码、`trace_id`、脱敏说明。
- `agentcafe doctor --json` 输出包含 `schema_version`、`trace_id`、`generated_at`、`runtimes`、`config_sources`、`plugins`、`skills`、`mcp_servers`、`hooks`、`conflicts`、`risk_findings`、`summary`、`redaction_notice`。
- `agentcafe doctor --json` 输出通过 `schemas/diagnostic-report.schema.json` 校验。
- fixture 覆盖 Codex only、Claude only、双 runtime、无 runtime、malformed config、secret redaction、权限不足、large scan。
- 报告 fixture 与真实 CLI 输出都满足 `docs/mvp1-acceptance.md` 的 completion gate。
- 不产生配置写入，不创建 snapshot，不启动 MCP server，不执行 Hook 或 Plugin command。
- 所有响应包含稳定错误码或明确成功状态，不泄露 API key、token、cookie、prompt、transcript、tool payload、shell output 或 nonce 明文。

## Phase 2：安全写入、Snapshot 与 Restore

目标：在 Rust core 内实现写入闭环，UI 仍只调用 sidecar。

交付物：

- 实现写入相关 IPC：
  - `config.diff`
  - `config.apply`
  - `backup.list`
  - `backup.create`
  - `backup.restore`
  - `plugin.enable`
  - `plugin.disable`
  - `skill.create`
  - `mcp.test`
- 实现写入流程：
  - validate input
  - diff preview
  - create snapshot
  - atomic write
  - re-validate
  - success or restore path
- 实现 snapshot 策略：
  - macOS：`~/Library/Application Support/AgentCafe/snapshots`
  - Windows：`%LOCALAPPDATA%\AgentCafe\snapshots`
  - 当前用户读写权限。
  - 默认保留最近 20 个或最近 30 天。
  - manifest 只记录脱敏摘要、hash、时间戳、变更路径摘要和 restore 状态。

验收：

- `config.diff` dry-run 输出样例。
- `config.apply` 成功证据。
- snapshot manifest 脱敏样例。
- `backup.restore` 成功证据。
- 权限不足、文件锁、malformed config、atomic write 失败、restore 失败 fixture。
- 报告不包含 API key、token、cookie、prompt、transcript、tool payload、shell output、nonce 明文或 snapshot 原文内容。

## Phase 3：Windows WPF Shell

目标：实现 Windows v1 原生 UI，所有业务能力通过 sidecar。

交付物：

- 建立 `apps/windows-wpf`。
- WPF 主窗口。
- 原生导航：总览、Codex助手、Claude助手、MCP、Plugins、Skills、风险、备份。
- sidecar 启动、握手、重启、崩溃提示。
- `agentcafe doctor --json` 结果可视化。

Windows 集成：

- PATH / Registry / PowerShell 检测入口。
- Credential Manager 只显示 secret 状态，不读取明文。
- 文件权限和文件锁错误展示。
- 托盘作为 v2 候选，MVP 可先不做常驻。

验收：

- Windows 10 / Windows 11 smoke。
- 无 runtime、Codex only、Claude only、双 runtime 都能展示。
- sidecar missing、version mismatch、timeout、crash 有明确 UI 状态。
- UI 不直接读写 `.codex` / `.claude` 复杂配置。

## Phase 4：macOS SwiftUI Shell

目标：实现 macOS v1 原生 UI，保持与 Windows 共用 sidecar 契约。

交付物：

- 建立 `apps/macos`。
- SwiftUI 主窗口。
- AppKit fallback 用于菜单、文件面板、Finder reveal 或窗口能力。
- sidecar 启动、握手、重启、崩溃提示。
- 与 Windows 同样的页面结构和状态模型。

macOS 集成：

- `~` 展开、app bundle、Homebrew、npm 检测。
- Keychain 只显示 secret 状态，不读取明文。
- Finder reveal 只对用户明确选择的路径执行。
- LaunchAgent / 菜单栏常驻作为 v2 候选。

验收：

- macOS Intel / Apple Silicon smoke。
- 权限不足、路径空格、中文路径、sidecar crash 都有明确 UI 状态。
- UI 不直接实现配置解析、写入、snapshot 或 MCP 测试逻辑。

## Phase 5：MVP 1 Release Gate

目标：把只读体检、标准 IPC、`agentcafe doctor --json` 和双平台只读 UI skeleton 组合成 MVP 1 发布候选。

自动测试：

- Rust parser / redaction / IPC schema / `DiagnosticReport` schema。
- fixture 覆盖 Codex、Claude、MCP、Plugin、Skill、Hook。
- JSON-RPC envelope 兼容性测试。
- secret redaction 测试。

手工 smoke：

- Windows 10 / 11。
- macOS Intel / Apple Silicon。
- 无 runtime、Codex only、Claude only、双 runtime。
- large scan：100 plugins、200 skills、50 MCP servers、1k 配置相关文件。

安全验收：

- 无 API key、token、cookie、prompt、transcript、tool payload、shell output 泄露到日志、报告、IPC response。
- 不产生配置写入，不创建 snapshot，不启动 MCP server，不执行 Hook 或 Plugin command。
- v1 sidecar 不监听任何端口。
- `loopback local service` 不进入 MVP 默认路径。

## Phase 6：MVP 2 Release Gate

目标：在 MVP 1 contract 稳定后，把安全写入、snapshot、restore 和用户触发的 MCP / Hook / Plugin 测试组合成 MVP 2 发布候选。

进入条件：

- MVP 2 IPC 方法 params/result schema 已冻结。
- snapshot manifest schema 已冻结。
- 写入确认模型、restore 失败语义和测试副作用策略已完成安全 review。

自动测试：

- diff / apply / snapshot / restore。
- atomic write failure。
- permission denied。
- file lock。
- malformed TOML / JSON / YAML。
- MCP / Hook / Plugin 测试 timeout 与 redaction。

安全验收：

- snapshot manifest 不包含 secret、prompt、transcript、tool payload、shell output 或 nonce 明文。
- restore 失败不得伪造成成功。
- MCP / Hook / Plugin 测试必须由用户显式触发，且不自动执行危险命令。

## Assumptions

- v1 默认只使用 `stdio JSON-RPC 2.0`。
- `loopback local service`、托盘常驻、菜单栏常驻、自动升级、团队策略远程下发和云同步全部延后。
- MVP 2 snapshot 目录不允许用户自定义。
- `agentcafe` 是仓库名和 CLI 名；`Agent Café` 是产品展示名；`Agent咖啡` 是中文名；`Codex助手` / `Claude助手` 是功能入口名。
- 本计划不初始化 Rust、WPF、SwiftUI 工程，不引入构建系统。
