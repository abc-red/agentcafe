use crate::redaction::{
    REDACTION_NOTICE, command_summary, is_secret_like_field, is_secret_like_value, redact_path,
    scrub_text, secret_finding, url_summary,
};
use crate::*;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub home: PathBuf,
    pub workspace_root: PathBuf,
    pub runtime_path_dir: Option<PathBuf>,
    pub include_untrusted_project_sources: bool,
}

impl ScanOptions {
    pub fn from_env() -> Self {
        let home = env::var_os("AGENTCAFE_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(PathBuf::from))
            .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        let workspace_root = env::var_os("AGENTCAFE_WORKSPACE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| home.clone()));
        let runtime_path_dir = env::var_os("AGENTCAFE_RUNTIME_PATH").map(PathBuf::from);
        Self {
            home,
            workspace_root,
            runtime_path_dir,
            include_untrusted_project_sources: true,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ScanArtifacts {
    pub config_sources: Vec<ConfigSource>,
    pub plugins: Vec<PluginItem>,
    pub skills: Vec<SkillItem>,
    pub mcp_servers: Vec<McpServerItem>,
    pub hooks: Vec<HookItem>,
    pub conflicts: Vec<ConflictFinding>,
    pub risk_findings: Vec<RiskFinding>,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
struct SourceMeta {
    id: String,
    runtime: RuntimeKind,
    scope: Scope,
    priority: u8,
    format: SourceFormat,
    path: PathBuf,
    read_policy: &'static str,
    display_policy: &'static str,
    write_policy: &'static str,
    trust_status: TrustStatus,
    mvp_stage: MvpStage,
    reference: Option<&'static str>,
    source_summary: &'static str,
}

pub fn generate_report(options: &ScanOptions) -> DiagnosticReport {
    let generated_at = now_utc();
    let trace_id = trace_id();
    let runtimes = detect_runtimes(options, &generated_at);
    let mut artifacts = scan_all(options);
    enforce_limits(&mut artifacts);
    let summary = summarize(&runtimes, &artifacts);
    DiagnosticReport {
        schema_version: "agentcafe.diagnostic.v1".to_string(),
        trace_id,
        generated_at,
        runtimes,
        config_sources: artifacts.config_sources,
        plugins: artifacts.plugins,
        skills: artifacts.skills,
        mcp_servers: artifacts.mcp_servers,
        hooks: artifacts.hooks,
        conflicts: artifacts.conflicts,
        risk_findings: artifacts.risk_findings,
        summary,
        redaction_notice: REDACTION_NOTICE.to_string(),
    }
}

pub fn detect_runtimes(options: &ScanOptions, detected_at: &str) -> Vec<RuntimeProfile> {
    [RuntimeKind::Codex, RuntimeKind::Claude]
        .into_iter()
        .map(|runtime| detect_runtime(runtime, options, detected_at))
        .collect()
}

pub fn detect_runtime(
    runtime: RuntimeKind,
    options: &ScanOptions,
    detected_at: &str,
) -> RuntimeProfile {
    let exe = runtime.as_str();
    let candidate = find_executable(exe, options.runtime_path_dir.as_deref());
    match candidate {
        Some(path) => {
            let (version, diagnostics) = match read_version(&path) {
                Ok(version) => (version, Vec::new()),
                Err(code) => (
                    "unknown".to_string(),
                    vec![Diagnostic {
                        code: code.to_string(),
                        message:
                            "Runtime executable was found, but version probing did not complete."
                                .to_string(),
                    }],
                ),
            };
            RuntimeProfile {
                runtime,
                display_name: runtime.display_name().to_string(),
                executable_path: Some(redact_path(&path, &options.home)),
                version,
                install_source: install_source(&path),
                path_status: Status::Available,
                status: Status::Available,
                detected_at: detected_at.to_string(),
                diagnostics,
            }
        }
        None => RuntimeProfile {
            runtime,
            display_name: runtime.display_name().to_string(),
            executable_path: None,
            version: "unknown".to_string(),
            install_source: InstallSource::Unknown,
            path_status: Status::Missing,
            status: Status::Missing,
            detected_at: detected_at.to_string(),
            diagnostics: vec![Diagnostic {
                code: "runtime_not_found".to_string(),
                message: format!("{exe} was not found on PATH or known install locations."),
            }],
        },
    }
}

pub fn scan_all(options: &ScanOptions) -> ScanArtifacts {
    let mut artifacts = ScanArtifacts::default();
    for meta in allowed_sources(options) {
        scan_source(&meta, options, &mut artifacts);
    }
    scan_plugins_and_skills(options, &mut artifacts);
    scan_project_skills(options, &mut artifacts);
    detect_conflicts(&mut artifacts);
    artifacts
}

pub fn validate_source(path: &Path, format: SourceFormat) -> ValidationStatus {
    match fs::read_to_string(path) {
        Ok(text) => match parse_by_format(&text, format) {
            Ok(_) => ValidationStatus::Valid,
            Err(_) => ValidationStatus::Invalid,
        },
        Err(err) if err.kind() == io::ErrorKind::PermissionDenied => ValidationStatus::Skipped,
        Err(_) => ValidationStatus::Unknown,
    }
}

fn allowed_sources(options: &ScanOptions) -> Vec<SourceMeta> {
    let home = &options.home;
    let cwd = &options.workspace_root;
    let mut sources = vec![
        SourceMeta {
            id: "codex-user-config".to_string(),
            runtime: RuntimeKind::Codex,
            scope: Scope::User,
            priority: 40,
            format: SourceFormat::Toml,
            path: home.join(".codex/config.toml"),
            read_policy: "read_whitelisted_fields",
            display_policy: "redacted_summary",
            write_policy: "mvp2_draft_only",
            trust_status: TrustStatus::NotApplicable,
            mvp_stage: MvpStage::Mvp1ReadOnly,
            reference: Some("https://developers.openai.com/codex/config-basic"),
            source_summary: "user config",
        },
        SourceMeta {
            id: "codex-user-hooks".to_string(),
            runtime: RuntimeKind::Codex,
            scope: Scope::User,
            priority: 40,
            format: SourceFormat::Json,
            path: home.join(".codex/hooks.json"),
            read_policy: "read_hook_metadata",
            display_policy: "redacted_summary",
            write_policy: "mvp2_draft_only",
            trust_status: TrustStatus::NotApplicable,
            mvp_stage: MvpStage::Mvp1ReadOnly,
            reference: Some("https://developers.openai.com/codex/hooks"),
            source_summary: "user hooks",
        },
        SourceMeta {
            id: "claude-user-settings".to_string(),
            runtime: RuntimeKind::Claude,
            scope: Scope::User,
            priority: 30,
            format: SourceFormat::Json,
            path: home.join(".claude/settings.json"),
            read_policy: "read_whitelisted_fields",
            display_policy: "redacted_summary",
            write_policy: "mvp2_draft_only",
            trust_status: TrustStatus::NotApplicable,
            mvp_stage: MvpStage::Mvp1ReadOnly,
            reference: Some("https://docs.anthropic.com/en/docs/claude-code/settings"),
            source_summary: "user settings",
        },
        SourceMeta {
            id: "claude-user-state-summary".to_string(),
            runtime: RuntimeKind::Claude,
            scope: Scope::User,
            priority: 50,
            format: SourceFormat::Json,
            path: home.join(".claude.json"),
            read_policy: "read_plugin_mcp_settings_summary",
            display_policy: "redacted_summary",
            write_policy: "mvp2_draft_only",
            trust_status: TrustStatus::NotApplicable,
            mvp_stage: MvpStage::Mvp1ReadOnly,
            reference: Some("https://docs.anthropic.com/en/docs/claude-code/settings"),
            source_summary: "user state summary",
        },
        SourceMeta {
            id: "claude-user-config-presence".to_string(),
            runtime: RuntimeKind::Claude,
            scope: Scope::User,
            priority: 30,
            format: SourceFormat::Json,
            path: home.join(".claude/config.json"),
            read_policy: "detect_presence_only",
            display_policy: "redacted_summary",
            write_policy: "not_writable",
            trust_status: TrustStatus::NotApplicable,
            mvp_stage: MvpStage::Mvp1ReadOnly,
            reference: Some("https://docs.anthropic.com/en/docs/claude-code/settings"),
            source_summary: "claude auth config presence",
        },
        SourceMeta {
            id: "claude-project-settings".to_string(),
            runtime: RuntimeKind::Claude,
            scope: Scope::Project,
            priority: 40,
            format: SourceFormat::Json,
            path: cwd.join(".claude/settings.json"),
            read_policy: "read_whitelisted_fields",
            display_policy: "redacted_summary",
            write_policy: "mvp2_draft_only",
            trust_status: TrustStatus::Trusted,
            mvp_stage: MvpStage::Mvp1ReadOnly,
            reference: Some("https://docs.anthropic.com/en/docs/claude-code/settings"),
            source_summary: "project settings",
        },
        SourceMeta {
            id: "claude-project-local-settings".to_string(),
            runtime: RuntimeKind::Claude,
            scope: Scope::Local,
            priority: 50,
            format: SourceFormat::Json,
            path: cwd.join(".claude/settings.local.json"),
            read_policy: "read_whitelisted_fields",
            display_policy: "redacted_summary",
            write_policy: "mvp2_draft_only",
            trust_status: TrustStatus::Trusted,
            mvp_stage: MvpStage::Mvp1ReadOnly,
            reference: Some("https://docs.anthropic.com/en/docs/claude-code/settings"),
            source_summary: "local settings",
        },
        SourceMeta {
            id: "claude-project-mcp".to_string(),
            runtime: RuntimeKind::Claude,
            scope: Scope::Project,
            priority: 40,
            format: SourceFormat::Json,
            path: cwd.join(".mcp.json"),
            read_policy: "read_mcp_metadata",
            display_policy: "redacted_summary",
            write_policy: "mvp2_draft_only",
            trust_status: TrustStatus::Trusted,
            mvp_stage: MvpStage::Mvp1ReadOnly,
            reference: Some("https://docs.anthropic.com/en/docs/claude-code/mcp"),
            source_summary: "project mcp config",
        },
        SourceMeta {
            id: "codex-project-config".to_string(),
            runtime: RuntimeKind::Codex,
            scope: Scope::Project,
            priority: 50,
            format: SourceFormat::Toml,
            path: cwd.join(".codex/config.toml"),
            read_policy: "read_whitelisted_fields",
            display_policy: "redacted_summary",
            write_policy: "mvp2_draft_only",
            trust_status: TrustStatus::Trusted,
            mvp_stage: MvpStage::Mvp1ReadOnly,
            reference: Some("https://developers.openai.com/codex/config-basic"),
            source_summary: "project config",
        },
        SourceMeta {
            id: "codex-project-hooks".to_string(),
            runtime: RuntimeKind::Codex,
            scope: Scope::Project,
            priority: 50,
            format: SourceFormat::Json,
            path: cwd.join(".codex/hooks.json"),
            read_policy: "read_hook_metadata",
            display_policy: "redacted_summary",
            write_policy: "mvp2_draft_only",
            trust_status: TrustStatus::Trusted,
            mvp_stage: MvpStage::Mvp1ReadOnly,
            reference: Some("https://developers.openai.com/codex/hooks"),
            source_summary: "project hooks",
        },
    ];

    if cfg!(unix) {
        sources.push(SourceMeta {
            id: "codex-system-config".to_string(),
            runtime: RuntimeKind::Codex,
            scope: Scope::System,
            priority: 30,
            format: SourceFormat::Toml,
            path: PathBuf::from("/etc/codex/config.toml"),
            read_policy: "read_whitelisted_fields",
            display_policy: "redacted_summary",
            write_policy: "not_writable",
            trust_status: TrustStatus::NotApplicable,
            mvp_stage: MvpStage::Mvp1ReadOnly,
            reference: Some("https://developers.openai.com/codex/config-basic"),
            source_summary: "system config",
        });
    }
    sources
}

fn scan_source(meta: &SourceMeta, options: &ScanOptions, artifacts: &mut ScanArtifacts) {
    if is_forbidden_path(&meta.path) || !meta.path.exists() {
        return;
    }

    let redacted_path = redact_path(&meta.path, &options.home);
    let mut source = ConfigSource {
        id: meta.id.clone(),
        runtime: meta.runtime,
        scope: meta.scope,
        path: redacted_path.clone(),
        priority: meta.priority,
        format: meta.format,
        read_policy: meta.read_policy.to_string(),
        display_policy: meta.display_policy.to_string(),
        write_policy: meta.write_policy.to_string(),
        validation_status: ValidationStatus::Unknown,
        trust_status: meta.trust_status,
        mvp_stage: meta.mvp_stage,
        source_reference: meta.reference.map(str::to_string),
    };

    if meta.read_policy == "detect_presence_only" {
        source.validation_status = ValidationStatus::Skipped;
        artifacts.config_sources.push(source);
        artifacts.risk_findings.push(RiskFinding {
            code: "redaction_required".to_string(),
            severity: Severity::Info,
            source: "config".to_string(),
            runtime: Some(meta.runtime),
            path: Some(redacted_path),
            message: "Sensitive runtime config was detected but not read.".to_string(),
            redacted_evidence: None,
            recommended_action: "Keep runtime auth and account state out of shared files."
                .to_string(),
        });
        return;
    }

    let text = match fs::read_to_string(&meta.path) {
        Ok(text) => text,
        Err(err) => {
            source.validation_status = if err.kind() == io::ErrorKind::PermissionDenied {
                ValidationStatus::Skipped
            } else {
                ValidationStatus::Unknown
            };
            artifacts.config_sources.push(source);
            artifacts.risk_findings.push(RiskFinding {
                code: if err.kind() == io::ErrorKind::PermissionDenied {
                    "permission_denied"
                } else {
                    "config_read_failed"
                }
                .to_string(),
                severity: Severity::Medium,
                source: "config".to_string(),
                runtime: Some(meta.runtime),
                path: Some(redacted_path),
                message: "A configuration source could not be read.".to_string(),
                redacted_evidence: None,
                recommended_action: "Check file permissions and rerun the diagnostic.".to_string(),
            });
            return;
        }
    };

    let parsed = parse_by_format(&text, meta.format);
    match parsed {
        Ok(value) => {
            source.validation_status = ValidationStatus::Valid;
            scan_value_for_inventory(meta, &value, &redacted_path, artifacts);
            scan_value_for_secrets(
                meta.runtime,
                &value,
                "",
                &redacted_path,
                meta.source_summary,
                artifacts,
            );
        }
        Err(_) => {
            source.validation_status = ValidationStatus::Invalid;
            artifacts.risk_findings.push(RiskFinding {
                code: "config_invalid".to_string(),
                severity: Severity::Medium,
                source: "config".to_string(),
                runtime: Some(meta.runtime),
                path: Some(redacted_path),
                message: "A configuration source is malformed and was not interpreted.".to_string(),
                redacted_evidence: None,
                recommended_action: "Fix the file syntax, then rerun agentcafe doctor --json."
                    .to_string(),
            });
        }
    }
    artifacts.config_sources.push(source);
}

fn scan_value_for_inventory(
    meta: &SourceMeta,
    value: &Value,
    redacted_path: &str,
    artifacts: &mut ScanArtifacts,
) {
    match meta.runtime {
        RuntimeKind::Codex => {
            scan_codex_mcp(meta, value, artifacts);
            scan_hooks(meta, value, redacted_path, artifacts);
            scan_codex_plugins(meta, value, artifacts);
        }
        RuntimeKind::Claude => {
            scan_claude_mcp(meta, value, artifacts);
            scan_hooks(meta, value, redacted_path, artifacts);
            scan_claude_plugins(meta, value, artifacts);
        }
    }
}

fn scan_codex_mcp(meta: &SourceMeta, value: &Value, artifacts: &mut ScanArtifacts) {
    if let Some(servers) = value.get("mcp_servers").and_then(Value::as_object) {
        for (name, server) in servers {
            let transport = transport_from_value(server);
            let summary = command_or_url_summary(server);
            artifacts.mcp_servers.push(McpServerItem {
                id: format!("{}-mcp-{}", meta.id, safe_id(name)),
                runtime: meta.runtime,
                scope: meta.scope,
                transport,
                command_or_url_summary: summary,
                enabled: true,
                connection_status: "not_tested".to_string(),
                tool_count: None,
                resource_count: None,
                template_count: None,
                validation_status: ValidationStatus::Valid,
                risk_count: count_secret_fields(server) as u32,
            });
        }
    }
}

fn scan_claude_mcp(meta: &SourceMeta, value: &Value, artifacts: &mut ScanArtifacts) {
    let servers = value
        .get("mcpServers")
        .or_else(|| value.get("mcp_servers"))
        .or_else(|| value.get("servers"));
    if let Some(map) = servers.and_then(Value::as_object) {
        for (name, server) in map {
            artifacts.mcp_servers.push(McpServerItem {
                id: format!("{}-mcp-{}", meta.id, safe_id(name)),
                runtime: meta.runtime,
                scope: meta.scope,
                transport: transport_from_value(server),
                command_or_url_summary: command_or_url_summary(server),
                enabled: !is_listed_disabled(value, "disabledMcpjsonServers", name),
                connection_status: "not_tested".to_string(),
                tool_count: None,
                resource_count: None,
                template_count: None,
                validation_status: ValidationStatus::Valid,
                risk_count: count_secret_fields(server) as u32,
            });
        }
    }
}

fn scan_hooks(
    meta: &SourceMeta,
    value: &Value,
    _redacted_path: &str,
    artifacts: &mut ScanArtifacts,
) {
    let hooks_value = value.get("hooks").unwrap_or(value);
    if let Some(map) = hooks_value.as_object() {
        for (event, event_value) in map {
            let entries = hook_entries(event_value);
            for (idx, hook) in entries.into_iter().enumerate() {
                let matcher = hook
                    .get("matcher")
                    .and_then(Value::as_str)
                    .map(|s| scrub_text(s));
                let handler_type = hook
                    .get("type")
                    .or_else(|| hook.get("handler_type"))
                    .and_then(Value::as_str)
                    .unwrap_or("command");
                let command = hook
                    .get("command")
                    .or_else(|| hook.get("url"))
                    .or_else(|| hook.get("mcpTool"))
                    .and_then(Value::as_str)
                    .map(command_summary);
                artifacts.hooks.push(HookItem {
                    id: format!("{}-hook-{}-{}", meta.id, safe_id(event), idx),
                    runtime: meta.runtime,
                    scope: meta.scope,
                    event: scrub_text(event),
                    matcher,
                    handler_type: scrub_text(handler_type),
                    command_summary: command,
                    enabled: true,
                    trust_status: meta.trust_status,
                    validation_status: ValidationStatus::Valid,
                    risk_count: count_secret_fields(hook) as u32 + risky_command_count(hook) as u32,
                });
                if risky_command_count(hook) > 0 {
                    artifacts.risk_findings.push(RiskFinding {
                        code: "dangerous_hook_command".to_string(),
                        severity: Severity::Medium,
                        source: "hook".to_string(),
                        runtime: Some(meta.runtime),
                        path: Some(meta.id.clone()),
                        message: "A hook command has elevated static risk and was not executed.".to_string(),
                        redacted_evidence: None,
                        recommended_action: "Review the hook command before enabling or running hook tests in a future MVP.".to_string(),
                    });
                }
            }
        }
    }
}

fn hook_entries(value: &Value) -> Vec<&Value> {
    if let Some(items) = value.as_array() {
        return items.iter().collect();
    }
    if let Some(hooks) = value.get("hooks").and_then(Value::as_array) {
        return hooks.iter().collect();
    }
    vec![value]
}

fn scan_codex_plugins(meta: &SourceMeta, value: &Value, artifacts: &mut ScanArtifacts) {
    if let Some(plugins) = value.get("plugins").and_then(Value::as_object) {
        for (name, plugin) in plugins {
            let enabled = plugin
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            artifacts.plugins.push(PluginItem {
                id: format!("{}-plugin-{}", meta.id, safe_id(name)),
                runtime: meta.runtime,
                name: scrub_text(name),
                version: "unknown".to_string(),
                scope: meta.scope,
                source: "config".to_string(),
                path: None,
                enabled,
                validation_status: ValidationStatus::Partial,
                capabilities: Vec::new(),
                risk_count: count_secret_fields(plugin) as u32,
            });
        }
    }
}

fn scan_claude_plugins(meta: &SourceMeta, value: &Value, artifacts: &mut ScanArtifacts) {
    for field in ["enabledPlugins", "disabledPlugins"] {
        if let Some(items) = value.get(field).and_then(Value::as_array) {
            for item in items.iter().filter_map(Value::as_str) {
                artifacts.plugins.push(PluginItem {
                    id: format!("{}-{}", field, safe_id(item)),
                    runtime: meta.runtime,
                    name: scrub_text(item),
                    version: "unknown".to_string(),
                    scope: meta.scope,
                    source: "settings".to_string(),
                    path: None,
                    enabled: field == "enabledPlugins",
                    validation_status: ValidationStatus::Partial,
                    capabilities: Vec::new(),
                    risk_count: 0,
                });
            }
        }
    }
}

fn scan_plugins_and_skills(options: &ScanOptions, artifacts: &mut ScanArtifacts) {
    let roots = [
        (RuntimeKind::Codex, options.home.join(".codex/plugins")),
        (RuntimeKind::Codex, options.home.join(".codex/skills")),
        (RuntimeKind::Claude, options.home.join(".claude/plugins")),
        (RuntimeKind::Claude, options.home.join(".claude/skills")),
        (RuntimeKind::Claude, options.home.join(".claude/agents")),
    ];
    for (runtime, root) in roots {
        if !root.exists() || is_forbidden_path(&root) {
            continue;
        }
        visit_dir_limited(&root, 4, &mut |path| {
            if path.file_name().and_then(|s| s.to_str()) == Some("plugin.json") {
                scan_plugin_manifest(runtime, path, options, artifacts);
            } else if path.file_name().and_then(|s| s.to_str()) == Some("SKILL.md")
                || path.extension().and_then(|s| s.to_str()) == Some("md")
                    && path.to_string_lossy().contains("/agents/")
            {
                scan_skill_markdown(runtime, path, Scope::User, options, artifacts);
            } else if path.file_name().and_then(|s| s.to_str()) == Some(".mcp.json") {
                let meta = SourceMeta {
                    id: format!(
                        "{}-plugin-mcp-{}",
                        runtime.as_str(),
                        artifacts.mcp_servers.len()
                    ),
                    runtime,
                    scope: Scope::Plugin,
                    priority: 20,
                    format: SourceFormat::Json,
                    path: path.to_path_buf(),
                    read_policy: "read_mcp_metadata",
                    display_policy: "redacted_summary",
                    write_policy: "mvp2_draft_only",
                    trust_status: TrustStatus::Unknown,
                    mvp_stage: MvpStage::Mvp1ReadOnly,
                    reference: None,
                    source_summary: "plugin mcp config",
                };
                scan_source(&meta, options, artifacts);
            }
        });
    }
}

fn scan_project_skills(options: &ScanOptions, artifacts: &mut ScanArtifacts) {
    for root in [
        options.workspace_root.join(".codex/skills"),
        options.workspace_root.join(".claude/skills"),
        options.workspace_root.join(".claude/agents"),
    ] {
        if !root.exists() || is_forbidden_path(&root) {
            continue;
        }
        let runtime = if root.to_string_lossy().contains(".codex") {
            RuntimeKind::Codex
        } else {
            RuntimeKind::Claude
        };
        visit_dir_limited(&root, 3, &mut |path| {
            if path.file_name().and_then(|s| s.to_str()) == Some("SKILL.md")
                || path.extension().and_then(|s| s.to_str()) == Some("md")
            {
                scan_skill_markdown(runtime, path, Scope::Project, options, artifacts);
            }
        });
    }
}

fn scan_plugin_manifest(
    runtime: RuntimeKind,
    path: &Path,
    options: &ScanOptions,
    artifacts: &mut ScanArtifacts,
) {
    let redacted = redact_path(path, &options.home);
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return,
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => {
            artifacts.risk_findings.push(RiskFinding {
                code: "config_invalid".to_string(),
                severity: Severity::Medium,
                source: "plugin".to_string(),
                runtime: Some(runtime),
                path: Some(redacted),
                message: "A plugin manifest is malformed and was not interpreted.".to_string(),
                redacted_evidence: None,
                recommended_action:
                    "Fix the plugin manifest syntax before enabling or updating it.".to_string(),
            });
            return;
        }
    };
    let name = value
        .get("name")
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("unknown-plugin");
    let version = value
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let mut capabilities = Vec::new();
    for key in [
        "skills",
        "mcp",
        "mcp_servers",
        "hooks",
        "commands",
        "agents",
        "apps",
    ] {
        if value.get(key).is_some() {
            capabilities.push(key.to_string());
        }
    }
    artifacts.plugins.push(PluginItem {
        id: format!("{}-plugin-{}", runtime.as_str(), safe_id(name)),
        runtime,
        name: scrub_text(name),
        version: scrub_text(version),
        scope: Scope::Plugin,
        source: "plugin_manifest".to_string(),
        path: Some(redacted.clone()),
        enabled: value
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        validation_status: ValidationStatus::Valid,
        capabilities,
        risk_count: count_secret_fields(&value) as u32,
    });
    scan_value_for_secrets(runtime, &value, "", &redacted, "plugin manifest", artifacts);
}

fn scan_skill_markdown(
    runtime: RuntimeKind,
    path: &Path,
    scope: Scope,
    options: &ScanOptions,
    artifacts: &mut ScanArtifacts,
) {
    let redacted = redact_path(path, &options.home);
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return,
    };
    let (frontmatter, body_start) = markdown_frontmatter(&text);
    let name = frontmatter
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| {
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
        })
        .unwrap_or("skill");
    let description = frontmatter
        .get("description")
        .and_then(Value::as_str)
        .map(scrub_text)
        .unwrap_or_default();
    let referenced_resource_count = count_reference_paths(&text[body_start..]);
    artifacts.skills.push(SkillItem {
        id: format!("{}-skill-{}", runtime.as_str(), safe_id(name)),
        runtime,
        name: scrub_text(name),
        description,
        scope,
        path: Some(redacted.clone()),
        validation_status: ValidationStatus::Valid,
        referenced_resource_count,
        risk_count: count_secret_fields(&frontmatter) as u32,
    });
    scan_value_for_secrets(
        runtime,
        &frontmatter,
        "",
        &redacted,
        "skill frontmatter",
        artifacts,
    );
}

fn markdown_frontmatter(text: &str) -> (Value, usize) {
    if !text.starts_with("---\n") {
        return (Value::Object(Default::default()), 0);
    }
    if let Some(end) = text[4..].find("\n---") {
        let yaml = &text[4..4 + end];
        let value = serde_yaml::from_str(yaml).unwrap_or(Value::Object(Default::default()));
        (value, 4 + end + 4)
    } else {
        (Value::Object(Default::default()), 0)
    }
}

fn count_reference_paths(text: &str) -> u32 {
    text.lines()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("references/")
                || lower.contains("assets/")
                || lower.contains("template")
                || lower.contains(".md")
        })
        .count() as u32
}

fn detect_conflicts(artifacts: &mut ScanArtifacts) {
    let mut keys: HashMap<(RuntimeKind, String), Vec<&ConfigSource>> = HashMap::new();
    for source in &artifacts.config_sources {
        if source.validation_status == ValidationStatus::Valid {
            for key in likely_keys_for_source(source) {
                keys.entry((source.runtime, key)).or_default().push(source);
            }
        }
    }
    for ((runtime, key), mut sources) in keys {
        if sources.len() < 2 {
            continue;
        }
        sources.sort_by_key(|source| source.priority);
        let winning = sources.last().unwrap();
        let shadowed = sources[..sources.len() - 1]
            .iter()
            .map(|s| s.id.clone())
            .collect();
        artifacts.conflicts.push(ConflictFinding {
            runtime,
            key,
            winning_source_id: winning.id.clone(),
            shadowed_source_ids: shadowed,
            explanation: "Higher priority configuration source wins for this whitelisted key; hook entries are merged separately.".to_string(),
        });
    }
}

fn likely_keys_for_source(source: &ConfigSource) -> Vec<String> {
    match (source.runtime, source.format) {
        (RuntimeKind::Codex, SourceFormat::Toml) => {
            vec!["model".to_string(), "approval_policy".to_string()]
        }
        (RuntimeKind::Claude, SourceFormat::Json) => vec!["model".to_string()],
        _ => Vec::new(),
    }
}

fn summarize(runtimes: &[RuntimeProfile], artifacts: &ScanArtifacts) -> Summary {
    let mut counts = RiskCountBySeverity {
        info: 0,
        low: 0,
        medium: 0,
        high: 0,
        critical: 0,
    };
    for finding in &artifacts.risk_findings {
        match finding.severity {
            Severity::Info => counts.info += 1,
            Severity::Low => counts.low += 1,
            Severity::Medium => counts.medium += 1,
            Severity::High => counts.high += 1,
            Severity::Critical => counts.critical += 1,
        }
    }
    let available = runtimes
        .iter()
        .filter(|runtime| runtime.status == Status::Available)
        .count() as u32;
    let overall_status = if counts.critical > 0 || counts.high > 0 {
        Status::Blocked
    } else if artifacts
        .config_sources
        .iter()
        .any(|s| s.validation_status == ValidationStatus::Invalid)
    {
        Status::Invalid
    } else if available > 0 {
        Status::Available
    } else {
        Status::Missing
    };
    Summary {
        runtime_count: available,
        config_source_count: artifacts.config_sources.len() as u32,
        plugin_count: artifacts.plugins.len() as u32,
        skill_count: artifacts.skills.len() as u32,
        mcp_server_count: artifacts.mcp_servers.len() as u32,
        hook_count: artifacts.hooks.len() as u32,
        risk_count_by_severity: counts,
        overall_status,
        truncated: artifacts.truncated,
    }
}

fn enforce_limits(artifacts: &mut ScanArtifacts) {
    let mut truncated = false;
    truncate_vec(&mut artifacts.config_sources, &mut truncated);
    truncate_vec(&mut artifacts.plugins, &mut truncated);
    truncate_vec(&mut artifacts.skills, &mut truncated);
    truncate_vec(&mut artifacts.mcp_servers, &mut truncated);
    truncate_vec(&mut artifacts.hooks, &mut truncated);
    truncate_vec(&mut artifacts.risk_findings, &mut truncated);
    if truncated {
        artifacts.truncated = true;
        artifacts.risk_findings.push(RiskFinding {
            code: "scan_truncated".to_string(),
            severity: Severity::Info,
            source: "scanner".to_string(),
            runtime: None,
            path: None,
            message: "One or more diagnostic collections exceeded the MVP1 item limit.".to_string(),
            redacted_evidence: None,
            recommended_action: "Narrow the scan scope or inspect individual runtime pages."
                .to_string(),
        });
    }
}

fn truncate_vec<T>(items: &mut Vec<T>, truncated: &mut bool) {
    if items.len() > 1000 {
        items.truncate(1000);
        *truncated = true;
    }
}

fn parse_by_format(text: &str, format: SourceFormat) -> Result<Value, String> {
    match format {
        SourceFormat::Toml => toml::from_str::<toml::Value>(text)
            .map_err(|err| err.to_string())
            .and_then(|v| serde_json::to_value(v).map_err(|err| err.to_string())),
        SourceFormat::Json => serde_json::from_str(text).map_err(|err| err.to_string()),
        SourceFormat::Yaml | SourceFormat::Markdown => {
            serde_yaml::from_str(text).map_err(|err| err.to_string())
        }
        _ => Ok(Value::Null),
    }
}

fn scan_value_for_secrets(
    runtime: RuntimeKind,
    value: &Value,
    prefix: &str,
    redacted_path: &str,
    source_summary: &str,
    artifacts: &mut ScanArtifacts,
) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let field = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                if let Some(value) = child.as_str()
                    && (is_secret_like_field(&field) || is_secret_like_value(value))
                {
                    artifacts.risk_findings.push(secret_finding(
                        runtime,
                        redacted_path.to_string(),
                        "config",
                        &field,
                        value,
                        source_summary,
                    ));
                    continue;
                }
                if is_secret_like_field(&field) && !child.is_null() {
                    let serialized = serde_json::to_string(child).unwrap_or_default();
                    artifacts.risk_findings.push(secret_finding(
                        runtime,
                        redacted_path.to_string(),
                        "config",
                        &field,
                        &serialized,
                        source_summary,
                    ));
                    continue;
                }
                scan_value_for_secrets(
                    runtime,
                    child,
                    &field,
                    redacted_path,
                    source_summary,
                    artifacts,
                );
            }
        }
        Value::Array(items) => {
            for (idx, child) in items.iter().enumerate() {
                scan_value_for_secrets(
                    runtime,
                    child,
                    &format!("{prefix}[{idx}]"),
                    redacted_path,
                    source_summary,
                    artifacts,
                );
            }
        }
        Value::String(value) if is_secret_like_value(value) => {
            artifacts.risk_findings.push(secret_finding(
                runtime,
                redacted_path.to_string(),
                "config",
                if prefix.is_empty() { "value" } else { prefix },
                value,
                source_summary,
            ));
        }
        _ => {}
    }
}

