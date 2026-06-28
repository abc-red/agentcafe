use crate::{RedactedEvidence, RiskFinding, RuntimeKind, Severity};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::path::{Component, Path};

pub const REDACTION_NOTICE: &str = "Agent Cafe returns field names, counts, lengths, hashes, and redacted paths only. Secrets, prompts, transcripts, tool payloads, shell output, and nonce values are omitted.";

pub fn redact_path(path: &Path, home: &Path) -> String {
    if let Ok(stripped) = path.strip_prefix(home) {
        return format!("~/redacted/{}", stripped.display());
    }

    let mut parts = Vec::new();
    for component in path.components() {
        if let Component::Normal(part) = component {
            parts.push(part.to_string_lossy().to_string());
        }
    }
    if parts.len() >= 2 {
        format!(".../{}/{}", parts[parts.len() - 2], parts[parts.len() - 1])
    } else {
        ".../redacted".to_string()
    }
}

pub fn sha256_12(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)[..12].to_string()
}

pub fn evidence(field: &str, value: &str, source_summary: &str) -> RedactedEvidence {
    RedactedEvidence {
        field: truncate(field, 200),
        length: value.chars().count(),
        sha256_12: sha256_12(value),
        source_summary: truncate(source_summary, 120),
    }
}

pub fn secret_finding(
    runtime: RuntimeKind,
    path: String,
    source: &str,
    field: &str,
    value: &str,
    source_summary: &str,
) -> RiskFinding {
    let severity = if value.contains("PRIVATE KEY") {
        Severity::Critical
    } else {
        Severity::High
    };
    RiskFinding {
        code: "secret_like_value".to_string(),
        severity,
        source: source.to_string(),
        runtime: Some(runtime),
        path: Some(path),
        message: "A secret-like field is present and will not be displayed.".to_string(),
        redacted_evidence: Some(evidence(field, value, source_summary)),
        recommended_action: "Move the value to the runtime's supported secret mechanism and keep it out of shared project files.".to_string(),
    }
}

pub fn scrub_text(input: &str) -> String {
    let mut out = input.to_string();
    for pattern in secret_patterns() {
        out = pattern.replace_all(&out, "[REDACTED]").to_string();
    }
    truncate(out.trim(), 500)
}

pub fn is_secret_like_field(field: &str) -> bool {
    let lowered = field.to_ascii_lowercase();
    lowered.contains("api_key")
        || lowered.contains("apikey")
        || lowered.contains("token")
        || lowered.contains("cookie")
        || lowered.contains("authorization")
        || lowered.contains("x-api-key")
        || lowered.contains("secret")
        || lowered.contains("password")
        || lowered.contains("passwd")
        || lowered.ends_with("_key")
        || lowered.ends_with("_token")
        || lowered.ends_with("_secret")
}

pub fn is_secret_like_value(value: &str) -> bool {
    if value.contains("-----BEGIN") && value.contains("PRIVATE KEY-----") {
        return true;
    }
    secret_patterns().iter().any(|re| re.is_match(value))
}

fn secret_patterns() -> Vec<Regex> {
    vec![
        Regex::new(r"sk-[A-Za-z0-9_-]{20,}").unwrap(),
        Regex::new(r"sk-ant-[A-Za-z0-9_-]{20,}").unwrap(),
        Regex::new(r"gh[pousr]_[A-Za-z0-9_]{20,}").unwrap(),
        Regex::new(r"xox[baprs]-[A-Za-z0-9-]{20,}").unwrap(),
        Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        Regex::new(r"(?i)bearer\s+[A-Za-z0-9._~+/=-]{16,}").unwrap(),
        Regex::new(
            r"(?i)(api[_-]?key|token|cookie|authorization|password|secret)\s*[:=]\s*[^\s,;]+",
        )
        .unwrap(),
        Regex::new(r"-----BEGIN [A-Z ]*PRIVATE KEY-----").unwrap(),
    ]
}

pub fn command_summary(command: &str) -> String {
    let scrubbed = scrub_text(command);
    if scrubbed.len() <= 160 {
        scrubbed
    } else {
        format!("{} ...", &scrubbed[..157])
    }
}

pub fn url_summary(url: &str) -> String {
    let scrubbed = scrub_text(url);
    if let Some((scheme, rest)) = scrubbed.split_once("://") {
        let host = rest.split('/').next().unwrap_or(rest);
        format!("{scheme}://{host}/**")
    } else {
        command_summary(&scrubbed)
    }
}

pub fn truncate(input: &str, max: usize) -> String {
    input.chars().take(max).collect()
}
