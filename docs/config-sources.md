# Agent Cafe 配置来源

## 目标

本文档定义 Codex助手 / Claude助手 在 v1 中可以扫描、展示和管理的配置来源，以及默认禁止扫描的敏感路径。MVP 1 目标是可信只读诊断，不执行写入、不启动 MCP server、不执行 Hook 或 Plugin 命令。

官方来源更新时间：2026-06-28。

参考来源：

- Codex config basics: https://developers.openai.com/codex/config-basic
- Codex hooks: https://developers.openai.com/codex/hooks
- Codex plugins: https://developers.openai.com/codex/plugins
- Claude Code settings: https://docs.anthropic.com/en/docs/claude-code/settings
- Claude Code hooks: https://docs.anthropic.com/en/docs/claude-code/hooks
- Claude Code MCP: https://docs.anthropic.com/en/docs/claude-code/mcp
- Claude Code plugins: https://docs.anthropic.com/en/docs/claude-code/plugins-reference

## 通用原则

- 默认只读配置白名单。
- 所有路径展示应脱敏。
- 未知字段默认保留，但不展示敏感原文。
- MVP 1 不执行配置写入。
- MVP 2 写入必须走 diff、snapshot、atomic write、re-validate。
- 无法通过官方文档确认、版本漂移明显、或路径可能包含私密状态的来源标记为 `experimental_detect_only`。

## 优先级模型

### Codex

Codex 配置解析顺序按官方文档建模，priority 数字越大表示越高优先级：

| priority | scope | 来源 | MVP 1 行为 |
| ---: | --- | --- | --- |
| 60 | `cli_override` | CLI flags 和 `--config` overrides | 只在 doctor 由当前进程显式传入时显示摘要。 |
| 50 | `project` | 项目 `.codex/config.toml`，从项目 root 到 cwd，最近者优先 | 仅在项目可信时读取白名单字段；不可信时只显示存在性和 `source_untrusted`。 |
| 45 | `profile` | `~/.codex/<profile>.config.toml`，由 `--profile` 选择 | MVP 1 detect-only；未获知 active profile 时不推断。 |
| 40 | `user` | `~/.codex/config.toml` | 读取白名单字段。 |
| 30 | `system` | `/etc/codex/config.toml` on Unix，如存在 | 读取白名单字段；Windows 暂标 `experimental_detect_only`。 |
| 10 | `unknown` | built-in defaults | 不读取，仅在 UI 中作为解释性 fallback。 |

冲突检测必须输出 winning source 和 shadowed sources，并解释 project trust、profile 和 CLI override 对结果的影响。Hook 不按普通 key 覆盖处理；多个 Hook 来源会合并。

### Claude Code

Claude Code scopes 按官方 settings 文档建模：

| priority | scope | 来源 | MVP 1 行为 |
| ---: | --- | --- | --- |
| 60 | `managed` | server-managed settings、MDM / plist / registry、system `managed-settings.json` / `managed-mcp.json` | 只读策略摘要；不展示敏感值。 |
| 55 | `cli_override` | 命令行参数 | 只在 doctor 由当前进程显式传入时显示摘要。 |
| 50 | `local` | `.claude/settings.local.json` 和 `~/.claude.json` per-project MCP/local state | 读取白名单字段；登录态和项目私密 metadata 跳过。 |
| 40 | `project` | `.claude/settings.json`、`.mcp.json`、CLAUDE.md 或 `.claude/CLAUDE.md` 摘要 | 读取配置白名单；CLAUDE.md 只做存在性和路径摘要。 |
| 30 | `user` | `~/.claude/settings.json`、`~/.claude/agents/`、`~/.claude/CLAUDE.md` 摘要 | 读取配置白名单；不读取 prompt 正文。 |

Claude permission rules may merge rather than simple override. MVP 1 只报告来源和 potential conflict，不模拟完整 permission engine。

## Codex 来源矩阵

