use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKind {
    Codex,
    Claude,
}

impl RuntimeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RuntimeKind::Codex => "codex",
            RuntimeKind::Claude => "claude",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            RuntimeKind::Codex => "Codex",
            RuntimeKind::Claude => "Claude Code",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Managed,
    System,
    User,
    Profile,
    Project,
    Local,
    CliOverride,
    Plugin,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Available,
    Missing,
    Disabled,
    Invalid,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceFormat {
    Toml,
    Json,
    Yaml,
    Markdown,
    Directory,
    Command,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Stdio,
    Http,
    Sse,
    Websocket,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Valid,
    Invalid,
    Partial,
    Skipped,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallSource {
    Path,
    Npm,
    Brew,
    Winget,
    AppBundle,
    Manual,
    Managed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustStatus {
    Trusted,
    Untrusted,
    Managed,
    NotApplicable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MvpStage {
    Mvp1ReadOnly,
    Mvp2WriteDraft,
    Mvp3EcosystemDraft,
    V2Draft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactedEvidence {
    pub field: String,
    pub length: usize,
    pub sha256_12: String,
    pub source_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProfile {
    pub runtime: RuntimeKind,
    pub display_name: String,
    pub executable_path: Option<String>,
    pub version: String,
    pub install_source: InstallSource,
    pub path_status: Status,
    pub status: Status,
    pub detected_at: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSource {
    pub id: String,
    pub runtime: RuntimeKind,
    pub scope: Scope,
    pub path: String,
    pub priority: u8,
    pub format: SourceFormat,
    pub read_policy: String,
    pub display_policy: String,
    pub write_policy: String,
    pub validation_status: ValidationStatus,
    pub trust_status: TrustStatus,
    pub mvp_stage: MvpStage,
    pub source_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginItem {
    pub id: String,
    pub runtime: RuntimeKind,
    pub name: String,
    pub version: String,
    pub scope: Scope,
    pub source: String,
    pub path: Option<String>,
    pub enabled: bool,
    pub validation_status: ValidationStatus,
    pub capabilities: Vec<String>,
    pub risk_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillItem {
    pub id: String,
    pub runtime: RuntimeKind,
    pub name: String,
    pub description: String,
    pub scope: Scope,
    pub path: Option<String>,
    pub validation_status: ValidationStatus,
    pub referenced_resource_count: u32,
    pub risk_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerItem {
    pub id: String,
    pub runtime: RuntimeKind,
    pub scope: Scope,
    pub transport: Transport,
    pub command_or_url_summary: String,
    pub enabled: bool,
    pub connection_status: String,
    pub tool_count: Option<u32>,
    pub resource_count: Option<u32>,
    pub template_count: Option<u32>,
    pub validation_status: ValidationStatus,
    pub risk_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookItem {
    pub id: String,
    pub runtime: RuntimeKind,
    pub scope: Scope,
    pub event: String,
    pub matcher: Option<String>,
    pub handler_type: String,
    pub command_summary: Option<String>,
    pub enabled: bool,
    pub trust_status: TrustStatus,
    pub validation_status: ValidationStatus,
    pub risk_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFinding {
    pub code: String,
    pub severity: Severity,
    pub source: String,
    pub runtime: Option<RuntimeKind>,
    pub path: Option<String>,
    pub message: String,
    pub redacted_evidence: Option<RedactedEvidence>,
    pub recommended_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictFinding {
    pub runtime: RuntimeKind,
    pub key: String,
    pub winning_source_id: String,
    pub shadowed_source_ids: Vec<String>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskCountBySeverity {
    pub info: u32,
    pub low: u32,
    pub medium: u32,
    pub high: u32,
    pub critical: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub runtime_count: u32,
    pub config_source_count: u32,
    pub plugin_count: u32,
    pub skill_count: u32,
    pub mcp_server_count: u32,
    pub hook_count: u32,
    pub risk_count_by_severity: RiskCountBySeverity,
    pub overall_status: Status,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub schema_version: String,
    pub trace_id: String,
    pub generated_at: String,
    pub runtimes: Vec<RuntimeProfile>,
    pub config_sources: Vec<ConfigSource>,
    pub plugins: Vec<PluginItem>,
    pub skills: Vec<SkillItem>,
    pub mcp_servers: Vec<McpServerItem>,
    pub hooks: Vec<HookItem>,
    pub conflicts: Vec<ConflictFinding>,
    pub risk_findings: Vec<RiskFinding>,
    pub summary: Summary,
    pub redaction_notice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResult {
    pub protocol_version: String,
    pub sidecar_version: String,
    pub accepted_capabilities: Vec<String>,
    pub trace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub validation_status: ValidationStatus,
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
    pub trace_id: String,
}

pub fn trace_id() -> String {
    format!("trace-{}", uuid::Uuid::new_v4().simple())
}

pub fn now_utc() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn empty_json() -> Value {
    serde_json::json!({})
}
