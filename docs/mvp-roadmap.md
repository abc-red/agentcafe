# Agent Café MVP 路线图

## MVP 1：只读体检

目标：在不写入任何配置的前提下，建立 Codex助手 / Claude助手 的本机健康检查基线。

交付物：

- Runtime 检测：`codex` / `claude` 是否安装、版本、路径、PATH 状态。
- 配置白名单扫描：全局配置、项目配置、Plugins、Skills、MCP、Hooks、Rules。
- 风险提示：缺失、冲突、未知来源、危险 hook、secret-like 字段。
- UI 基础页面：总览、Codex助手、Claude助手、风险列表、诊断摘要。
- `agentcafe doctor --json` 输出满足 `DiagnosticReport` contract。

验收标准：

- 无 runtime、Codex only、Claude only、双 runtime 场景都能给出明确状态。
- 不读取 transcript、debug logs、session JSONL、file-history。
- 不执行写入，不创建 snapshot，不启动 MCP server，不执行 Hook 或 Plugin command。
- 不把 secret、prompt、tool payload 或 shell output 写入日志、报告、IPC response。

## 验收证据

MVP 1 必须提供 `agentcafe doctor --json` 或等价 sidecar IPC 诊断输出。

诊断 JSON 必须包含：

- `schema_version`
- `trace_id`
- `generated_at`
- `runtimes`
- `config_sources`
- `plugins`
- `skills`
- `mcp_servers`
- `hooks`
- `conflicts`
- `risk_findings`
- `summary`
- `redaction_notice`

诊断 JSON 必须通过 `schemas/diagnostic-report.schema.json` 校验。

必须提供 fixture 覆盖清单：

- Codex only。
- Claude only。
- 双 runtime。
- 无 runtime。
- malformed config。
- secret redaction。
- 权限不足。
- large scan。

验收报告不得包含 API key、token、cookie、Authorization header、prompt、transcript、tool payload、shell output、nonce 明文或私有协议帧。

## MVP 2：安全编辑

目标：在可预览、可回滚的前提下开放有限配置写入。

交付物：

- MCP server 添加、禁用、删除。
- plugin 启用、禁用、更新。
- Skill 创建、编辑、校验。
- 项目级 `.codex` / `.claude` 配置编辑。
- diff、snapshot、atomic write、re-validate、restore。

验收标准：

- 所有写入必须先展示 diff。
- 所有写入前必须创建 snapshot。
- snapshot 必须使用应用私有数据目录，不允许用户在 MVP 自定义目录。
- 写入失败必须有明确错误码和恢复入口。
- malformed TOML / JSON / YAML 不得导致 sidecar panic。
- Windows 文件锁、macOS 权限不足等场景有明确诊断。

## 写入验收证据

MVP 2 必须提供：

- `config.diff` dry-run 输出样例。
- `config.apply` 成功证据。
- snapshot manifest 脱敏样例。
- `backup.restore` / restore 成功证据。
- 权限不足 fixture。
- 文件锁 fixture。
- malformed config fixture。
- atomic write 失败 fixture。
- restore 失败 fixture。
- MCP / plugin / Skill 修改不泄露 secret 的报告。

MVP 2 验收报告不得包含 API key、token、cookie、prompt、transcript、tool payload、shell output、nonce 明文或 snapshot 原文内容。写入失败必须返回稳定错误码和可诊断 `trace_id`，restore 失败不得伪造成成功。

## MVP 3：版本与生态

目标：形成可持续管理的插件、Skill 和版本生态入口。

交付物：

- 插件 marketplace 浏览。
- Skill 模板库。
- 配置 profile 切换。
- Codex / Claude CLI 升级建议。
- 团队配置模板导入导出。

验收标准：

- 导出模板只包含配置白名单。
- marketplace 插件安装前显示 manifest、权限和风险。
- 版本建议不自动改动用户环境。
- 不读取或导出 API key、token、cookie、prompt、transcript。

## v2：高级原生能力

目标：提升原生操控体验和长期运维能力。

候选能力：

- 托盘或菜单栏常驻健康监控。
- MCP server 运行状态监控。
- 自动升级建议或可回滚自动升级。
- 更完整的配置 profile、模板和团队策略导入。
- Windows Credential Manager / macOS Keychain 更深集成。

准入条件：

- MVP 2 的写入、snapshot、restore 机制稳定。
- sidecar 安全边界完成测试；如未来启用 `loopback local service`，必须另行完成 v2 安全验收。
- 自动升级具备可回滚策略和清晰权限提示。

## 明确延后

- 云同步。
- 团队策略远程下发。
- 在线审计。
- transcript 管理。
- 完整 conversation / session 浏览器。
- 直接接管 Codex / Claude 登录态或 API key。
