# Agent Café MVP 1 Release Notes

Release candidate: `v0.1.0-mvp1-rc1`

## 已完成能力

- Rust workspace：`agentcafe-core`、`agentcafe-cli`、`agentcafe-sidecar`。
- CLI：`agentcafe doctor --json`，支持 schema 校验入口。
- Sidecar：stdio JSON-RPC 2.0、`ipc.handshake`、只读诊断方法、MVP 2 draft 方法副作用拦截。
- Diagnostic contract：`DiagnosticReport` Rust model、`schemas/diagnostic-report.schema.json`、golden sample 和场景 fixtures。
- 只读扫描：runtime detection、配置白名单扫描、Plugins / Skills / MCP / Hooks metadata、冲突和风险摘要。
- Redaction：secret-like 字段只输出字段名、长度、`sha256_12`、来源摘要和脱敏路径。
- 安全边界：不写配置、不创建 snapshot、不启动 MCP server、不执行 Hook 或 Plugin command。
- 原生 UI：macOS SwiftUI 与 Windows WPF 只读 shell，可展示 doctor 诊断、脱敏诊断详情、trace id 复制和基础 retry。
- 测试：Rust core / sidecar tests、diagnostic fixture validator、JSON parse validation。

## 不在 MVP 1 范围内

- 配置写入、diff apply、snapshot、restore。
- MCP 连接测试、工具列表读取和外部 server 启动。
- Hook dry-run、Hook command / URL / MCP tool / prompt / agent 执行。
- Plugin install、update、enable、disable、remove 或任意 plugin command。
- Skill create/edit。
- 云同步、团队策略远程下发、在线审计。

## 验证命令与结果

MVP 1 RC 使用以下命令验证：

```sh
cargo test
cargo run -q -p agentcafe-cli -- doctor --json --schema schemas/diagnostic-report.schema.json > /tmp/agentcafe-doctor.json
node tests/integration/validate-diagnostic-fixtures.mjs
node tests/integration/validate-mvp2-fixtures.mjs
node tests/integration/validate-native-ui-contracts.mjs
find . -path ./.git -prune -o -name '*.json' -print | sort | xargs -n1 jq empty
```

额外安全检查：

```sh
pattern=$(printf '%b' '\\u5c1a\\u65e0\\u4ea7\\u54c1\\u4ee3\\u7801|\\u5c1a\\u672a\\u521d\\u59cb\\u5316|\\u5f53\\u524d\\u5c1a\\u65e0\\u4ea7\\u54c1\\u4ee3\\u7801')
rg -n "$pattern" README.md docs
sensitive_pattern=$(printf '%s|%s|%s|%s|%s|%s' 'AGENTCAFE_FIXTURE_' 'bearer-token-literal' 'private-key-marker' 'tool-payload-body' 'transcript-body' 'shell-output-body')
rg -n "$sensitive_pattern" fixtures/diagnostic/reports
```

RC 验证要求：

- Rust tests 全部通过。
- `agentcafe doctor --json` 输出通过 `DiagnosticReport` schema。
- 诊断 fixtures 全部通过 validator。
- 仓库 JSON 文件均可解析。
- README / docs 不包含过期的初始化阶段状态。
- 诊断报告 fixtures 不包含 fixture sentinel secret、private key marker、prompt text、transcript text or shell output text。

## 已知限制

- 原生 UI 当前是只读 shell；尚未提供签名安装器、自动更新、完整导出报告或平台 reveal path 流程。
- Scanner 仍遵循 MVP 1 静态/白名单能力，部分生态来源只做 metadata 或 detect-only 摘要。
- Runtime version probing 只允许执行 runtime 的版本命令，并受 timeout 限制。
- Large scan 性能已有覆盖基线，但不同机器和真实 workspace 仍需后续采样。
- MVP 2 写入能力尚未启用；所有 draft 写入/测试/备份方法必须继续返回 `feature_not_in_mvp`。
