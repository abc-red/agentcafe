# Agent Cafe Windows WPF

Windows native MVP1 UI for Agent Cafe.

## Run

Build the Rust sidecar first:

```powershell
cargo build -p agentcafe-sidecar
```

Then run the WPF app:

```powershell
dotnet run --project apps/windows-wpf/AgentCafe.Windows.csproj
```

For fixture-driven UI checks without launching the sidecar:

```powershell
$env:AGENTCAFE_UI_FIXTURE="fixtures/diagnostic/reports/golden-sample.json"
dotnet run --project apps/windows-wpf/AgentCafe.Windows.csproj
```

The app is read-only. MVP2 draft methods are not called from this UI.
