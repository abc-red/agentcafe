# Agent Cafe macOS SwiftUI

macOS native MVP1 UI for Agent Cafe.

## Run

Build the Rust sidecar first:

```sh
cargo build -p agentcafe-sidecar
```

Then run the SwiftUI app:

```sh
swift run --package-path apps/macos AgentCafeMac
```

For fixture-driven UI checks without launching the sidecar:

```sh
AGENTCAFE_UI_FIXTURE=fixtures/diagnostic/reports/golden-sample.json swift run --package-path apps/macos AgentCafeMac
```

The app is read-only. MVP2 draft methods are not called from this UI.
