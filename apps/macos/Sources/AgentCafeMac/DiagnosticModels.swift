import Foundation

struct DiagnosticReport: Decodable {
    let schemaVersion: String
    let traceId: String
    let generatedAt: Date
    let runtimes: [RuntimeProfile]
    let configSources: [ConfigSource]
    let plugins: [PluginItem]
    let skills: [SkillItem]
    let mcpServers: [McpServerItem]
    let hooks: [HookItem]
    let conflicts: [ConflictFinding]
    let riskFindings: [RiskFinding]
    let summary: DiagnosticSummary
    let redactionNotice: String

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case traceId = "trace_id"
        case generatedAt = "generated_at"
        case runtimes
        case configSources = "config_sources"
        case plugins
        case skills
        case mcpServers = "mcp_servers"
        case hooks
        case conflicts
        case riskFindings = "risk_findings"
        case summary
        case redactionNotice = "redaction_notice"
    }
}

struct RuntimeProfile: Decodable, Identifiable {
    var id: String { runtime }
    let runtime: String
    let displayName: String
    let executablePath: String?
    let version: String
    let installSource: String
    let pathStatus: String
    let status: String
    let detectedAt: Date
    let diagnostics: [DiagnosticMessage]

    enum CodingKeys: String, CodingKey {
        case runtime
        case displayName = "display_name"
        case executablePath = "executable_path"
        case version
        case installSource = "install_source"
        case pathStatus = "path_status"
        case status
        case detectedAt = "detected_at"
        case diagnostics
    }
}

struct DiagnosticMessage: Decodable, Identifiable {
    var id: String { code }
    let code: String
    let message: String
}

struct ConfigSource: Decodable, Identifiable {
    let id: String
    let runtime: String
    let scope: String
    let path: String
    let priority: Int
    let format: String
    let readPolicy: String
    let displayPolicy: String
    let writePolicy: String
    let validationStatus: String
    let trustStatus: String
    let mvpStage: String
    let sourceReference: String?

    enum CodingKeys: String, CodingKey {
        case id
        case runtime
        case scope
        case path
        case priority
        case format
        case readPolicy = "read_policy"
        case displayPolicy = "display_policy"
        case writePolicy = "write_policy"
        case validationStatus = "validation_status"
        case trustStatus = "trust_status"
        case mvpStage = "mvp_stage"
        case sourceReference = "source_reference"
    }
}

struct PluginItem: Decodable, Identifiable {
    let id: String
    let runtime: String
    let name: String
    let version: String
    let scope: String
    let source: String
    let path: String?
    let enabled: Bool
    let validationStatus: String
    let capabilities: [String]
    let riskCount: Int

    enum CodingKeys: String, CodingKey {
        case id
        case runtime
        case name
        case version
        case scope
        case source
        case path
        case enabled
        case validationStatus = "validation_status"
        case capabilities
        case riskCount = "risk_count"
    }
}

struct SkillItem: Decodable, Identifiable {
    let id: String
    let runtime: String
    let name: String
    let description: String
    let scope: String
    let path: String?
    let validationStatus: String
    let referencedResourceCount: Int
    let riskCount: Int

    enum CodingKeys: String, CodingKey {
        case id
        case runtime
        case name
        case description
        case scope
        case path
        case validationStatus = "validation_status"
        case referencedResourceCount = "referenced_resource_count"
        case riskCount = "risk_count"
    }
}

struct McpServerItem: Decodable, Identifiable {
    let id: String
    let runtime: String
    let scope: String
    let transport: String
    let commandOrUrlSummary: String
    let enabled: Bool
    let connectionStatus: String
    let toolCount: Int?
    let resourceCount: Int?
    let templateCount: Int?
    let validationStatus: String
    let riskCount: Int

    enum CodingKeys: String, CodingKey {
        case id
        case runtime
        case scope
        case transport
        case commandOrUrlSummary = "command_or_url_summary"
        case enabled
        case connectionStatus = "connection_status"
        case toolCount = "tool_count"
        case resourceCount = "resource_count"
        case templateCount = "template_count"
        case validationStatus = "validation_status"
        case riskCount = "risk_count"
    }
}

struct HookItem: Decodable, Identifiable {
    let id: String
    let runtime: String
    let scope: String
    let event: String
    let matcher: String?
    let handlerType: String
    let commandSummary: String?
    let enabled: Bool
    let trustStatus: String
    let validationStatus: String
    let riskCount: Int

    enum CodingKeys: String, CodingKey {
        case id
        case runtime
        case scope
        case event
        case matcher
        case handlerType = "handler_type"
        case commandSummary = "command_summary"
        case enabled
        case trustStatus = "trust_status"
        case validationStatus = "validation_status"
        case riskCount = "risk_count"
    }
}

struct ConflictFinding: Decodable, Identifiable {
    var id: String { "\(runtime)-\(key)" }
    let runtime: String
    let key: String
    let winningSourceId: String
    let shadowedSourceIds: [String]
    let explanation: String

    enum CodingKeys: String, CodingKey {
        case runtime
        case key
        case winningSourceId = "winning_source_id"
        case shadowedSourceIds = "shadowed_source_ids"
        case explanation
    }
}

struct RiskFinding: Decodable, Identifiable {
    var id: String { "\(severity)-\(code)-\(path ?? "global")" }
    let code: String
    let severity: String
    let source: String
    let runtime: String?
    let path: String?
    let message: String
    let redactedEvidence: RedactedEvidence?
    let recommendedAction: String

    enum CodingKeys: String, CodingKey {
        case code
        case severity
        case source
        case runtime
        case path
        case message
        case redactedEvidence = "redacted_evidence"
        case recommendedAction = "recommended_action"
    }
}

struct RedactedEvidence: Decodable {
    let field: String
    let length: Int
    let sha25612: String
    let sourceSummary: String

    enum CodingKeys: String, CodingKey {
        case field
        case length
        case sha25612 = "sha256_12"
        case sourceSummary = "source_summary"
    }
}

struct DiagnosticSummary: Decodable {
    let runtimeCount: Int
    let configSourceCount: Int
    let pluginCount: Int
    let skillCount: Int
    let mcpServerCount: Int
    let hookCount: Int
    let riskCountBySeverity: RiskCountBySeverity
    let overallStatus: String
    let truncated: Bool

    enum CodingKeys: String, CodingKey {
        case runtimeCount = "runtime_count"
        case configSourceCount = "config_source_count"
        case pluginCount = "plugin_count"
        case skillCount = "skill_count"
        case mcpServerCount = "mcp_server_count"
        case hookCount = "hook_count"
        case riskCountBySeverity = "risk_count_by_severity"
        case overallStatus = "overall_status"
        case truncated
    }
}

struct RiskCountBySeverity: Decodable {
    let info: Int
    let low: Int
    let medium: Int
    let high: Int
    let critical: Int

    var total: Int {
        info + low + medium + high + critical
    }
}
