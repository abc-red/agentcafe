use agentcafe_core::{
    ScanOptions, SourceFormat, Status, ValidationStatus, generate_report, validate_source,
};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn no_runtime_returns_two_missing_entries() {
    let fixture = Fixture::new();
    let report = generate_report(&fixture.options());
    assert_eq!(report.runtimes.len(), 2);
    assert!(report.runtimes.iter().all(|r| r.status == Status::Missing));
    assert_eq!(report.summary.runtime_count, 0);
}

#[test]
fn codex_only_claude_only_and_dual_runtime_are_detected() {
    let fixture = Fixture::new();
    fixture.runtime("codex", "codex 0.1.0");
    let report = generate_report(&fixture.options());
    assert_eq!(report.summary.runtime_count, 1);
    assert_eq!(report.runtimes[0].status, Status::Available);
    assert_eq!(report.runtimes[1].status, Status::Missing);

    let fixture = Fixture::new();
    fixture.runtime("claude", "claude 1.2.3");
    let report = generate_report(&fixture.options());
    assert_eq!(report.summary.runtime_count, 1);
    assert_eq!(report.runtimes[0].status, Status::Missing);
    assert_eq!(report.runtimes[1].status, Status::Available);

    let fixture = Fixture::new();
    fixture.runtime("codex", "codex 0.1.0");
    fixture.runtime("claude", "claude 1.2.3");
    let report = generate_report(&fixture.options());
    assert_eq!(report.summary.runtime_count, 2);
}

#[test]
fn malformed_toml_json_and_yaml_do_not_panic() {
    let fixture = Fixture::new();
    fixture.write_home(".codex/config.toml", "model = [");
    fixture.write_home(".claude/settings.json", "{ nope");
    let report = generate_report(&fixture.options());
    assert!(
        report
            .config_sources
            .iter()
            .any(|source| source.validation_status == ValidationStatus::Invalid)
    );
    assert!(
        report
            .risk_findings
            .iter()
            .any(|finding| finding.code == "config_invalid")
    );

    let yaml = fixture.path("bad.yaml");
    fs::write(&yaml, ":\n  - nope").unwrap();
    assert_eq!(
        validate_source(&yaml, SourceFormat::Yaml),
        ValidationStatus::Invalid
    );
}

#[test]
fn secret_values_are_redacted_from_report_json() {
    let fixture = Fixture::new();
    let secret = "sk-test_DO_NOT_LEAK_12345678901234567890";
    fixture.write_home(
        ".codex/config.toml",
        &format!(
            r#"
model = "gpt-test"
[mcp_servers.github]
command = "github-mcp"
[mcp_servers.github.env]
GITHUB_TOKEN = "{secret}"
"#
        ),
    );
    fixture.write_workspace(
        ".claude/settings.json",
        r#"{"env":{"ANTHROPIC_API_KEY":"sk-ant_DO_NOT_LEAK_12345678901234567890"}}"#,
    );
    let report = generate_report(&fixture.options());
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains(secret));
    assert!(!json.contains("sk-ant_DO_NOT_LEAK"));
    assert!(!json.contains("GITHUB_TOKEN ="));
    assert!(
        report
            .risk_findings
            .iter()
            .any(|finding| finding.redacted_evidence.is_some())
    );
}

#[test]
fn permission_denied_and_runtime_probe_failure_are_diagnostic() {
    let fixture = Fixture::new();
    fixture.runtime("codex", "codex 0.1.0");
    let bad_runtime = fixture.temp.path().join("bin").join("claude");
    write_file(&bad_runtime, "#!/bin/sh\nsleep 5\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bad_runtime).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bad_runtime, perms).unwrap();
    }

    let denied = fixture.temp.path().join("home/.claude/settings.json");
    write_file(&denied, r#"{"model":"ok"}"#);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&denied).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&denied, perms).unwrap();
    }

    let report = generate_report(&fixture.options());
    assert!(
        report.runtimes[1]
            .diagnostics
            .iter()
            .any(|d| d.code == "runtime_probe_timeout")
    );
    #[cfg(unix)]
    assert!(
        report
            .risk_findings
            .iter()
            .any(|finding| finding.code == "permission_denied")
            || users_like_root()
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&denied).unwrap().permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&denied, perms).unwrap();
    }
}

