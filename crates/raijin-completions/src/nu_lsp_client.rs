/// Nushell LSP client for intelligent completions.
///
/// Spawns `nu --lsp` as a long-lived background subprocess and communicates
/// via JSON-RPC over stdin/stdout. Provides `textDocument/completion` for
/// context-aware Nushell completions (builtins, custom commands, variables).
///
/// This is the official, stable API since Nushell 0.79.
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;

use lsp_types::{CompletionItem, CompletionItemKind};

/// Request sent from the main thread to the LSP worker thread.
pub struct NuCompletionRequest {
    pub text: String,
    pub line: u32,
    pub character: u32,
    pub response_tx: mpsc::Sender<Vec<CompletionItem>>,
}

/// The async LSP client that runs nu --lsp in a background thread.
pub struct NuLspClient {
    request_tx: mpsc::Sender<NuCompletionRequest>,
    _worker: thread::JoinHandle<()>,
}

impl NuLspClient {
    /// Create a new NuLspClient if the nu binary is available.
    /// Returns None if nu is not installed or --lsp is not supported.
    pub fn new(nu_path: &Path) -> Option<Self> {
        let nu_path = nu_path.to_path_buf();

        // Verify nu --lsp is supported by checking --help output
        let check = Command::new(&nu_path)
            .args(["--help"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        let help_text = String::from_utf8_lossy(&check.stdout);
        if !help_text.contains("--lsp") {
            log::warn!("Nushell at {:?} does not support --lsp", nu_path);
            return None;
        }

        let (request_tx, request_rx) = mpsc::channel::<NuCompletionRequest>();

        let worker = thread::Builder::new()
            .name("nu-lsp-worker".into())
            .spawn(move || {
                Self::worker_loop(&nu_path, request_rx);
            })
            .ok()?;

        Some(Self {
            request_tx,
            _worker: worker,
        })
    }

    /// Send a completion request. Returns a receiver for the response.
    /// Non-blocking — the actual LSP communication happens in the worker thread.
    pub fn complete(&self, text: &str, line: u32, character: u32) -> mpsc::Receiver<Vec<CompletionItem>> {
        let (response_tx, response_rx) = mpsc::channel();
        let _ = self.request_tx.send(NuCompletionRequest {
            text: text.to_string(),
            line,
            character,
            response_tx,
        });
        response_rx
    }

    /// The worker loop runs in a background thread.
    /// It manages the nu --lsp subprocess lifecycle and handles requests.
    fn worker_loop(nu_path: &Path, request_rx: mpsc::Receiver<NuCompletionRequest>) {
        let mut lsp_process: Option<LspProcess> = None;

        for request in request_rx.iter() {
            // Lazily start or restart the LSP process
            let alive = lsp_process.as_mut().map_or(false, |l| l.is_alive());
            let lsp = if alive {
                lsp_process.as_mut().unwrap()
            } else {
                lsp_process = LspProcess::spawn(nu_path);
                match lsp_process.as_mut() {
                    Some(lsp) => lsp,
                    None => {
                        let _ = request.response_tx.send(vec![]);
                        continue;
                    }
                }
            };

            // Send completion request and get response
            let items = lsp.complete(&request.text, request.line, request.character);
            let _ = request.response_tx.send(items);
        }

        // Clean shutdown when channel closes
        if let Some(mut lsp) = lsp_process {
            lsp.shutdown();
        }
    }
}

/// Wraps the nu --lsp child process with JSON-RPC communication.
struct LspProcess {
    child: Child,
    stdin: BufWriter<std::process::ChildStdin>,
    stdout: BufReader<std::process::ChildStdout>,
    request_id: u64,
    document_version: i32,
    document_open: bool,
}

const DOCUMENT_URI: &str = "file:///raijin-input.nu";

impl LspProcess {
    fn spawn(nu_path: &Path) -> Option<Self> {
        let mut child = Command::new(nu_path)
            .args(["--lsp"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;

        let stdin = BufWriter::new(child.stdin.take()?);
        let stdout = BufReader::new(child.stdout.take()?);

        let mut lsp = Self {
            child,
            stdin,
            stdout,
            request_id: 0,
            document_version: 0,
            document_open: false,
        };

        // Send initialize request
        lsp.send_initialize();

        Some(lsp)
    }

    fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    fn next_id(&mut self) -> u64 {
        self.request_id += 1;
        self.request_id
    }

    fn send_initialize(&mut self) {
        let id = self.next_id();
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "capabilities": {},
                "rootUri": null,
            }
        });
        self.send_message(&msg);
        // Read initialize response (we don't need to parse it)
        let _ = self.read_message();

        // Send initialized notification
        let initialized = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });
        self.send_message(&initialized);
    }

    fn complete(&mut self, text: &str, line: u32, character: u32) -> Vec<CompletionItem> {
        // Open or update the virtual document
        if !self.document_open {
            self.send_did_open(text);
        } else {
            self.send_did_change(text);
        }

        // Send completion request
        let id = self.next_id();
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": DOCUMENT_URI },
                "position": { "line": line, "character": character },
            }
        });
        self.send_message(&msg);

        // Read response — may need to skip notifications (diagnostics, etc.)
        let mut attempts = 0;
        while attempts < 10 {
            attempts += 1;
            if let Some(response) = self.read_message() {
                // Check if this is our completion response (has matching id)
                if response.get("id").and_then(|v| v.as_u64()) == Some(id) {
                    return self.parse_completion_response(&response);
                }
                // Otherwise it's a notification — skip it
            } else {
                break;
            }
        }

        vec![]
    }

    fn send_did_open(&mut self, text: &str) {
        self.document_version = 1;
        self.document_open = true;
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": DOCUMENT_URI,
                    "languageId": "nushell",
                    "version": self.document_version,
                    "text": text,
                }
            }
        });
        self.send_message(&msg);
    }

    fn send_did_change(&mut self, text: &str) {
        self.document_version += 1;
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": DOCUMENT_URI,
                    "version": self.document_version,
                },
                "contentChanges": [{ "text": text }]
            }
        });
        self.send_message(&msg);
    }

    fn send_message(&mut self, msg: &serde_json::Value) {
        let body = serde_json::to_string(msg).unwrap();
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let _ = self.stdin.write_all(header.as_bytes());
        let _ = self.stdin.write_all(body.as_bytes());
        let _ = self.stdin.flush();
    }

    fn read_message(&mut self) -> Option<serde_json::Value> {
        let mut content_length: usize = 0;
        let mut line = String::new();

        // Read headers until empty line
        loop {
            line.clear();
            if self.stdout.read_line(&mut line).ok()? == 0 {
                return None;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
                content_length = len_str.trim().parse().ok()?;
            }
        }

        if content_length == 0 {
            return None;
        }

        // Read the body
        let mut body = vec![0u8; content_length];
        self.stdout.read_exact(&mut body).ok()?;

        serde_json::from_slice(&body).ok()
    }

    fn parse_completion_response(&self, response: &serde_json::Value) -> Vec<CompletionItem> {
        let result = match response.get("result") {
            Some(r) => r,
            None => return vec![],
        };

        // LSP completion response can be CompletionList or CompletionItem[]
        let items = if let Some(items) = result.as_array() {
            items.clone()
        } else if let Some(items) = result.get("items").and_then(|v| v.as_array()) {
            items.clone()
        } else {
            return vec![];
        };

        items
            .iter()
            .filter_map(|item| {
                let label = item.get("label")?.as_str()?;
                let detail = item
                    .get("detail")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let kind = item
                    .get("kind")
                    .and_then(|v| v.as_u64())
                    .map(|k| match k {
                        1 => CompletionItemKind::TEXT,
                        2 => CompletionItemKind::METHOD,
                        3 => CompletionItemKind::FUNCTION,
                        6 => CompletionItemKind::VARIABLE,
                        9 => CompletionItemKind::MODULE,
                        14 => CompletionItemKind::KEYWORD,
                        _ => CompletionItemKind::TEXT,
                    })
                    .unwrap_or(CompletionItemKind::TEXT);

                Some(CompletionItem {
                    label: label.to_string(),
                    kind: Some(kind),
                    detail,
                    ..Default::default()
                })
            })
            .collect()
    }

    fn shutdown(&mut self) {
        let id = self.next_id();
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "shutdown",
            "params": null
        });
        self.send_message(&msg);
        let _ = self.read_message(); // Read shutdown response

        let exit = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        });
        self.send_message(&exit);

        let _ = self.child.wait();
    }
}

impl Drop for LspProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