fn count_secret_fields(value: &Value) -> usize {
    match value {
        Value::Object(map) => map
            .iter()
            .map(|(key, child)| {
                usize::from(is_secret_like_field(key))
                    + usize::from(child.as_str().is_some_and(is_secret_like_value))
                    + count_secret_fields(child)
            })
            .sum(),
        Value::Array(items) => items.iter().map(count_secret_fields).sum(),
        Value::String(value) => usize::from(is_secret_like_value(value)),
        _ => 0,
    }
}

fn risky_command_count(value: &Value) -> usize {
    value
        .get("command")
        .and_then(Value::as_str)
        .map(|cmd| {
            let lower = cmd.to_ascii_lowercase();
            usize::from(lower.contains("curl ") || lower.contains("rm ") || lower.contains("sudo "))
        })
        .unwrap_or(0)
}

fn command_or_url_summary(value: &Value) -> String {
    if let Some(url) = value
        .get("url")
        .or_else(|| value.get("endpoint"))
        .and_then(Value::as_str)
    {
        return url_summary(url);
    }
    if let Some(command) = value.get("command").and_then(Value::as_str) {
        return command_summary(command);
    }
    if let Some(transport) = value.get("transport").and_then(Value::as_str) {
        return scrub_text(transport);
    }
    "metadata only".to_string()
}

