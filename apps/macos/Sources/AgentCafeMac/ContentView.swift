import SwiftUI

struct ContentView: View {
    @StateObject private var viewModel = AppViewModel()

    var body: some View {
        NavigationSplitView {
            List(AppSection.allCases, selection: $viewModel.selectedSection) { section in
                Text(section.title)
                    .tag(section)
            }
            .navigationTitle("Agent Café")
        } detail: {
            VStack(spacing: 0) {
                header
                Divider()
                content
            }
            .background(Color(nsColor: .windowBackgroundColor))
        }
        .task {
            await viewModel.refresh()
        }
    }

    private var header: some View {
        HStack(alignment: .center, spacing: 16) {
            VStack(alignment: .leading, spacing: 4) {
                Text(viewModel.selectedSection.title)
                    .font(.title2)
                    .fontWeight(.semibold)
                Text(viewModel.statusMessage)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }
            Spacer()
            Text(viewModel.connectionState)
                .font(.callout)
                .foregroundStyle(.secondary)
                .padding(.horizontal, 10)
                .padding(.vertical, 5)
                .background(.quaternary, in: Capsule())
            Button {
                viewModel.copyTraceId()
            } label: {
                Label("复制 Trace ID", systemImage: "doc.on.doc")
            }
            .disabled(viewModel.report == nil)
            Button {
                Task { await viewModel.refresh() }
            } label: {
                Label(viewModel.report == nil ? "重试" : "刷新诊断", systemImage: "arrow.clockwise")
            }
            .disabled(viewModel.isLoading)
        }
        .padding(20)
    }

    @ViewBuilder
    private var content: some View {
        switch viewModel.selectedSection {
        case .overview:
            OverviewView(viewModel: viewModel)
        case .codex:
            RuntimeDetailView(
                title: "Codex runtime",
                runtimes: viewModel.codexRuntimes,
                configSources: viewModel.report?.configSources.filter { $0.runtime == "codex" } ?? []
            )
        case .claude:
            RuntimeDetailView(
                title: "Claude runtime",
                runtimes: viewModel.claudeRuntimes,
                configSources: viewModel.report?.configSources.filter { $0.runtime == "claude" } ?? []
            )
        case .mcp:
            McpTableView(items: viewModel.report?.mcpServers ?? [])
        case .plugins:
            PluginTableView(items: viewModel.report?.plugins ?? [])
        case .skills:
            SkillTableView(items: viewModel.report?.skills ?? [])
        case .risks:
            RiskTableView(items: viewModel.report?.riskFindings ?? [])
        case .backups:
            BackupTableView(items: viewModel.backupRows)
        case .diagnostics:
            DiagnosticsDetailView(viewModel: viewModel)
        }
    }
}

private struct OverviewView: View {
    @ObservedObject var viewModel: AppViewModel

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 150), spacing: 12)], spacing: 12) {
                    ForEach(viewModel.summaryCards) { card in
                        VStack(alignment: .leading, spacing: 8) {
                            Text(card.label)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Text(card.value)
                                .font(.title)
                                .fontWeight(.semibold)
                                .lineLimit(1)
                                .minimumScaleFactor(0.7)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(14)
                        .background(.background, in: RoundedRectangle(cornerRadius: 8))
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(Color(nsColor: .separatorColor))
                        )
                    }
                }

                VStack(alignment: .leading, spacing: 6) {
                    Text("Last scan")
                        .font(.headline)
                    Text(viewModel.generatedAtDisplay)
                        .foregroundStyle(.secondary)
                    Text(viewModel.report?.redactionNotice ?? "No diagnostic report loaded.")
                        .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(14)
                .background(.background, in: RoundedRectangle(cornerRadius: 8))
                .overlay(RoundedRectangle(cornerRadius: 8).stroke(Color(nsColor: .separatorColor)))

                RuntimeTableView(items: viewModel.report?.runtimes ?? [])
                    .frame(minHeight: 220)
            }
            .padding(20)
        }
    }
}

private struct RuntimeDetailView: View {
    let title: String
    let runtimes: [RuntimeProfile]
    let configSources: [ConfigSource]

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(title)
                .font(.headline)
            RuntimeTableView(items: runtimes)
                .frame(height: 180)
            Text("Config sources")
                .font(.headline)
            ConfigSourceTableView(items: configSources)
        }
        .padding(20)
    }
}

private struct RuntimeTableView: View {
    let items: [RuntimeProfile]

    var body: some View {
        Table(items) {
            TableColumn("Runtime") { Text($0.displayName) }
            TableColumn("Status") { StatusText($0.status) }
            TableColumn("Version") { Text($0.version) }
            TableColumn("Install") { Text($0.installSource) }
            TableColumn("Path") { Text($0.executablePath ?? "missing").foregroundStyle(.secondary) }
        }
        .overlay { EmptyStateView(isEmpty: items.isEmpty, text: "No runtime data.") }
    }
}

private struct ConfigSourceTableView: View {
    let items: [ConfigSource]

    var body: some View {
        Table(items) {
            TableColumn("ID") { Text($0.id) }
            TableColumn("Scope") { Text($0.scope) }
            TableColumn("Validation") { StatusText($0.validationStatus) }
            TableColumn("Format") { Text($0.format) }
            TableColumn("Path") { Text($0.path).foregroundStyle(.secondary) }
        }
        .overlay { EmptyStateView(isEmpty: items.isEmpty, text: "No config sources.") }
    }
}

