import Foundation

@MainActor
final class AppViewModel: ObservableObject {
    @Published var selectedSection: AppSection = .overview
    @Published private(set) var report: DiagnosticReport?
    @Published private(set) var connectionState = "Not connected"
    @Published private(set) var statusMessage = "Ready to run a read-only diagnostic scan."
    @Published private(set) var isLoading = false

    private let sidecarClient = SidecarClient()

    var generatedAtDisplay: String {
        guard let report else { return "No scan yet" }
        return Self.dateFormatter.string(from: report.generatedAt)
    }

    var summaryCards: [SummaryCard] {
        guard let summary = report?.summary else { return [] }
        return [
            SummaryCard(label: "Overall", value: summary.overallStatus),
            SummaryCard(label: "Runtimes", value: "\(summary.runtimeCount)"),
            SummaryCard(label: "Config sources", value: "\(summary.configSourceCount)"),
            SummaryCard(label: "MCP servers", value: "\(summary.mcpServerCount)"),
            SummaryCard(label: "Plugins", value: "\(summary.pluginCount)"),
            SummaryCard(label: "Skills", value: "\(summary.skillCount)"),
            SummaryCard(label: "Risks", value: "\(summary.riskCountBySeverity.total)"),
            SummaryCard(label: "Truncated", value: summary.truncated ? "Yes" : "No")
        ]
    }

    var codexRuntimes: [RuntimeProfile] {
        report?.runtimes.filter { $0.runtime == "codex" } ?? []
    }

    var claudeRuntimes: [RuntimeProfile] {
        report?.runtimes.filter { $0.runtime == "claude" } ?? []
    }

    var backupRows: [BackupRow] {
        [
            BackupRow(name: "Snapshot list", state: "尚未启用", detail: "MVP2 draft: backup.list returns feature_not_in_mvp."),
            BackupRow(name: "Create snapshot", state: "尚未启用", detail: "MVP2 draft: backup.create is not called by this UI."),
            BackupRow(name: "Restore snapshot", state: "尚未启用", detail: "MVP2 draft: backup.restore is not called by this UI.")
        ]
    }

    func refresh() async {
        guard !isLoading else { return }
        isLoading = true
        connectionState = "Connecting"
        statusMessage = "Starting sidecar and running doctor.run..."

        let result = await sidecarClient.runDoctor()
        if let report = result.report {
            self.report = report
            connectionState = "Connected"
            statusMessage = "Read-only diagnostic loaded. Trace: \(report.traceId)"
        } else {
            connectionState = result.errorCode ?? "Unavailable"
            statusMessage = normalizeError(code: result.errorCode, message: result.errorMessage)
        }

        isLoading = false
    }

    private func normalizeError(code: String?, message: String?) -> String {
        switch code {
        case "feature_not_in_mvp":
            return "尚未启用：此操作属于 MVP2 draft。"
        case "sidecar_missing":
            return "Sidecar missing. Build agentcafe-sidecar or set AGENTCAFE_SIDECAR."
        case "handshake_failed":
            return "Handshake failed. Check protocol version compatibility."
        case "timeout":
            return "Sidecar request timed out."
        case "sidecar_crash":
            return "Sidecar crashed or closed the IPC stream."
        default:
            return message ?? "Unable to load diagnostic report."
        }
    }

    private static let dateFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .medium
        return formatter
    }()
}

enum AppSection: String, CaseIterable, Identifiable {
    case overview
    case codex
    case claude
    case mcp
    case plugins
    case skills
    case risks
    case backups

    var id: String { rawValue }

    var title: String {
        switch self {
        case .overview: "总览"
        case .codex: "Codex助手"
        case .claude: "Claude助手"
        case .mcp: "MCP"
        case .plugins: "Plugins"
        case .skills: "Skills"
        case .risks: "风险"
        case .backups: "备份"
        }
    }
}

struct SummaryCard: Identifiable {
    var id: String { label }
    let label: String
    let value: String
}

struct BackupRow: Identifiable {
    var id: String { name }
    let name: String
    let state: String
    let detail: String
}