fn transport_from_value(value: &Value) -> Transport {
    let raw = value
        .get("transport")
        .or_else(|| value.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("");
    match raw.to_ascii_lowercase().as_str() {
        "stdio" | "" if value.get("command").is_some() => Transport::Stdio,
        "http" | "https" => Transport::Http,
        "sse" => Transport::Sse,
        "websocket" | "ws" | "wss" => Transport::Websocket,
        _ if value.get("url").is_some() => Transport::Http,
        _ => Transport::Unknown,
    }
}

fn is_listed_disabled(value: &Value, field: &str, name: &str) -> bool {
    value
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(name)))
}

fn is_forbidden_path(path: &Path) -> bool {
    let lowered = path.to_string_lossy().to_ascii_lowercase();
    lowered.contains("/.codex/sessions")
        || lowered.contains("/.claude/projects/")
        || lowered.ends_with(".jsonl")
        || lowered.contains("/.claude/file-history")
        || lowered.contains("/.claude/debug")
}

fn visit_dir_limited(root: &Path, max_depth: usize, visit: &mut impl FnMut(&Path)) {
    fn inner(path: &Path, depth: usize, max_depth: usize, visit: &mut impl FnMut(&Path)) {
        if depth > max_depth || is_forbidden_path(path) {
            return;
        }
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten().take(1200) {
            let path = entry.path();
            if path.is_dir() {
                inner(&path, depth + 1, max_depth, visit);
            } else {
                visit(&path);
            }
        }
    }
    inner(root, 0, max_depth, visit);
}

