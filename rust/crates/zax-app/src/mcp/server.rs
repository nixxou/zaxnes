//! The transport of the MCP server: Streamable HTTP (MCP 2025-06-18, JSON responses only) on
//! `127.0.0.1`, hand-written over `std::net` and `serde_json` — no async runtime, no new
//! dependency. One thread accepts, one thread per connection reads the JSON-RPC messages; a tool
//! call becomes a [`ToolCall`] for the main thread, which answers on the call's channel.
//!
//! Only the loopback interface is bound, and a request whose `Origin` header names another host
//! is refused (the DNS-rebinding protection the specification asks for).

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use serde_json::{json, Value};

use super::tools;

/// The protocol versions this server speaks, newest first.
const PROTOCOL_VERSIONS: [&str; 2] = ["2025-06-18", "2025-03-26"];
/// The longest a tool call waits for the main thread (a long exact seek included).
const CALL_TIMEOUT: Duration = Duration::from_secs(300);
/// The largest request body read.
const MAX_BODY: usize = 4 * 1024 * 1024;

/// What a tool answers: its content, and whether it is an error.
#[derive(Debug, Clone, Default)]
pub struct ToolResult {
    /// Text and image parts, in order.
    pub content: Vec<Content>,
    /// Whether the call failed (the text says why).
    pub is_error: bool,
}

impl ToolResult {
    /// A text answer.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![Content::Text(text.into())],
            is_error: false,
        }
    }

    /// A failure, with its reason.
    #[must_use]
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            content: vec![Content::Text(text.into())],
            is_error: true,
        }
    }
}

/// A part of a tool's answer.
#[derive(Debug, Clone)]
pub enum Content {
    /// Text.
    Text(String),
    /// A PNG picture.
    Png(Vec<u8>),
}

/// A tool call for the main thread.
#[derive(Debug)]
pub struct ToolCall {
    /// The tool's name.
    pub name: String,
    /// Its arguments (an object, possibly empty).
    pub arguments: Value,
    /// Where the answer goes.
    pub reply: Sender<ToolResult>,
}

/// The server: the calls waiting for the main thread.
pub struct McpServer {
    calls: Receiver<ToolCall>,
    port: u16,
}

impl McpServer {
    /// Listens on `127.0.0.1:port` (0: a free port).
    ///
    /// # Errors
    /// The port could not be bound.
    pub fn start(port: u16) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        let port = listener.local_addr()?.port();
        let (sender, calls) = channel();
        std::thread::Builder::new()
            .name("zaxnes-mcp".to_owned())
            .spawn(move || {
                for stream in listener.incoming().flatten() {
                    let sender = sender.clone();
                    let _ = std::thread::Builder::new()
                        .name("zaxnes-mcp-connection".to_owned())
                        .spawn(move || serve_connection(stream, &sender));
                }
            })?;
        Ok(Self { calls, port })
    }

    /// The port listened on.
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// The tool calls received since the last call.
    #[must_use]
    pub fn take_calls(&self) -> Vec<ToolCall> {
        self.calls.try_iter().collect()
    }
}

/// One HTTP request.
struct Request {
    method: String,
    path: String,
    origin: Option<String>,
    body: Vec<u8>,
}

/// Serves the requests of one connection until it closes.
fn serve_connection(stream: TcpStream, calls: &Sender<ToolCall>) {
    let Ok(write_half) = stream.try_clone() else {
        return;
    };
    let mut writer = write_half;
    let mut reader = BufReader::new(stream);
    while let Some(request) = read_request(&mut reader) {
        let (status, body) = answer(&request, calls);
        if write_response(&mut writer, status, body.as_deref()).is_err() {
            return;
        }
    }
}

/// The status and JSON body of the answer to `request`.
fn answer(request: &Request, calls: &Sender<ToolCall>) -> (&'static str, Option<String>) {
    if let Some(origin) = &request.origin {
        if !is_local_origin(origin) {
            return ("403 Forbidden", None);
        }
    }
    if request.path != "/mcp" && request.path != "/" {
        return ("404 Not Found", None);
    }
    if request.method != "POST" {
        // No server-initiated stream: GET is not offered (Streamable HTTP allows that).
        return ("405 Method Not Allowed", None);
    }
    let Ok(message) = serde_json::from_slice::<Value>(&request.body) else {
        return (
            "400 Bad Request",
            Some(error_response(&Value::Null, -32700, "parse error").to_string()),
        );
    };
    match message {
        Value::Array(batch) => {
            let answers: Vec<Value> = batch
                .iter()
                .filter_map(|message| handle_message(message, calls))
                .collect();
            if answers.is_empty() {
                ("202 Accepted", None)
            } else {
                ("200 OK", Some(Value::Array(answers).to_string()))
            }
        }
        message => match handle_message(&message, calls) {
            Some(answer) => ("200 OK", Some(answer.to_string())),
            None => ("202 Accepted", None),
        },
    }
}

