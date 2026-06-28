use agentcafe_core::{
    HandshakeResult, RuntimeKind, ScanOptions, ValidationResult, detect_runtime, detect_runtimes,
    generate_report, now_utc, scan_all, trace_id, validate_source,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const PROTOCOL_VERSION: &str = "1.0";
const SIDECAR_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i32,
    message: String,
    data: Value,
}

fn main() {
    let mut server = Server::default();
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let response = match line {
            Ok(line) => server.handle_line(&line),
            Err(_) => break,
        };
        println!("{}", response);
        let _ = io::stdout().flush();
    }
}

#[derive(Default)]
struct Server {
    handshaken: bool,
}

impl Server {
    fn handle_line(&mut self, line: &str) -> String {
        let request: RpcRequest = match serde_json::from_str(line) {
            Ok(request) => request,
            Err(_) => {
                return error_response(
                    Value::Null,
                    -32700,
                    "Parse error.",
                    "parse_error",
                    "ipc.parse",
                );
            }
        };
        if request.jsonrpc != "2.0" {
            return error_response(
                request.id,
                -32600,
                "Invalid request.",
                "invalid_request",
                "ipc.request",
            );
        }
        if !self.handshaken && request.method != "ipc.handshake" {
            return error_response(
                request.id,
                -32004,
                "Handshake required.",
                "handshake_failed",
                &request.method,
            );
        }
        match self.handle_request(&request.method, request.params) {
            Ok(result) => success_response(request.id, result),
            Err(err) => error_response(
                request.id,
                err.rpc_code,
                &err.message,
                &err.business_code,
                &err.stage,
            ),
        }
    }

    fn handle_request(&mut self, method: &str, params: Value) -> Result<Value, BusinessError> {
        let options = ScanOptions::from_env();
        match method {
            "ipc.handshake" => {
                let protocol = params
                    .get("protocol_version")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if protocol != PROTOCOL_VERSION {
                    return Err(BusinessError::handshake("ipc.handshake"));
                }
                self.handshaken = true;
                serde_json::to_value(HandshakeResult {
                    protocol_version: PROTOCOL_VERSION.to_string(),
                    sidecar_version: SIDECAR_VERSION.to_string(),
                    accepted_capabilities: vec![
                        "runtime.list".to_string(),
                        "runtime.probe".to_string(),
                        "config.scan".to_string(),
                        "config.validate".to_string(),
                        "plugin.list".to_string(),
                        "skill.list".to_string(),
                        "mcp.list".to_string(),
                        "risk.scan".to_string(),
                    ],
                    trace_id: trace_id(),
                })
                .map_err(|_| BusinessError::internal("ipc.handshake"))
            }
            "doctor.run" => serde_json::to_value(generate_report(&options))
                .map_err(|_| BusinessError::internal("doctor.run")),
            "runtime.list" => Ok(json!({
                "runtimes": detect_runtimes(&options, &now_utc()),
                "trace_id": trace_id()
            })),
            "runtime.probe" => {
                let runtime = parse_runtime(params.get("runtime"))?;
                Ok(json!({
                    "runtime": detect_runtime(runtime, &options, &now_utc()),
                    "trace_id": trace_id()
                }))
            }
            "config.scan" => {
                let artifacts = scan_all(&options);
                Ok(json!({
                    "config_sources": artifacts.config_sources,
                    "conflicts": artifacts.conflicts,
                    "trace_id": trace_id()
                }))
            }
            "config.validate" => {
                let path = params
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| BusinessError::invalid_params("config.validate"))?;
                let format = params
                    .get("format")
                    .and_then(Value::as_str)
                    .and_then(parse_format)
                    .ok_or_else(|| BusinessError::invalid_params("config.validate"))?;
                serde_json::to_value(ValidationResult {
                    validation_status: validate_source(&PathBuf::from(path), format),
                    errors: Vec::new(),
                    warnings: Vec::new(),
                    trace_id: trace_id(),
                })
                .map_err(|_| BusinessError::internal("config.validate"))
            }
            "plugin.list" => {
                let artifacts = scan_all(&options);
                Ok(json!({ "plugins": artifacts.plugins, "trace_id": trace_id() }))
            }
            "skill.list" => {
                let artifacts = scan_all(&options);
                Ok(json!({ "skills": artifacts.skills, "trace_id": trace_id() }))
            }
            "mcp.list" => {
                if params
                    .get("test_connections")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    return Err(BusinessError::feature_not_in_mvp("mcp.list"));
                }
                let artifacts = scan_all(&options);
                Ok(json!({ "mcp_servers": artifacts.mcp_servers, "trace_id": trace_id() }))
            }
            "risk.scan" => {
                let artifacts = scan_all(&options);
                Ok(json!({ "risk_findings": artifacts.risk_findings, "trace_id": trace_id() }))
            }
            "config.diff" | "config.apply" | "plugin.inspect" | "plugin.enable"
            | "plugin.disable" | "skill.validate" | "skill.create" | "mcp.test" | "backup.list"
            | "backup.create" | "backup.restore" => Err(BusinessError::feature_not_in_mvp(method)),
            _ => Err(BusinessError {
                rpc_code: -32601,
                business_code: "method_not_found".to_string(),
                message: "Method not found.".to_string(),
                stage: method.to_string(),
            }),
        }
    }
}

