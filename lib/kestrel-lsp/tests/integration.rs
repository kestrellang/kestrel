//! End-to-end LSP integration tests over stdio.
//!
//! Each test spawns a fresh `kestrel-lsp` binary, sends JSON-RPC messages
//! over stdin/stdout, and asserts on the responses.

use std::io::{BufRead, BufReader, Read as _, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// Reads LSP messages on a background thread and sends them over a channel.
/// This avoids blocking the test thread on `read_line`.
struct LspClient {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    rx: mpsc::Receiver<Value>,
    next_id: i64,
}

impl LspClient {
    fn spawn() -> Self {
        let bin = cargo_bin("kestrel-lsp");
        let mut child = Command::new(&bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let msg = match read_lsp_message(&mut reader) {
                    Some(m) => m,
                    None => break,
                };
                if tx.send(msg).is_err() {
                    break;
                }
            }
        });

        Self {
            child,
            stdin,
            rx,
            next_id: 1,
        }
    }

    fn send(&mut self, msg: &Value) {
        let body = serde_json::to_string(msg).unwrap();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        self.stdin.flush().unwrap();
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        self.wait_for_response(id)
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({"jsonrpc":"2.0","method":method,"params":params}));
    }

    fn wait_for_response(&self, id: i64) -> Value {
        let deadline = Duration::from_secs(30);
        let start = std::time::Instant::now();
        loop {
            let remaining = deadline.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                panic!("timeout waiting for response id={id}");
            }
            match self.rx.recv_timeout(remaining) {
                Ok(msg) => {
                    if msg.get("id").and_then(|v| v.as_i64()) == Some(id) {
                        return msg;
                    }
                    // Server notification or different response — skip.
                },
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    panic!("timeout waiting for response id={id}");
                },
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    panic!("server closed connection while waiting for id={id}");
                },
            }
        }
    }

    /// Wait for server to settle, then send a barrier request and collect
    /// all notifications that arrived before and shortly after the barrier.
    fn flush_notifications(&mut self) -> Vec<Value> {
        std::thread::sleep(Duration::from_secs(2));
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc":"2.0","id":id,"method":"textDocument/hover","params":{
            "textDocument":{"uri":"file:///tmp/__flush__.ks"},
            "position":{"line":0,"character":0}
        }}));
        let mut notifications = Vec::new();
        let mut got_barrier = false;
        loop {
            let timeout = if got_barrier {
                Duration::from_millis(500)
            } else {
                Duration::from_secs(30)
            };
            match self.rx.recv_timeout(timeout) {
                Ok(msg) => {
                    if msg.get("id").and_then(|v| v.as_i64()) == Some(id) {
                        got_barrier = true;
                        continue;
                    }
                    notifications.push(msg);
                },
                Err(mpsc::RecvTimeoutError::Timeout) if got_barrier => {
                    return notifications;
                },
                Err(_) => {
                    panic!("timeout in flush_notifications");
                },
            }
        }
    }

    fn initialize(&mut self) -> Value {
        let resp = self.request("initialize", json!({
            "processId": std::process::id(),
            "capabilities": {},
            "rootUri": null,
        }));
        self.notify("initialized", json!({}));
        resp
    }

    fn open(&mut self, uri: &str, text: &str) {
        self.notify("textDocument/didOpen", json!({
            "textDocument": {"uri": uri, "languageId": "kestrel", "version": 1, "text": text}
        }));
    }

    fn change(&mut self, uri: &str, version: i32, text: &str) {
        self.notify("textDocument/didChange", json!({
            "textDocument": {"uri": uri, "version": version},
            "contentChanges": [{"text": text}]
        }));
    }

    fn shutdown(mut self) {
        let _ = self.request("shutdown", json!(null));
        self.notify("exit", json!(null));
        std::thread::sleep(Duration::from_millis(200));
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_lsp_message(reader: &mut BufReader<std::process::ChildStdout>) -> Option<Value> {
    let mut line = String::new();
    let mut content_length: usize = 0;
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return None,
            Ok(_) => {},
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(val) = trimmed.strip_prefix("Content-Length:") {
            content_length = val.trim().parse().ok()?;
        }
    }
    if content_length == 0 {
        return None;
    }
    let mut buf = vec![0u8; content_length];
    reader.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

fn cargo_bin(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    let debug = path.join("target/debug").join(name);
    assert!(debug.exists(), "`{name}` not found at {}", debug.display());
    debug
}

fn diagnostics_for(notifications: &[Value], uri: &str) -> Vec<Value> {
    notifications
        .iter()
        .filter(|n| {
            n.get("method").and_then(|m| m.as_str()) == Some("textDocument/publishDiagnostics")
                && n.pointer("/params/uri").and_then(|u| u.as_str()) == Some(uri)
        })
        .flat_map(|n| {
            n.pointer("/params/diagnostics")
                .and_then(|d| d.as_array().cloned())
                .unwrap_or_default()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn initialize_and_shutdown() {
    let mut c = LspClient::spawn();
    let resp = c.initialize();
    assert_eq!(
        resp.pointer("/result/serverInfo/name").and_then(|v| v.as_str()),
        Some("kestrel-lsp"),
    );
    assert!(resp.pointer("/result/capabilities").is_some());
    c.shutdown();
}

#[test]
fn open_invalid_file_does_not_crash() {
    let mut c = LspClient::spawn();
    c.initialize();
    c.open("file:///tmp/test_diag.ks", "module Bad\nstruct Foo {");
    let _ = c.flush_notifications();
    // Server must survive opening a file with syntax errors.
    let resp = c.request("textDocument/documentSymbol", json!({
        "textDocument": {"uri": "file:///tmp/test_diag.ks"}
    }));
    assert!(resp.get("result").is_some(), "server should respond after opening invalid file");
    c.shutdown();
}

#[test]
fn document_symbols() {
    let mut c = LspClient::spawn();
    c.initialize();
    let uri = "file:///tmp/test_sym.ks";
    c.open(uri, "module Sym\nstruct Point { var x: Int64 }\nfunc greet() {}");
    let _ = c.flush_notifications();

    let resp = c.request("textDocument/documentSymbol", json!({"textDocument":{"uri":uri}}));
    let syms = resp.pointer("/result").and_then(|v| v.as_array());
    assert!(syms.is_some(), "should return symbol array");
    let names: Vec<&str> = syms
        .unwrap()
        .iter()
        .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.contains(&"Point"), "missing Point in {names:?}");
    assert!(names.contains(&"greet"), "missing greet in {names:?}");
    c.shutdown();
}

#[test]
fn hover_responds() {
    let mut c = LspClient::spawn();
    c.initialize();
    let uri = "file:///tmp/test_hov.ks";
    c.open(uri, "module Hov\nstruct Foo {}");
    let _ = c.flush_notifications();

    let resp = c.request("textDocument/hover", json!({
        "textDocument": {"uri": uri},
        "position": {"line": 1, "character": 7}
    }));
    assert!(resp.get("result").is_some());
    c.shutdown();
}

#[test]
fn completion_responds() {
    let mut c = LspClient::spawn();
    c.initialize();
    let uri = "file:///tmp/test_comp.ks";
    c.open(uri, "module Comp\nstruct Foo { var x: Int64 }\nfunc f() { let a = Foo(x: 1); a. }");
    let _ = c.flush_notifications();

    let resp = c.request("textDocument/completion", json!({
        "textDocument": {"uri": uri},
        "position": {"line": 2, "character": 32}
    }));
    assert!(resp.get("result").is_some());
    c.shutdown();
}

#[test]
fn rapid_edits_survive() {
    let mut c = LspClient::spawn();
    c.initialize();
    let uri = "file:///tmp/test_rapid.ks";
    c.open(uri, "module Rapid\nstruct A {}");

    for i in 0..10 {
        c.change(uri, i + 2, &format!("module Rapid\nstruct A{i} {{}}"));
    }

    let _ = c.flush_notifications();

    let resp = c.request("textDocument/documentSymbol", json!({"textDocument":{"uri":uri}}));
    assert!(resp.get("result").is_some(), "server alive after rapid edits");
    c.shutdown();
}

#[test]
fn goto_definition_responds() {
    let mut c = LspClient::spawn();
    c.initialize();
    let uri = "file:///tmp/test_goto.ks";
    c.open(uri, "module Goto\nstruct Foo {}\nfunc bar() -> Foo { Foo() }");
    let _ = c.flush_notifications();

    let resp = c.request("textDocument/definition", json!({
        "textDocument": {"uri": uri},
        "position": {"line": 2, "character": 14}
    }));
    assert!(resp.get("result").is_some());
    c.shutdown();
}
