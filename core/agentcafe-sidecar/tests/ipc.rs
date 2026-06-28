use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn sidecar_requires_handshake_and_guards_mvp2_methods() {
    let bin = env!("CARGO_BIN_EXE_agentcafe-sidecar");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());

    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":"1","method":"runtime.list","params":{}}),
    );
    let first = read(&mut stdout);
    assert_eq!(first["error"]["code"], -32004);
    assert_eq!(first["error"]["data"]["code"], "handshake_failed");

    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":"2","method":"ipc.handshake","params":{"protocol_version":"1.0","ui_name":"test","ui_version":"0.1.0","ui_platform":"test","ui_capabilities":[],"nonce":"not-returned"}}),
    );
    let second = read(&mut stdout);
    assert!(second.get("result").is_some());
    assert!(!second.to_string().contains("not-returned"));

    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":"3","method":"backup.create","params":{}}),
    );
    let third = read(&mut stdout);
    assert_eq!(third["error"]["code"], -32001);
    assert_eq!(third["error"]["data"]["code"], "feature_not_in_mvp");

    drop(stdin);
    let _ = child.wait();
}

fn send(stdin: &mut std::process::ChildStdin, value: Value) {
    writeln!(stdin, "{}", value).unwrap();
    stdin.flush().unwrap();
}

fn read(stdout: &mut std::io::BufReader<std::process::ChildStdout>) -> Value {
    use std::io::BufRead;
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}
