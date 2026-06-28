# Agent Café 产品方案

## 背景与目标

`Agent Café`（中文名：`Agent咖啡`）是一个独立原生桌面产品，用于管理开发者本机 Codex、Claude Code 及未来 Agent runtime 的配置生态。

产品同时保留两个功能入口名：

- `Codex助手`：面向 Codex 用户。
- `Claude助手`：面向 Claude Code 用户。

本项目目标是把分散在 `.codex`、`.claude`、项目配置、Plugins、Skills、MCP、Hooks 和 CLI 版本中的本机 Agent 配置能力，整理为可视化、可诊断、可恢复的原生控制台。

## 产品定位

Agent Café 是一个本机优先的配置管理器，不是远程控制服务，也不是 transcript 同步服务。

核心能力：

- 本机配置总览与健康检查。
- Plugins、Skills、MCP、Hooks、Rules 的发现、校验和管理。
- Codex / Claude Code 版本检测和升级建议。
- 配置冲突检测、风险提示、备份与回滚。
- Windows 和 macOS 原生操控体验。

首期默认不做云同步，不上传 `.codex` / `.claude` 私密内容，不接管 Codex / Claude API key 或登录态。Codex auth/session/token/account state、raw rollout、logs/goals/memories 正文，以及 Claude `primaryApiKey` 均属于高敏边界，默认不读取明文。

## 目标用户

- Codex 重度用户：需要管理 Codex 配置、Plugins、Skills、MCP、Hooks 和版本。
- Claude Code 重度用户：需要管理 Claude Code settings、Skills、Plugins、MCP、Hooks 和本机运行环境。
- 多 Agent runtime 用户：需要统一查看本机 Agent 工具链状态。
- 团队工具负责人：需要管理团队标准配置、MCP server、插件、安全规则和 Skill 模板。

## 命名

```text
产品总名：Agent Café（中文名：Agent咖啡）
功能入口：Codex助手 / Claude助手
仓库名：agentcafe
CLI：agentcafe
Tagline：A native control center for Codex, Claude Code, MCP, plugins, and skills.
```

## 最终技术方案

- Windows v1：`WPF + C#`。
- macOS v1：`SwiftUI + AppKit fallback`。
- Core：`Rust sidecar`。
- UI 与 Rust：通过 `stdio JSON-RPC 2.0` 通信。
- v1 运行形态：UI 启动 Rust sidecar 子进程，sidecar 不监听任何端口。
- v1 握手：UI 必须先调用 `ipc.handshake`，校验协议版本、sidecar 版本、UI capability 和一次性启动 nonce。
- v1 边界：UI 进程不承载复杂配置解析、配置写入、风险扫描、MCP 测试、snapshot 或 restore 逻辑。
- v2 预留：`loopback local service` 仅作为高级模式候选，不进入 MVP 默认路径。

## MVP 范围

### MVP 1：只读体检

- 检测 `codex` / `claude` 是否安装、版本、可执行路径、PATH 状态。
- 扫描 `.codex` / `.claude` 配置白名单。
- 展示 Plugins、Skills、MCP、Hooks、Rules、项目级配置来源。
- 输出健康状态、配置冲突、缺失项、风险提示。
- 输出 `agentcafe doctor --json`，并满足 `DiagnosticReport` contract。
- 不执行配置写入、snapshot、restore、MCP 连接测试、Hook dry-run 或 Plugin 命令。

### MVP 2：安全编辑

- 支持添加、禁用、删除 MCP server。
- 支持启用、禁用、更新 plugin。
- 支持创建、编辑、校验 Skill。
- 支持项目级 `.codex` / `.claude` 配置管理。
- 所有写入必须先 diff、再 snapshot、再原子写入、最后校验。

### MVP 3：版本与生态

- 插件 marketplace 浏览。
- Skill 模板库。
- 配置 profile 切换。
- Codex / Claude CLI 升级建议。
- 团队配置模板导入导出，但只导出配置白名单。

### v2：高级原生能力

- 常驻健康监控。
- MCP server 运行状态监控。
- 自动升级建议或可回滚自动升级。
- 更完整的配置 profile、模板和团队策略导入。

## 核心模块

职责按最终能力描述；MVP 1 只实现只读诊断子集，写入和外部连接测试能力保留为 MVP 2 draft。

| 模块 | 职责 |
| --- | --- |
| `RuntimeDetector` | 发现 Codex / Claude Code 安装、版本、路径、平台来源和可执行状态。 |
| `ConfigInventory` | 读取字段级配置白名单，解析 TOML / JSON / YAML，输出来源、优先级、冲突和校验结果。 |
| `PluginManager` | 解析插件 manifest，展示插件来源、版本、启用状态和包含的 Skills / MCP / Hooks。 |
| `SkillManager` | 发现并校验 `SKILL.md`，检查 frontmatter、引用资源和跨 runtime 兼容性。 |
| `McpManager` | MVP 1 展示 MCP server transport、scope、命令或 URL 摘要；MVP 2 才允许用户触发连接测试并列出 tools / resources / templates。 |
| `SafetyScanner` | 检测 secret、危险 hook、未知来源、敏感路径和外泄风险。 |
| `BackupStore` | MVP 2 写入前创建 snapshot，支持 diff、restore 和失败回滚；MVP 1 不创建 snapshot。 |
| `VersionManager` | 检测 Codex / Claude Code 版本和安装来源，提供升级建议。 |

## 明确非范围

- 不上传 `.codex` / `.claude` 私密内容。
- 不默认读取完整 session JSONL、transcript、debug logs、file-history。
- 不接管 Codex / Claude API key 或登录态。
- 不绕过 Codex / Claude 官方配置机制直接修改私有运行时状态。
- 不在 MVP 实现云同步、团队策略下发或在线审计。
- 不改变 AgentRemoter public contract。

## 验收标准

- 能在无 Codex、无 Claude、单 runtime、双 runtime 场景下给出明确健康状态。
- 能在不泄露 secret 和 transcript 的前提下展示配置、Plugins、Skills、MCP 和 Hooks。
- MVP 1 `agentcafe doctor --json` 必须通过 `schemas/diagnostic-report.schema.json` 校验。
- MVP 1 不执行写入、snapshot、restore、MCP 连接测试、Hook dry-run 或 Plugin 命令。
- MVP 2 的所有写入类能力都有 diff、snapshot、原子写入、校验和恢复路径。
- MVP 2 snapshot 使用应用私有数据目录，不允许用户自定义到项目目录、Git 工作区或云同步目录。
- MVP 2 MCP / Hook / Plugin 测试必须有 timeout 和稳定错误码。
