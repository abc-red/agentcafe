# Agent Cafe Customer Alpha Release Plan

Target release: `v0.2.0-alpha.1`

## Scope

The first customer-testable build is a controlled Alpha for trusted macOS and Windows users. It validates read-only local diagnostics, native UI navigation, redacted diagnostic detail display, trace id copying, and basic retry behavior.

This is not a public beta. The Alpha does not include signed installers, notarization, automatic updates, configuration writes, snapshots, restore, MCP connection tests, Hook dry-runs, Plugin commands, or Skill editing.

## Deliverables

- macOS release zip containing the SwiftUI UI binary and Rust sidecar.
- Windows release zip containing the WPF publish output and Rust sidecar.
- SHA256 checksum file for each zip.
- Customer-facing notes that explain the read-only boundary, install/run steps, known limits, and feedback format.

The UI must continue to call only `ipc.handshake` and `doctor.run`. Backup UI remains disabled and labels MVP2 draft behavior as unavailable.

## Customer Feedback Rules

Customers may send:

- Platform and OS version.
- Agent Cafe version.
- `trace_id`.
- Redacted screenshot.
- Steps to reproduce.

Customers must not send full diagnostic JSON by default, API keys, tokens, cookies, prompt text, transcript text, tool payloads, shell output, private key material, or full private paths. If engineering needs more data, request a narrower redacted excerpt tied to the trace id.

## Release Gate

Run before packaging:

```sh
node tests/integration/validate-diagnostic-fixtures.mjs
node tests/integration/validate-mvp2-fixtures.mjs
node tests/integration/validate-native-ui-contracts.mjs
find . -path ./.git -prune -o -name '*.json' -print | sort | xargs -n1 jq empty
cargo test
swift build --package-path apps/macos
dotnet build apps/windows-wpf/AgentCafe.Windows.csproj
git diff --check
```

Run packaging:

```sh
scripts/package-alpha.sh v0.2.0-alpha.1
```

Manual smoke after packaging:

- macOS Apple Silicon: launch, fixture mode, real doctor scan, sidecar missing, refresh/retry, copy trace id, diagnostic detail.
- Windows 11: launch, fixture mode, real doctor scan, sidecar missing, refresh/retry, copy trace id, diagnostic detail.
- Confirm no configuration write, snapshot directory, MCP server start, Hook execution, or Plugin command execution occurs.

## Known Limits

- Windows 10 smoke is optional for the first controlled Alpha unless a customer requires it.
- macOS package is not notarized in this Alpha track.
- Windows package is not an installer or MSIX in this Alpha track.
- MVP2 write contract is frozen, but implementation remains disabled.
