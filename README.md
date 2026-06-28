# Agent Café

`Agent Café`（中文名：`Agent咖啡`）是一个原生桌面配置管理工具，用于管理开发者本机 Codex、Claude Code 及未来 Agent runtime 的配置生态。

它同时提供两个面向用户心智的功能入口：

- `Codex助手`：管理 `.codex` 配置、Codex Plugins、Skills、MCP、Hooks 和版本状态。
- `Claude助手`：管理 `.claude` 配置、Claude Code Skills、Plugins、MCP、Hooks 和版本状态。

CLI 名称固定为：

```text
agentcafe
```

Tagline：

```text
A native control center for Codex, Claude Code, MCP, plugins, and skills.
```

## 状态

本仓库当前处于 `v0.1.0-mvp1-rc1` 收口阶段。MVP 1 的 Rust core、CLI、stdio sidecar、`agentcafe doctor --json`、`DiagnosticReport` schema、fixture、redaction 和安全边界测试已经落地。

Windows WPF 与 macOS SwiftUI 原生 UI 尚未接入；MVP 1 RC 的完成定义是可复跑的只读 doctor 诊断基线，不包含原生 UI 发布包。MVP 2 写入、snapshot、restore、MCP 连接测试、Hook dry-run 和 Plugin 命令仍保持禁用。

## 技术方案

Agent Café 采用“双原生 UI + Rust sidecar”的 v1 架构。v1 默认运行形态是 UI 启动 Rust sidecar 子进程，并通过 `stdio JSON-RPC 2.0` 通信；loopback local service 只作为 v2 或高级模式预留。

架构图：

```text
Windows v1: WPF + C#
macOS v1: SwiftUI + AppKit fallback
        ↓
IPC
        ↓
Rust sidecar
        ↓
Config / Plugin / Skill / MCP / Backup / Version Managers
```

原则：

- Windows v1 使用 `WPF + C#`。
- macOS v1 使用 `SwiftUI + AppKit fallback`。
- Core 使用 `Rust sidecar`；`local service` 仅作为 v2 或高级模式预留。
- v1 IPC 固定为 `stdio JSON-RPC 2.0`，成功响应使用标准 `result` envelope。
- UI 通过 `IPC` 调用 Rust，不把复杂配置解析、写入、风险扫描和备份回滚逻辑塞进 UI 进程。

## 核心能力

最终产品能力包括：

- Codex / Claude Code 安装与版本检测。
- `.codex` / `.claude` 配置白名单扫描。
- Plugins、Skills、MCP、Hooks、Rules 发现与校验。
- 配置冲突检测、风险提示、diff、snapshot 和 restore。
- MCP server 连接测试和工具列表读取。
- 面向 Windows 与 macOS 的原生操控体验。

MVP 1 聚焦只读体检：不得执行配置写入、snapshot、restore、MCP 连接测试、Hook dry-run 或 Plugin 命令。写入类能力从 MVP 2 开始，但必须先冻结写入 IPC schema 与安全流程。

## 安全红线

默认不得读取、展示、上传或写入报告：

- API key、token、cookie、runtime 登录态。
- prompt 正文、完整 transcript、tool payload、shell output。
- Codex / Claude 私有协议帧。
- workspace secret、证书私钥、调试日志全文。
- Codex auth、session、token、account state、logs / goals / memories 正文、raw rollout JSONL。
- `~/.claude/config.json.primaryApiKey` 或等价 Claude API key 字段。

默认不扫描：

- `~/.codex/sessions`
- `~/.claude/projects/**/*.jsonl`
- `~/.claude/file-history`
- debug logs
- 完整 conversation / transcript 存储

MVP 1 不执行任何配置写入。MVP 2 及之后的所有配置写入必须先 diff、再 snapshot、再原子写入、最后校验，并提供失败恢复路径。MVP 2 snapshot 必须使用应用私有数据目录，不允许用户自定义到项目目录、Git 工作区或云同步目录。

## 文档

- [产品方案](docs/product-plan.md)
- [架构设计](docs/architecture.md)
- [安全模型](docs/security-model.md)
- [IPC 契约](docs/ipc-contract.md)
- [MVP 路线图](docs/mvp-roadmap.md)
- [配置来源](docs/config-sources.md)
- [MVP 1 验收标准](docs/mvp1-acceptance.md)
- [MVP 1 风险文案](docs/mvp1-risk-copy.md)
- [MVP 2 写入预研草案](docs/mvp2-write-draft.md)
- [安全检查清单](docs/security-checklist.md)
- [MVP 1 UI 产品规格](docs/ui-product-spec.md)
- [开发计划](docs/development-plan.md)
- [MVP 1 Release Notes](docs/mvp1-release-notes.md)
- [DiagnosticReport JSON Schema](schemas/diagnostic-report.schema.json)

MVP 1 RC 的验收入口是 `agentcafe doctor --json` 或等价 sidecar IPC 诊断输出。现有诊断报告样本位于 `fixtures/diagnostic/reports/`。

## MVP 1 验证

提交或打 tag 前运行：

```sh
cargo test
cargo run -q -p agentcafe-cli -- doctor --json --schema schemas/diagnostic-report.schema.json > /tmp/agentcafe-doctor.json
node tests/integration/validate-diagnostic-fixtures.mjs
find . -path ./.git -prune -o -name '*.json' -print | sort | xargs -n1 jq empty
```

安全回归检查：

```sh
pattern=$(printf '%b' '\\u5c1a\\u65e0\\u4ea7\\u54c1\\u4ee3\\u7801|\\u5c1a\\u672a\\u521d\\u59cb\\u5316|\\u5f53\\u524d\\u5c1a\\u65e0\\u4ea7\\u54c1\\u4ee3\\u7801')
rg -n "$pattern" README.md docs
rg -n "AGENTCAFE_FIXTURE_SECRET|Bearer |-----BEGIN|tool payload body|transcript body|shell output body" fixtures/diagnostic/reports
```

## 仓库结构

```text
agentcafe/
  README.md
  docs/
    product-plan.md
    architecture.md
    security-model.md
    ipc-contract.md
    mvp-roadmap.md
    config-sources.md
    ui-product-spec.md
    development-plan.md
    mvp1-acceptance.md
    mvp1-risk-copy.md
    mvp2-write-draft.md
    security-checklist.md
    mvp1-release-notes.md
  schemas/
    diagnostic-report.schema.json
  core/
    agentcafe-core/
    agentcafe-sidecar/
    agentcafe-cli/
  apps/
    windows-wpf/
    macos/
  fixtures/
    codex/
    claude/
    diagnostic/
  tests/
    integration/
```

## 与 AgentRemoter 的关系

Agent Café 是独立产品和独立仓库。本项目不改变 AgentRemoter 的 HostAgent、AgentNode、RelayServer、ControlPlane、Runtime Adapter、public contract、Relay envelope、session state machine 或 OpenAPI 行为。