private struct McpTableView: View {
    let items: [McpServerItem]

    var body: some View {
        Table(items) {
            TableColumn("ID") { Text($0.id) }
            TableColumn("Runtime") { Text($0.runtime) }
            TableColumn("Transport") { Text($0.transport) }
            TableColumn("Connection") { Text($0.connectionStatus == "not_tested" ? "not tested" : $0.connectionStatus) }
            TableColumn("Validation") { StatusText($0.validationStatus) }
            TableColumn("Summary") { Text($0.commandOrUrlSummary).foregroundStyle(.secondary) }
        }
        .padding(20)
        .overlay { EmptyStateView(isEmpty: items.isEmpty, text: "No MCP servers.") }
    }
}

private struct PluginTableView: View {
    let items: [PluginItem]

    var body: some View {
        Table(items) {
            TableColumn("Name") { Text($0.name) }
            TableColumn("Runtime") { Text($0.runtime) }
            TableColumn("Version") { Text($0.version) }
            TableColumn("Enabled") { Text($0.enabled ? "Yes" : "No") }
            TableColumn("Validation") { StatusText($0.validationStatus) }
            TableColumn("Path") { Text($0.path ?? "unknown").foregroundStyle(.secondary) }
        }
        .padding(20)
        .overlay { EmptyStateView(isEmpty: items.isEmpty, text: "No plugins.") }
    }
}

private struct SkillTableView: View {
    let items: [SkillItem]

    var body: some View {
        Table(items) {
            TableColumn("Name") { Text($0.name) }
            TableColumn("Runtime") { Text($0.runtime) }
            TableColumn("Scope") { Text($0.scope) }
            TableColumn("Validation") { StatusText($0.validationStatus) }
            TableColumn("Description") { Text($0.description).foregroundStyle(.secondary) }
        }
        .padding(20)
        .overlay { EmptyStateView(isEmpty: items.isEmpty, text: "No skills.") }
    }
}

private struct RiskTableView: View {
    let items: [RiskFinding]

    var body: some View {
        Table(items) {
            TableColumn("Severity") { Text($0.severity) }
            TableColumn("Code") { Text($0.code) }
            TableColumn("Runtime") { Text($0.runtime ?? "global") }
            TableColumn("Source") { Text($0.source) }
            TableColumn("Message") { Text($0.message).foregroundStyle(.secondary) }
            TableColumn("Recommended action") { Text($0.recommendedAction).foregroundStyle(.secondary) }
        }
        .padding(20)
        .overlay { EmptyStateView(isEmpty: items.isEmpty, text: "No risk findings.") }
    }
}

private struct BackupTableView: View {
    let items: [BackupRow]

    var body: some View {
        Table(items) {
            TableColumn("Capability") { Text($0.name) }
            TableColumn("State") { Text($0.state).foregroundStyle(.secondary) }
            TableColumn("Detail") { Text($0.detail).foregroundStyle(.secondary) }
        }
        .padding(20)
    }
}

private struct DiagnosticsDetailView: View {
    @ObservedObject var viewModel: AppViewModel

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 220), spacing: 12)], spacing: 12) {
                    DetailItem(label: "Schema", value: viewModel.report?.schemaVersion ?? "No report")
                    DetailItem(label: "Trace ID", value: viewModel.report?.traceId ?? "No scan yet")
                    DetailItem(label: "Generated", value: viewModel.generatedAtDisplay)
                }

                VStack(alignment: .leading, spacing: 8) {
                    Text("Redaction notice")
                        .font(.headline)
                    Text(viewModel.report?.redactionNotice ?? "No diagnostic report loaded.")
                        .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(14)
                .background(.background, in: RoundedRectangle(cornerRadius: 8))
                .overlay(RoundedRectangle(cornerRadius: 8).stroke(Color(nsColor: .separatorColor)))

                VStack(alignment: .leading, spacing: 8) {
                    Text("Redacted DiagnosticReport JSON")
                        .font(.headline)
                    ScrollView([.horizontal, .vertical]) {
                        Text(viewModel.diagnosticJson.isEmpty ? "No diagnostic report loaded." : viewModel.diagnosticJson)
                            .font(.system(.body, design: .monospaced))
                            .textSelection(.enabled)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(12)
                    }
                    .frame(minHeight: 360)
                    .background(Color(nsColor: .textBackgroundColor), in: RoundedRectangle(cornerRadius: 8))
                    .overlay(RoundedRectangle(cornerRadius: 8).stroke(Color(nsColor: .separatorColor)))
                }
            }
            .padding(20)
        }
    }
}

private struct DetailItem: View {
    let label: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.body)
                .lineLimit(3)
                .textSelection(.enabled)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(14)
        .background(.background, in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).stroke(Color(nsColor: .separatorColor)))
    }
}

private struct StatusText: View {
    let value: String

    init(_ value: String) {
        self.value = value
    }

    var body: some View {
        Text(value)
            .foregroundStyle(value == "invalid" || value == "blocked" ? .red : .primary)
    }
}

private struct EmptyStateView: View {
    let isEmpty: Bool
    let text: String

    var body: some View {
        if isEmpty {
            Text(text)
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}