| path / source | scope | format | 读取 | 展示 | 可写 | MVP 阶段 | 敏感字段策略 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `~/.codex/config.toml` | `user` | `toml` | 是，白名单字段 | 脱敏摘要、来源、冲突 | MVP 2 draft | env secret、provider key、token 只显示字段名/长度/hash。 |
| `~/.codex/<profile>.config.toml` | `profile` | `toml` | detect-only，除非 active profile 明确 | 脱敏摘要 | MVP 2 draft | 同 user config。 |
| `.codex/config.toml` | `project` | `toml` | 仅 trusted project | 脱敏摘要；untrusted 只显示存在性 | MVP 2 draft | 不读取高敏字段值；禁止读取 workspace secret。 |
| nested `.codex/config.toml` | `project` | `toml` | 仅 trusted project；closest wins | 冲突说明 | MVP 2 draft | 同 project config。 |
| `/etc/codex/config.toml` | `system` | `toml` | Unix detect/read whitelist | 脱敏摘要 | 不写 | MVP 1 read-only | 同 user config。 |
| Codex managed requirements / admin policy | `managed` | `toml/json` | detect-only | 策略摘要 | 不写 | `experimental_detect_only` | 不读 auth/cache；只显示 enforced setting names。 |
| `~/.codex/hooks.json` | `user` | `json` | 是，hook metadata | event、matcher、command 摘要、风险 | MVP 2 draft | command args 脱敏；不执行。 |
| inline `[hooks]` in config.toml | matching config scope | `toml` | 是，hook metadata | event、matcher、command 摘要、风险 | MVP 2 draft | 不执行；合并来源要标明。 |
| project `.codex/hooks.json` | `project` | `json` | 仅 trusted project | event、matcher、command 摘要、风险 | MVP 2 draft | 不执行；untrusted 只显示存在性。 |
| plugin manifest / plugin bundle | `plugin` | `json/directory` | 是，manifest 和 metadata | name、version、enabled、capabilities | MVP 2 draft | 不执行 install/update/remove。 |
| plugin `hooks/hooks.json` | `plugin` | `json` | 是，metadata | event、matcher、handler 摘要 | MVP 2 draft | 不执行。 |
| plugin bundled MCP config | `plugin` | `toml/json` | 是，metadata | server name、transport、URL/command 摘要 | MVP 2 draft | env/header secret 只显示存在性。 |
| `SKILL.md` under Codex skill/plugin dirs | `user/plugin/project` | `markdown` | 是，frontmatter 和引用路径摘要 | name、description、资源计数 | MVP 2 draft | 不展示引用资源 secret 或 prompt 样本文本。 |
| `codex --version` | `unknown` | `command` | 是，只运行版本命令 | version、path、install_source | 不写 | MVP 1 read-only | 捕获输出限版本摘要；不记录 shell output 全文。 |

默认禁止读取 Codex 高敏路径：

- `~/.codex/sessions`
- raw rollout JSONL
- logs DB 正文
- goals / memories DB 正文
- auth / session / token / account state 明文
- 完整 transcript
- Codex 私有协议帧

## Claude Code 来源矩阵

| path / source | scope | format | 读取 | 展示 | 可写 | MVP 阶段 | 敏感字段策略 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `~/.claude/settings.json` | `user` | `json` | 是，白名单字段 | settings 摘要、plugins、hooks、权限摘要 | MVP 2 draft | `env` secret 只显示字段名/长度/hash。 |
| `.claude/settings.json` | `project` | `json` | 是，白名单字段 | 项目共享 settings 摘要 | MVP 2 draft | secret-like 值脱敏；共享文件中 secret 记 high risk。 |
| `.claude/settings.local.json` | `local` | `json` | 是，白名单字段 | local override 摘要 | MVP 2 draft | secret-like 值脱敏。 |
| managed settings | `managed` | `json/plist/registry` | detect/read whitelist | enforced policy 摘要 | 不写 | MVP 1 read-only | 不读登录态；只显示 policy key。 |
| `~/.claude.json` | `user/local` | `json` | 只读 plugin / MCP / settings 白名单摘要 | source summary、server names、enabled state | MVP 2 draft | OAuth/session/account/project metadata 跳过；API key 永不展示。 |
| `.mcp.json` | `project` | `json` | 是，MCP metadata | server name、transport、URL/command 摘要 | MVP 2 draft | headers/env secret 只显示存在性。 |
| plugin root `plugin.json` | `plugin` | `json` | 是，manifest | name、version、components、enabled | MVP 2 draft | 不执行插件命令。 |
| plugin root `.mcp.json` | `plugin` | `json` | 是，metadata | MCP server 摘要 | MVP 2 draft | 同 MCP secret policy。 |
| plugin `hooks/hooks.json` | `plugin` | `json` | 是，metadata | event、matcher、handler 摘要 | MVP 2 draft | 不执行 command/http/mcp_tool/prompt/agent handlers。 |
| plugin `skills/` or root `SKILL.md` | `plugin` | `markdown` | 是，frontmatter | name、description、资源计数 | MVP 2 draft | 不展示引用资源 secret。 |
| `~/.claude/config.json` | `user` | `json` | 仅存在性和非敏感字段摘要 | `primaryApiKey` 存在性，不展示值 | 不写 | MVP 1 read-only | `primaryApiKey` 永不读明文、永不缓存。 |
| `~/.claude/stats-cache.json` | `user` | `json` | 聚合计数和模型名摘要 | count/model summary | 不写 | `experimental_detect_only` | 不读取 prompt、路径或会话正文。 |
| `~/.claude/agents/` / `.claude/agents/` | `user/project` | `markdown` | frontmatter 摘要 | name、description、tool restrictions | MVP 2 draft | 不展示 agent prompt 正文。 |
| `CLAUDE.md`, `.claude/CLAUDE.md`, `~/.claude/CLAUDE.md` | `project/user` | `markdown` | 存在性和路径摘要 | loaded instruction source summary | 不写 | `experimental_detect_only` | 不读取或展示正文。 |
| `claude --version` | `unknown` | `command` | 是，只运行版本命令 | version、path、install_source | 不写 | MVP 1 read-only | 捕获输出限版本摘要；不记录 shell output 全文。 |

