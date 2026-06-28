# Agent Café 架构设计

## 总体架构

Agent Café 采用双原生 UI 和共享 Rust core。v1 默认运行形态是 UI 启动 Rust sidecar 子进程，并通过 `stdio JSON-RPC 2.0` 通信；loopback IPC / local service 仅作为 v2 或高级模式预留，不进入 MVP 默认路径。

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

UI 负责原生体验，Rust core 负责业务事实。复杂配置解析、写入、风险扫描、备份回滚和 CLI 调用都在 Rust sidecar 中完成。v1 sidecar 不监听任何端口。

## Windows Shell

Windows v1 使用 `WPF + C#`。

职责：

- 原生窗口、导航、表格、设置页和弹窗。
- 托盘、系统通知、Credential Manager、PowerShell、PATH / Registry 检测入口。
- 启动和监督 Rust sidecar。
- 通过 IPC 调用 Rust core。

不得在 WPF 进程中实现：

- `.codex` / `.claude` 复杂配置解析。
- 配置写入和回滚。
- MCP 测试逻辑。
- secret redaction 规则主实现。

## macOS Shell

macOS v1 使用 `SwiftUI + AppKit fallback`。

职责：

- 原生窗口、菜单栏、设置页、系统通知和 Finder reveal。
- Keychain、LaunchAgent 等平台集成入口。
- 启动和监督 Rust sidecar。
- 通过 IPC 调用 Rust core。

`AppKit fallback` 用于补足 SwiftUI 不适合承载的系统级能力，例如复杂菜单栏行为、窗口管理或系统文件面板细节。

## Rust Sidecar

Rust core 是跨平台业务事实来源。

职责：

- 文件系统访问和路径规范化。
- TOML / JSON / YAML 结构化解析。
- Codex / Claude 配置来源扫描。
- Plugins、Skills、MCP、Hooks 元数据解析。
- CLI 版本检测和安装来源识别。
- MCP server 测试。
- 风险扫描和敏感数据脱敏。
- diff、snapshot、原子写入、restore。
- 统一错误码、timeout 和 trace id。

## IPC 模型

v1 固定使用 `stdio JSON-RPC 2.0`。UI 以子进程方式启动 sidecar，通过 stdin / stdout 发送 JSON-RPC 请求和响应。loopback IPC / local service 是 v2 预留能力，不进入 MVP 默认路径。

约束：

- UI 启动 sidecar 后必须先调用 `ipc.handshake`。
- `ipc.handshake` 必须校验协议版本、sidecar 版本、UI capability 和一次性启动 nonce。
- 握手完成前，sidecar 必须拒绝除 `ipc.handshake` 之外的所有方法。
- 所有请求必须有 timeout。
- 所有请求参数必须校验方法名、类型、枚举、大小和路径范围。
- IPC response 不返回 secret 明文、完整 transcript、tool payload、shell output 或私有协议帧。
- 长耗时任务需要支持取消或返回 progress 事件。

如果 v2 启用 loopback local service：

- 必须只监听 loopback。
- 不暴露公网或局域网端口。
- 必须使用一次性 session token 或等价握手凭据。
- 必须绑定启动方上下文，不能接受未授权本机进程调用。
- 崩溃后 UI 应显示明确 degraded 状态，而不是静默失败。

## Sidecar 生命周期

基础流程：

```text
UI start
  -> locate bundled sidecar
  -> create one-time startup nonce
  -> start sidecar over stdio
  -> ipc.handshake(protocol_version, sidecar_version, ui_capabilities, nonce)
  -> config.scan / runtime.list
```

异常行为：

- sidecar 缺失：UI 显示 `sidecar_missing`。
- sidecar 版本不匹配：UI 显示 `sidecar_version_mismatch`。
- 启动超时：UI 显示 `sidecar_start_timeout`。
- 运行中崩溃：UI 标记 degraded，并允许用户重启 sidecar。

## 目录建议

```text
agentcafe/
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
  tests/
    integration/
```

## 与 AgentRemoter 的边界

Agent Café 是独立本机配置管理产品。本项目不改变 AgentRemoter 的 HostAgent、AgentNode、RelayServer、ControlPlane、Runtime Adapter、public contract、Relay envelope 或 session state machine。