fn find_executable(name: &str, runtime_path_dir: Option<&Path>) -> Option<PathBuf> {
    if let Some(dir) = runtime_path_dir {
        let candidate = dir.join(name);
        if is_executable_candidate(&candidate) {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let candidate = dir.join(format!("{name}.exe"));
            if is_executable_candidate(&candidate) {
                return Some(candidate);
            }
        }
        return None;
    }
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if is_executable_candidate(&candidate) {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let candidate = dir.join(format!("{name}.exe"));
            if is_executable_candidate(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn is_executable_candidate(path: &Path) -> bool {
    path.is_file()
}

fn read_version(path: &Path) -> Result<String, &'static str> {
    let mut child = Command::new(path)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "runtime_probe_failed")?;
    let Some(status) = child
        .wait_timeout(Duration::from_secs(2))
        .map_err(|_| "runtime_probe_failed")?
    else {
        let _ = child.kill();
        let _ = child.wait();
        return Err("runtime_probe_timeout");
    };
    if !status.success() {
        return Err("runtime_probe_failed");
    }
    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        use std::io::Read;
        let _ = pipe.read_to_string(&mut stdout);
    }
    let text = stdout;
    Ok(scrub_text(text.lines().next().unwrap_or("unknown")))
}

fn install_source(path: &Path) -> InstallSource {
    let text = path.to_string_lossy().to_ascii_lowercase();
    if text.contains("homebrew") || text.contains("/brew/") {
        InstallSource::Brew
    } else if text.contains("npm") || text.contains("node") {
        InstallSource::Npm
    } else {
        InstallSource::Path
    }
}

fn safe_id(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    out.trim_matches('-').chars().take(80).collect()
}

#[allow(dead_code)]
fn object_keys(value: &Value) -> Vec<String> {
    value
        .as_object()
        .map(|map| {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            keys
        })
        .unwrap_or_default()
}