默认禁止读取 Claude 高敏路径：

- `~/.claude/projects/**/*.jsonl`
- `~/.claude/file-history`
- `~/.claude/debug`
- 完整 transcript
- tool output
- shell output
- attachments 正文
- `~/.claude/config.json.primaryApiKey` 明文
- `~/.claude.json` 中的账号、登录态和项目私密 metadata

## 字段级白名单

MVP 1 scanner 必须按字段白名单读取。未列入白名单的字段只能保留为 `unknown_field_count`、`unknown_sensitive_like_count` 或 `redaction_required` 风险摘要，不得进入 UI、日志、IPC response 或 doctor 报告原文。

### Codex `config.toml`

| 字段路径 | 读取 | 展示 | 用途 |
| --- | --- | --- | --- |
| `model` | 是 | value summary | conflict detection、runtime summary。 |
| `model_reasoning_effort` | 是 | value summary | conflict detection。 |
| `approval_policy` | 是 | value summary | risk summary。 |
| `sandbox_mode` | 是 | value summary | risk summary。 |
| `default_permissions` | 是 | value summary | risk summary。 |
| `features.*` | 是 | feature name + bool | hooks enabled、experimental feature summary。 |
| `mcp_servers.<name>.transport` / `type` | 是 | transport | MCP inventory。 |
| `mcp_servers.<name>.command` | 是 | command summary | MCP inventory、risk scan。 |
| `mcp_servers.<name>.args` | 是 | args count + redacted summary | MCP inventory、risk scan。 |
| `mcp_servers.<name>.url` | 是 | URL origin + path redaction | MCP inventory、risk scan。 |
| `mcp_servers.<name>.headers.*` | 字段名 only | presence + redaction | secret-like risk。 |
| `mcp_servers.<name>.env.*` | 字段名 only | presence + redaction | secret-like risk。 |
| `plugins.<id>.enabled` | 是 | bool | plugin inventory。 |
| `hooks.*.matcher` | 是 | matcher | hook inventory。 |
| `hooks.*.hooks.*.type` | 是 | handler type | hook inventory。 |
| `hooks.*.hooks.*.command` | 是 | command summary | hook risk scan。 |
| `project_root_markers` | 是 | count + names | project source explanation。 |
| `tui.keymap.*` | detect-only | key count | configuration summary only。 |

Codex `hooks.json` 只读取 `event`、`matcher`、`hooks[].type`、`hooks[].command`、`hooks[].timeout`、`hooks[].statusMessage`。`command` 必须经过 command summary redaction。

Codex plugin manifest 只读取 plugin id/name/version/source、enabled state、component paths、declared skills/MCP/hooks/apps summary。安装脚本、commands、credential helpers 只做存在性和风险摘要。

### Claude `settings.json`

