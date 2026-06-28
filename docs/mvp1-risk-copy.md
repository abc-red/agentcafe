# MVP 1 风险文案

本文档固定 MVP 1 Doctor UI 和 CLI 诊断中常见风险的用户可见文案方向。实现可以按平台微调语气，但不得改变安全含义。

| Code | Severity | Title | Message | Recommended action |
| --- | --- | --- | --- | --- |
| `runtime_not_found` | `info` | Runtime not found | The runtime was not found on PATH or known install locations. | Install the runtime or configure its executable path before expecting configuration details. |
| `config_invalid` | `critical` | Configuration cannot be parsed | A configuration source could not be parsed. | Open the configuration in the runtime's own editor and fix the syntax error before relying on merged settings. |
| `permission_denied` | `medium` | Permission required | A configuration source exists but could not be read with current permissions. | Retry after granting operating system permission or inspect the managed policy with an administrator. |
| `secret_like_value` | `high` | Secret-like field detected | A secret-like field is present and will not be displayed. | Move the value to the runtime's supported secret mechanism and keep it out of shared project files. |
| `authorization_header_present` | `high` | Authorization header detected | An authorization header field is present and will not be displayed. | Keep authorization headers in a local secret store or runtime-supported environment mechanism. |
| `primary_api_key_present` | `high` | Claude API key present | Claude primary API key presence was detected without reading the value. | Do not copy this value into shared settings or diagnostic output. |
| `source_untrusted` | `medium` | Untrusted project source | A project configuration source exists but was not trusted for field-level scanning. | Trust the project in the runtime first, or inspect the configuration manually. |
| `hook_executes_command` | `medium` | Hook command declared | A hook declares an external command. MVP 1 reports metadata only and does not execute it. | Review the hook command in the runtime's own configuration before enabling write or test workflows. |
| `scan_timeout` | `medium` | Scan timed out | The scan returned partial results after a timeout. | Retry, narrow the workspace, or inspect the slow source using the trace id. |
| `scan_canceled` | `info` | Scan canceled | The scan was canceled before completion. | Start a new scan when ready. |
| `feature_not_in_mvp` | `info` | Available in MVP 2 | This action is intentionally disabled in MVP 1. | Use the read-only report now; enable write workflows only after MVP 2 safety gates are complete. |

## Copy Rules

- Do not include raw parser snippets, command output, prompt text, transcript text, tool payloads, nonce values, or secret values in `message`.
- Put stable error codes and `trace_id` in detail views, not in large headings.
- Treat `not_tested` MCP connection status as neutral copy, not as failure copy.
- For `critical` and `high`, make the recommended action specific but manual; MVP 1 must not offer automatic fixes.