#[test]
fn dangerous_hook_commands_are_static_risks_not_executed() {
    let fixture = Fixture::new();
    fixture.write_home(
        ".codex/hooks.json",
        r#"{"PreToolUse":[{"matcher":"Bash","type":"command","command":"curl https://example.invalid/hook"}]}"#,
    );
    let report = generate_report(&fixture.options());
    assert!(
        report
            .risk_findings
            .iter()
            .any(|finding| finding.code == "dangerous_hook_command")
    );
    assert_eq!(report.hooks.len(), 1);
}

#[test]
fn forbidden_prompt_transcript_and_session_paths_are_not_scanned() {
    let fixture = Fixture::new();
    fixture.write_home(
        ".codex/sessions/rollout.jsonl",
        "prompt transcript tool payload shell output",
    );
    fixture.write_home(
        ".claude/projects/project/session.jsonl",
        "prompt transcript tool payload shell output",
    );
    fixture.write_home(".claude/file-history/history.json", "prompt text");
    let report = generate_report(&fixture.options());
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("prompt transcript"));
    assert!(!json.contains("prompt transcript"));
    assert!(report.config_sources.is_empty());
}

#[test]
fn large_scan_is_stable_and_schema_shaped() {
    let fixture = Fixture::new();
    for idx in 0..100 {
        fixture.write_home(
            &format!(".claude/plugins/plugin-{idx}/plugin.json"),
            &format!(r#"{{"name":"plugin-{idx}","version":"1.0.0","skills":[]}}"#),
        );
    }
    for idx in 0..200 {
        fixture.write_home(
            &format!(".claude/skills/skill-{idx}/SKILL.md"),
            &format!("---\nname: skill-{idx}\ndescription: ok\n---\nBody not exported."),
        );
    }
    let mut mcp = String::from("{\"mcpServers\":{");
    for idx in 0..50 {
        if idx > 0 {
            mcp.push(',');
        }
        mcp.push_str(&format!(
            r#""server-{idx}":{{"type":"stdio","command":"server-{idx}"}}"#
        ));
    }
    mcp.push_str("}}");
    fixture.write_workspace(".mcp.json", &mcp);

    let report = generate_report(&fixture.options());
    assert!(report.plugins.len() >= 100);
    assert!(report.skills.len() >= 200);
    assert!(report.mcp_servers.len() >= 50);
    let value = serde_json::to_value(report).unwrap();
    assert!(value.get("schema_version").is_some());
}

#[test]
fn report_matches_schema_without_additional_fields() {
    let fixture = Fixture::new();
    fixture.write_home(".codex/config.toml", "model = \"gpt-test\"\n");
    let report = generate_report(&fixture.options());
    let value = serde_json::to_value(report).unwrap();
    let schema_text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/diagnostic-report.schema.json"),
    )
    .unwrap();
    let schema: Value = serde_json::from_str(&schema_text).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    validator.validate(&value).unwrap();
}

struct Fixture {
    temp: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("home")).unwrap();
        fs::create_dir_all(temp.path().join("workspace")).unwrap();
        fs::create_dir_all(temp.path().join("bin")).unwrap();
        Self { temp }
    }

    fn options(&self) -> ScanOptions {
        ScanOptions {
            home: self.temp.path().join("home"),
            workspace_root: self.temp.path().join("workspace"),
            runtime_path_dir: Some(self.temp.path().join("bin")),
            include_untrusted_project_sources: true,
        }
    }

    fn path(&self, relative: &str) -> std::path::PathBuf {
        self.temp.path().join(relative)
    }

    fn write_home(&self, relative: &str, contents: &str) {
        write_file(&self.temp.path().join("home").join(relative), contents);
    }

    fn write_workspace(&self, relative: &str, contents: &str) {
        write_file(&self.temp.path().join("workspace").join(relative), contents);
    }

    fn runtime(&self, name: &str, version: &str) {
        let path = self.temp.path().join("bin").join(name);
        write_file(
            &path,
            &format!(
                "#!/bin/sh\nprintf '%s\\n' '{}'\n",
                version.replace('\'', "")
            ),
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).unwrap();
        }
    }
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = fs::File::create(path).unwrap();
    file.write_all(contents.as_bytes()).unwrap();
}

#[cfg(unix)]
fn users_like_root() -> bool {
    std::env::var("USER").is_ok_and(|user| user == "root")
}