/// The JSON-RPC answer to one message; none for a notification or a response.
fn handle_message(message: &Value, calls: &Sender<ToolCall>) -> Option<Value> {
    let id = message.get("id")?.clone();
    let method = message.get("method")?.as_str()?;
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => {
            let asked = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL_VERSIONS[0]);
            let version = PROTOCOL_VERSIONS
                .iter()
                .find(|known| **known == asked)
                .unwrap_or(&PROTOCOL_VERSIONS[0]);
            json!({
                "protocolVersion": version,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "zaxnes", "version": env!("CARGO_PKG_VERSION") },
                "instructions": tools::INSTRUCTIONS,
            })
        }
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools::definitions() }),
        "tools/call" => {
            let Some(name) = params.get("name").and_then(Value::as_str) else {
                return Some(error_response(&id, -32602, "a tool name is needed"));
            };
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let result = call_tool(name, arguments, calls);
            tool_result_json(&result)
        }
        _ => return Some(error_response(&id, -32601, "method not found")),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

/// Hands a call to the main thread and waits for its answer.
fn call_tool(name: &str, arguments: Value, calls: &Sender<ToolCall>) -> ToolResult {
    let (reply, answer) = channel();
    let call = ToolCall {
        name: name.to_owned(),
        arguments,
        reply,
    };
    if calls.send(call).is_err() {
        return ToolResult::error("Zaxnes is closing.");
    }
    answer
        .recv_timeout(CALL_TIMEOUT)
        .unwrap_or_else(|_| ToolResult::error("Zaxnes did not answer in time."))
}

/// The `result` of a `tools/call`.
fn tool_result_json(result: &ToolResult) -> Value {
    let content: Vec<Value> = result
        .content
        .iter()
        .map(|part| match part {
            Content::Text(text) => json!({ "type": "text", "text": text }),
            Content::Png(png) => {
                json!({ "type": "image", "data": base64(png), "mimeType": "image/png" })
            }
        })
        .collect();
    json!({ "content": content, "isError": result.is_error })
}

fn error_response(id: &Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Whether an `Origin` header names this machine.
fn is_local_origin(origin: &str) -> bool {
    let authority = origin.split_once("://").map_or(origin, |(_, rest)| rest);
    let authority = authority.split('/').next().unwrap_or("");
    let host = if let Some(bracketed) = authority.strip_prefix('[') {
        bracketed.split(']').next().unwrap_or("")
    } else {
        authority.split(':').next().unwrap_or("")
    };
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

/// Reads one request; none when the connection closed or the request does not read.
fn read_request(reader: &mut impl BufRead) -> Option<Request> {
    let mut line = String::new();
    if reader.read_line(&mut line).ok()? == 0 {
        return None;
    }
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_owned();
    let path = parts.next()?.split('?').next().unwrap_or("/").to_owned();
    let mut length = 0usize;
    let mut origin = None;
    loop {
        line.clear();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let header = line.trim_end();
        if header.is_empty() {
            break;
        }
        let (name, value) = header.split_once(':')?;
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            length = value.parse().ok()?;
        } else if name.eq_ignore_ascii_case("origin") {
            origin = Some(value.to_owned());
        }
    }
    if length > MAX_BODY {
        return None;
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    Some(Request {
        method,
        path,
        origin,
        body,
    })
}

fn write_response(
    stream: &mut impl Write,
    status: &str,
    body: Option<&str>,
) -> std::io::Result<()> {
    let body = body.unwrap_or("");
    let content_type = if body.is_empty() {
        ""
    } else {
        "Content-Type: application/json\r\n"
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\n{content_type}Content-Length: {}\r\nConnection: keep-alive\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()
}

/// Standard base64 with padding, for the pictures.
#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = chunk
            .iter()
            .enumerate()
            .fold(0u32, |value, (index, &byte)| {
                value | u32::from(byte) << (16 - 8 * index)
            });
        for index in 0..4 {
            if index <= chunk.len() {
                text.push(char::from(
                    ALPHABET[((value >> (18 - 6 * index)) & 63) as usize],
                ));
            } else {
                text.push('=');
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_rfc_examples() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn only_local_origins_pass() {
        assert!(is_local_origin("http://localhost:3000"));
        assert!(is_local_origin("http://127.0.0.1"));
        assert!(is_local_origin("http://[::1]:80"));
        assert!(!is_local_origin("https://evil.example"));
        assert!(!is_local_origin("http://localhost.evil.example"));
    }

    #[test]
    fn requests_read_with_their_body() {
        let raw = b"POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: 2\r\nOrigin: http://localhost\r\n\r\n{}";
        let request = read_request(&mut &raw[..]).unwrap_or(Request {
            method: String::new(),
            path: String::new(),
            origin: None,
            body: Vec::new(),
        });
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/mcp");
        assert_eq!(request.body, b"{}");
        assert_eq!(request.origin.as_deref(), Some("http://localhost"));
    }

    #[test]
    fn initialize_and_list_answer_without_the_main_thread() {
        let (calls, _receiver) = channel();
        let init = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"protocolVersion": "2025-03-26"}});
        let answer = handle_message(&init, &calls).unwrap_or_default();
        assert_eq!(answer["result"]["protocolVersion"], "2025-03-26");
        let list = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"});
        let answer = handle_message(&list, &calls).unwrap_or_default();
        assert!(answer["result"]["tools"]
            .as_array()
            .is_some_and(|tools| !tools.is_empty()));
        let note = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
        assert!(handle_message(&note, &calls).is_none());
    }
}
