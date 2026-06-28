using System.Text.Json.Serialization;

namespace AgentCafe.Windows.Models;

public sealed record DiagnosticReport(
    [property: JsonPropertyName("schema_version")] string SchemaVersion,
    [property: JsonPropertyName("trace_id")] string TraceId,
    [property: JsonPropertyName("generated_at")] DateTimeOffset GeneratedAt,
    [property: JsonPropertyName("runtimes")] IReadOnlyList<RuntimeProfile> Runtimes,
    [property: JsonPropertyName("config_sources")] IReadOnlyList<ConfigSource> ConfigSources,
    [property: JsonPropertyName("plugins")] IReadOnlyList<PluginItem> Plugins,
    [property: JsonPropertyName("skills")] IReadOnlyList<SkillItem> Skills,
    [property: JsonPropertyName("mcp_servers")] IReadOnlyList<McpServerItem> McpServers,
    [property: JsonPropertyName("hooks")] IReadOnlyList<HookItem> Hooks,
    [property: JsonPropertyName("conflicts")] IReadOnlyList<ConflictFinding> Conflicts,
    [property: JsonPropertyName("risk_findings")] IReadOnlyList<RiskFinding> RiskFindings,
    [property: JsonPropertyName("summary")] DiagnosticSummary Summary,
    [property: JsonPropertyName("redaction_notice")] string RedactionNotice
);

public sealed record RuntimeProfile(
    [property: JsonPropertyName("runtime")] string Runtime,
    [property: JsonPropertyName("display_name")] string DisplayName,
    [property: JsonPropertyName("executable_path")] string? ExecutablePath,
    [property: JsonPropertyName("version")] string Version,
    [property: JsonPropertyName("install_source")] string InstallSource,
    [property: JsonPropertyName("path_status")] string PathStatus,
    [property: JsonPropertyName("status")] string Status,
    [property: JsonPropertyName("detected_at")] DateTimeOffset DetectedAt,
    [property: JsonPropertyName("diagnostics")] IReadOnlyList<DiagnosticMessage> Diagnostics
);

public sealed record DiagnosticMessage(
    [property: JsonPropertyName("code")] string Code,
    [property: JsonPropertyName("message")] string Message
);

public sealed record ConfigSource(
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("runtime")] string Runtime,
    [property: JsonPropertyName("scope")] string Scope,
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("priority")] int Priority,
    [property: JsonPropertyName("format")] string Format,
    [property: JsonPropertyName("read_policy")] string ReadPolicy,
    [property: JsonPropertyName("display_policy")] string DisplayPolicy,
    [property: JsonPropertyName("write_policy")] string WritePolicy,
    [property: JsonPropertyName("validation_status")] string ValidationStatus,
    [property: JsonPropertyName("trust_status")] string TrustStatus,
    [property: JsonPropertyName("mvp_stage")] string MvpStage,
    [property: JsonPropertyName("source_reference")] string? SourceReference
);

public sealed record PluginItem(
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("runtime")] string Runtime,
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("version")] string Version,
    [property: JsonPropertyName("scope")] string Scope,
    [property: JsonPropertyName("source")] string Source,
    [property: JsonPropertyName("path")] string? Path,
    [property: JsonPropertyName("enabled")] bool Enabled,
    [property: JsonPropertyName("validation_status")] string ValidationStatus,
    [property: JsonPropertyName("capabilities")] IReadOnlyList<string> Capabilities,
    [property: JsonPropertyName("risk_count")] int RiskCount
);

public sealed record SkillItem(
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("runtime")] string Runtime,
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("description")] string Description,
    [property: JsonPropertyName("scope")] string Scope,
    [property: JsonPropertyName("path")] string? Path,
    [property: JsonPropertyName("validation_status")] string ValidationStatus,
    [property: JsonPropertyName("referenced_resource_count")] int ReferencedResourceCount,
    [property: JsonPropertyName("risk_count")] int RiskCount
);

public sealed record McpServerItem(
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("runtime")] string Runtime,
    [property: JsonPropertyName("scope")] string Scope,
    [property: JsonPropertyName("transport")] string Transport,
    [property: JsonPropertyName("command_or_url_summary")] string CommandOrUrlSummary,
    [property: JsonPropertyName("enabled")] bool Enabled,
    [property: JsonPropertyName("connection_status")] string ConnectionStatus,
    [property: JsonPropertyName("tool_count")] int? ToolCount,
    [property: JsonPropertyName("resource_count")] int? ResourceCount,
    [property: JsonPropertyName("template_count")] int? TemplateCount,
    [property: JsonPropertyName("validation_status")] string ValidationStatus,
    [property: JsonPropertyName("risk_count")] int RiskCount
);

public sealed record HookItem(
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("runtime")] string Runtime,
    [property: JsonPropertyName("scope")] string Scope,
    [property: JsonPropertyName("event")] string Event,
    [property: JsonPropertyName("matcher")] string? Matcher,
    [property: JsonPropertyName("handler_type")] string HandlerType,
    [property: JsonPropertyName("command_summary")] string? CommandSummary,
    [property: JsonPropertyName("enabled")] bool Enabled,
    [property: JsonPropertyName("trust_status")] string TrustStatus,
    [property: JsonPropertyName("validation_status")] string ValidationStatus,
    [property: JsonPropertyName("risk_count")] int RiskCount
);

public sealed record ConflictFinding(
    [property: JsonPropertyName("runtime")] string Runtime,
    [property: JsonPropertyName("key")] string Key,
    [property: JsonPropertyName("winning_source_id")] string WinningSourceId,
    [property: JsonPropertyName("shadowed_source_ids")] IReadOnlyList<string> ShadowedSourceIds,
    [property: JsonPropertyName("explanation")] string Explanation
);

public sealed record RiskFinding(
    [property: JsonPropertyName("code")] string Code,
    [property: JsonPropertyName("severity")] string Severity,
    [property: JsonPropertyName("source")] string Source,
    [property: JsonPropertyName("runtime")] string? Runtime,
    [property: JsonPropertyName("path")] string? Path,
    [property: JsonPropertyName("message")] string Message,
    [property: JsonPropertyName("redacted_evidence")] RedactedEvidence? RedactedEvidence,
    [property: JsonPropertyName("recommended_action")] string RecommendedAction
);

public sealed record RedactedEvidence(
    [property: JsonPropertyName("field")] string Field,
    [property: JsonPropertyName("length")] int Length,
    [property: JsonPropertyName("sha256_12")] string Sha256_12,
    [property: JsonPropertyName("source_summary")] string SourceSummary
);

public sealed record DiagnosticSummary(
    [property: JsonPropertyName("runtime_count")] int RuntimeCount,
    [property: JsonPropertyName("config_source_count")] int ConfigSourceCount,
    [property: JsonPropertyName("plugin_count")] int PluginCount,
    [property: JsonPropertyName("skill_count")] int SkillCount,
    [property: JsonPropertyName("mcp_server_count")] int McpServerCount,
    [property: JsonPropertyName("hook_count")] int HookCount,
    [property: JsonPropertyName("risk_count_by_severity")] RiskCountBySeverity RiskCountBySeverity,
    [property: JsonPropertyName("overall_status")] string OverallStatus,
    [property: JsonPropertyName("truncated")] bool Truncated
);

public sealed record RiskCountBySeverity(
    [property: JsonPropertyName("info")] int Info,
    [property: JsonPropertyName("low")] int Low,
    [property: JsonPropertyName("medium")] int Medium,
    [property: JsonPropertyName("high")] int High,
    [property: JsonPropertyName("critical")] int Critical
);
