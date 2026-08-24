// tests/support/mod.rs
//
// A throwaway HTTP server that answers each request with the next scripted
// reply and records what it received, plus fixtures for the registry's typed
// models.
//
// It is a few lines of tokio rather than a mocking framework so the test suite
// stays dependency-free and runs offline.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use registry_client::{ClientConfig, RegistryClient, RetryPolicy};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// One canned HTTP reply.
#[derive(Clone)]
pub struct Reply {
    pub status: u16,
    pub body: String,
    pub headers: Vec<(String, String)>,
    /// Read the request and then never answer, so the client times out.
    pub hang: bool,
}

impl Reply {
    pub fn ok(body: &str) -> Self {
        Self {
            status: 200,
            body: body.to_string(),
            headers: Vec::new(),
            hang: false,
        }
    }

    pub fn error(status: u16, body: &str) -> Self {
        Self {
            status,
            body: body.to_string(),
            headers: Vec::new(),
            hang: false,
        }
    }

    /// A reply that never arrives.
    pub fn hang() -> Self {
        Self {
            status: 200,
            body: String::new(),
            headers: Vec::new(),
            hang: true,
        }
    }

    /// `204 No Content`, as the registry returns for deletes and test sends.
    pub fn no_content() -> Self {
        Self {
            status: 204,
            body: String::new(),
            headers: Vec::new(),
            hang: false,
        }
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
}

/// What the server saw.
#[derive(Clone, Debug)]
pub struct RecordedRequest {
    /// e.g. `GET /api/contracts?limit=2 HTTP/1.1`
    pub line: String,
    /// The full request head, headers included.
    pub head: String,
    pub body: String,
}

impl RecordedRequest {
    /// A request header's value, matched case-insensitively.
    pub fn header(&self, name: &str) -> Option<String> {
        self.head
            .lines()
            .skip(1)
            .filter_map(|line| line.split_once(':'))
            .find(|(key, _)| key.trim().eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim().to_string())
    }

    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }

    /// The path with its query string.
    pub fn target(&self) -> &str {
        self.line.split_whitespace().nth(1).unwrap_or_default()
    }

    pub fn method(&self) -> &str {
        self.line.split_whitespace().next().unwrap_or_default()
    }
}

/// A local stand-in for the registry API.
pub struct FakeRegistry {
    base_url: String,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

impl FakeRegistry {
    pub async fn start(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local port");
        let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&requests);

        tokio::spawn(async move {
            let mut replies = replies.into_iter();
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = Vec::new();
                let mut chunk = [0_u8; 1024];

                // Read the head.
                let head_end = loop {
                    match socket.read(&mut chunk).await {
                        Ok(0) | Err(_) => break None,
                        Ok(read) => buffer.extend_from_slice(&chunk[..read]),
                    }
                    if let Some(index) = find_head_end(&buffer) {
                        break Some(index);
                    }
                };
                let Some(head_end) = head_end else { continue };

                let head = String::from_utf8_lossy(&buffer[..head_end]).to_string();
                let content_length = content_length(&head);

                // Read the body, if the client announced one.
                let mut body = buffer[head_end + 4..].to_vec();
                while body.len() < content_length {
                    match socket.read(&mut chunk).await {
                        Ok(0) | Err(_) => break,
                        Ok(read) => body.extend_from_slice(&chunk[..read]),
                    }
                }

                recorded
                    .lock()
                    .expect("record request")
                    .push(RecordedRequest {
                        line: head.lines().next().unwrap_or_default().to_string(),
                        head: head.clone(),
                        body: String::from_utf8_lossy(&body).to_string(),
                    });

                let reply = replies
                    .next()
                    .unwrap_or_else(|| Reply::error(500, r#"{"code":"NO_REPLY_SCRIPTED"}"#));

                if reply.hang {
                    // Hold this connection open without answering, on its own
                    // task so the accept loop can still serve the retry that
                    // follows the client's timeout.
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        drop(socket);
                    });
                    continue;
                }

                let mut extra = String::new();
                for (name, value) in &reply.headers {
                    extra.push_str(&format!("{name}: {value}\r\n"));
                }
                let payload = format!(
                    "HTTP/1.1 {} {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n{extra}connection: close\r\n\r\n{}",
                    reply.status,
                    reason(reply.status),
                    reply.body.len(),
                    reply.body
                );
                let _ = socket.write_all(payload.as_bytes()).await;
                let _ = socket.flush().await;
            }
        });

        Self { base_url, requests }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// A client pointed at this server, retries disabled so one call is one
    /// request.
    pub fn client(&self) -> RegistryClient {
        RegistryClient::from_config(
            ClientConfig::new(&self.base_url).with_retry_policy(RetryPolicy::none()),
        )
        .expect("build client")
    }

    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().expect("read requests").clone()
    }

    /// Just the request lines, for tests that only care about method and target.
    pub fn request_lines(&self) -> Vec<String> {
        self.requests()
            .into_iter()
            .map(|request| request.line)
            .collect()
    }
}

fn find_head_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn content_length(head: &str) -> usize {
    head.lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .find(|(key, _)| key.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse().ok())
        .unwrap_or(0)
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        418 => "I'm a teapot",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Unknown",
    }
}

// ── Fixtures ──────────────────────────────────────────────────────────────────

/// A `shared::models::Contract` as the API serializes it.
///
/// Every field the model requires is present, so decoding this fixture also
/// checks that the client and the backend model still agree.
pub fn contract_body(index: usize) -> Value {
    json!({
        "id": format!("11111111-1111-1111-1111-{:012}", index),
        "contract_id": format!("CA{:054}", index),
        "wasm_hash": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
        "name": format!("contract-{index}"),
        "slug": format!("contract-{index}"),
        "description": "a contract",
        "publisher_id": "22222222-2222-2222-2222-222222222222",
        "network": "testnet",
        "is_verified": index.is_multiple_of(2),
        "verification_status": "unverified",
        "category": "DeFi",
        "tags": [],
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-02T00:00:00Z",
        "verified_at": null,
        "deployed_at": null,
        "verified_by": null,
        "verification_notes": null,
        "last_accessed_at": null,
        "health_score": 80,
        "is_maintenance": false,
        "logical_id": null,
        "network_configs": null,
        "organization_id": null,
        "visibility": "public",
        "usage_count": 3
    })
}

/// A `PaginatedResponse<Contract>` body with `count` items out of `total`.
pub fn contract_list_body(count: usize, total: i64) -> String {
    contract_list_body_with_cursor(count, total, None)
}

/// The same, with a keyset continuation token.
pub fn contract_list_body_with_cursor(
    count: usize,
    total: i64,
    next_cursor: Option<&str>,
) -> String {
    let items: Vec<Value> = (0..count).map(contract_body).collect();
    let mut body = json!({
        "items": items,
        "total": total,
        "page": 1,
        "per_page": count.max(1),
        "pages": 1
    });
    if let Some(cursor) = next_cursor {
        body["next_cursor"] = json!(cursor);
    }
    body.to_string()
}