#[derive(Debug)]
struct BusinessError {
    rpc_code: i32,
    business_code: String,
    message: String,
    stage: String,
}

impl BusinessError {
    fn handshake(stage: &str) -> Self {
        Self {
            rpc_code: -32004,
            business_code: "handshake_failed".to_string(),
            message: "Handshake failed.".to_string(),
            stage: stage.to_string(),
        }
    }

    fn invalid_params(stage: &str) -> Self {
        Self {
            rpc_code: -32602,
            business_code: "invalid_params".to_string(),
            message: "Invalid params.".to_string(),
            stage: stage.to_string(),
        }
    }

    fn feature_not_in_mvp(stage: &str) -> Self {
        Self {
            rpc_code: -32001,
            business_code: "feature_not_in_mvp".to_string(),
            message: "This method is not available in MVP1.".to_string(),
            stage: stage.to_string(),
        }
    }

    fn internal(stage: &str) -> Self {
        Self {
            rpc_code: -32603,
            business_code: "internal_error".to_string(),
            message: "Internal error.".to_string(),
            stage: stage.to_string(),
        }
    }
}

fn parse_runtime(value: Option<&Value>) -> Result<RuntimeKind, BusinessError> {
    match value.and_then(Value::as_str) {
        Some("codex") => Ok(RuntimeKind::Codex),
        Some("claude") => Ok(RuntimeKind::Claude),
        _ => Err(BusinessError::invalid_params("runtime.probe")),
    }
}

fn parse_format(value: &str) -> Option<agentcafe_core::SourceFormat> {
    match value {
        "toml" => Some(agentcafe_core::SourceFormat::Toml),
        "json" => Some(agentcafe_core::SourceFormat::Json),
        "yaml" => Some(agentcafe_core::SourceFormat::Yaml),
        "markdown" => Some(agentcafe_core::SourceFormat::Markdown),
        "directory" => Some(agentcafe_core::SourceFormat::Directory),
        "command" => Some(agentcafe_core::SourceFormat::Command),
        "unknown" => Some(agentcafe_core::SourceFormat::Unknown),
        _ => None,
    }
}

fn success_response(id: Value, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

fn error_response(id: Value, code: i32, message: &str, business_code: &str, stage: &str) -> String {
    let error = RpcError {
        code,
        message: message.to_string(),
        data: json!({
            "code": business_code,
            "retryable": false,
            "trace_id": trace_id(),
            "stage": stage,
        }),
    };
    json!({ "jsonrpc": "2.0", "id": id, "error": error }).to_string()
}