| 字段路径 | 读取 | 展示 | 用途 |
| --- | --- | --- | --- |
| `$schema` | 是 | schema URL host | validation hint。 |
| `model` / `fallbackModel` / `effortLevel` | 是 | value summary | runtime summary、conflict summary。 |
| `permissions.allow[]` / `permissions.deny[]` | 是 | count + redacted matcher summary | risk summary；不模拟完整 permission engine。 |
| `hooks.*.matcher` | 是 | matcher | hook inventory。 |
| `hooks.*.hooks.*.type` | 是 | handler type | hook inventory。 |
| `hooks.*.hooks.*.command` | 是 | command summary | hook risk scan。 |
| `hooks.*.hooks.*.url` | 是 | URL origin + path redaction | hook risk scan。 |
| `hooks.*.hooks.*.mcpTool` | 是 | tool name | hook inventory。 |
| `enabledPlugins[]` | 是 | plugin ids | plugin inventory。 |
| `disabledPlugins[]` | 是 | plugin ids | plugin inventory。 |
| `enabledMcpjsonServers[]` | 是 | server ids | MCP inventory。 |
| `disabledMcpjsonServers[]` | 是 | server ids | MCP inventory。 |
| `enableAllProjectMcpServers` | 是 | bool | MCP trust summary。 |
| `env.*` | 字段名 only | presence + redaction | secret-like risk。 |
| `apiKeyHelper` | detect-only | presence | auth risk summary；不 execute。 |
| `disableSkillShellExecution` | 是 | bool | skill risk summary。 |
| `allowedMcpServers` / `deniedMcpServers` | 是 | count + server ids | managed policy summary。 |

Claude `.mcp.json` 和 plugin `.mcp.json` 只读取 server name、transport/type、command、args count、cwd summary、URL origin、timeout、alwaysLoad、headers/env field names。headers/env values 永不展示。

Claude `plugin.json` 只读取 plugin name/version/source、declared component paths、skills、agents、hooks、MCP servers、LSP servers summary。plugin script bodies、prompt bodies、credential helper output 不读取。

Claude `~/.claude.json` 只读取 plugin/MCP/settings 白名单摘要。OAuth session、account、trust metadata、project private state、conversation state、API key 明文必须跳过。

## MCP 来源

可展示：

- server 名称。
- runtime。
- scope。
- transport。
- 命令或 URL 的脱敏摘要。
- 是否启用。
- 连接状态：MVP 1 固定为 `not_tested`，除非配置自身明显 invalid。

不可展示：

- API key。
- Authorization header。
- bearer token。
- cookie。
- env secret 明文。

MVP 1 不执行 `mcp.test`。MVP 2 的 `mcp.test` 必须用户显式触发、设置 timeout、不记录完整 tool payload、失败返回稳定错误码。

## Plugin 来源

可展示：

- manifest。
- 名称、版本、来源。
- 启用状态。
- 包含的 Skills、MCP、Hooks、commands / agents / apps 摘要。
- 风险摘要。

默认不执行插件命令。安装、更新、启用、禁用和删除全部是 MVP 2 draft 行为，执行前必须展示 manifest、capability 和风险提示。

## Skill 来源

可展示：

- `SKILL.md` frontmatter。
- Skill 名称、描述、触发条件摘要。
- 引用资源路径摘要。
- runtime 兼容性提示。

不得默认展示：

- 引用资源中的 secret。
- prompt 正文样本中的敏感内容。
- 工具输出和 shell output。

## Hooks / Rules 来源

可展示：

- hook 名称或生成 id。
- 触发事件。
- matcher。
- handler type。
- command / URL / MCP tool / prompt / agent 摘要。
- 文件写入、网络访问、外部命令风险。

不得自动执行 hook。dry-run 或测试必须由用户显式触发，并设置 timeout。MVP 1 只做静态风险判断。

Codex hooks 可来自 active config layers 的 `hooks.json` 或 inline `[hooks]`，也可来自 enabled plugins。多个来源会合并，不按普通配置 key 覆盖。

Claude hooks 可来自 user/project/local/managed settings、plugin `hooks/hooks.json` 或 inline plugin config，也可来自 skill / agent frontmatter。MVP 1 只扫描 hook metadata。

## 跨平台路径注意事项

Windows：

- 支持 `%USERPROFILE%`、`APPDATA`、路径空格、长路径和文件锁。
- 版本检测应考虑 PATH、npm、winget、手动安装目录。
- Claude managed settings 可能来自 HKLM / HKCU registry 或 `C:\Program Files\ClaudeCode\`。

macOS：

- 支持 `~` 展开、app bundle、Homebrew、npm、权限不足和 Keychain 边界。
- Finder reveal 只展示用户明确选择的路径。
- Claude managed settings 可能来自 plist managed preferences 或 `/Library/Application Support/ClaudeCode/`。

## 资源基准

v1 本机扫描应覆盖：

- 100 plugins。
- 200 skills。
- 50 MCP servers。
- 1k 配置相关文件。

扫描必须可取消，UI 不应阻塞。
