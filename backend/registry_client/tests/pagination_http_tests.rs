// tests/pagination_http_tests.rs
//
// End-to-end pagination over real HTTP: the client walks pages served by a
// throwaway local server that mimics the registry's two search endpoints —
// `GET /api/search` (PostgreSQL, `contracts` + `next_cursor`) and
// `GET /api/v1/contracts/search` (Elasticsearch, `results` + `total`/`offset`).
//
// The server is a few lines of tokio rather than a mocking framework so these
// tests stay dependency-free and run offline.

use std::sync::{Arc, Mutex};

use registry_client::{
    ContractSearchRequest, PageCursor, PageLimits, PaginationMode, RegistryClient, StopReason,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// One canned HTTP reply.
#[derive(Clone)]
struct Reply {
    status: u16,
    body: String,
}

impl Reply {
    fn ok(body: &str) -> Self {
        Self {
            status: 200,
            body: body.to_string(),
        }
    }

    fn error(status: u16, body: &str) -> Self {
        Self {
            status,
            body: body.to_string(),
        }
    }
}

/// A local server that answers each request with the next canned reply and
/// records the request lines it saw.
struct FakeRegistry {
    base_url: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl FakeRegistry {
    async fn start(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local port");
        let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&requests);

        tokio::spawn(async move {
            let mut replies = replies.into_iter();
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut head = Vec::new();
                let mut chunk = [0_u8; 1024];
                loop {
                    match socket.read(&mut chunk).await {
                        Ok(0) | Err(_) => break,
                        Ok(read) => head.extend_from_slice(&chunk[..read]),
                    }
                    if head.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }

                let text = String::from_utf8_lossy(&head).to_string();
                recorded
                    .lock()
                    .expect("record request")
                    .push(text.lines().next().unwrap_or_default().to_string());

                let reply = replies
                    .next()
                    .unwrap_or_else(|| Reply::error(500, r#"{"code":"NO_REPLY_SCRIPTED"}"#));
                let payload = format!(
                    "HTTP/1.1 {} {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
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

    fn client(&self) -> RegistryClient {
        RegistryClient::new(&self.base_url).expect("build client")
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().expect("read requests").clone()
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

fn hit(name: &str) -> String {
    format!(
        r#"{{"id":"11111111-1111-1111-1111-11111111111{n}","contract_id":"CA{name}","name":"{name}","description":null,"category":"DeFi","network":"testnet","is_verified":true,"is_deprecated":false,"deprecation_status":"active","relevance_score":0.5}}"#,
        n = name.len(),
        name = name
    )
}

// ── Cursor pagination (PostgreSQL, `next_cursor`) ─────────────────────────────

#[tokio::test]
async fn cursor_walk_follows_next_cursor_to_the_end() {
    let registry = FakeRegistry::start(vec![
        Reply::ok(&format!(
            r#"{{"contracts":[{},{}],"total":3,"took_ms":2,"next_cursor":"tok-2"}}"#,
            hit("aa"),
            hit("bb")
        )),
        Reply::ok(&format!(
            r#"{{"contracts":[{}],"total":3,"took_ms":1}}"#,
            hit("cc")
        )),
    ])
    .await;

    let mut walk = registry
        .client()
        .search_paginator(
            ContractSearchRequest::cursor("swap").with_networks(["testnet"]),
            PageLimits::default().with_page_size(2),
        )
        .expect("paginator");

    let collected = walk.collect_all().await.expect("walk should succeed");

    let names: Vec<String> = collected.items.iter().map(|hit| hit.name.clone()).collect();
    assert_eq!(names, vec!["aa", "bb", "cc"], "server order is preserved");
    assert_eq!(collected.total, Some(3), "server total is preserved");
    assert_eq!(collected.pages_fetched, 2);
    assert_eq!(collected.stop_reason, StopReason::Exhausted);

    let requests = registry.requests();
    assert!(
        requests[0].contains("/api/search?"),
        "cursor mode uses the full-text endpoint: {}",
        requests[0]
    );
    assert!(
        requests[0].contains("cursor=") && !requests[0].contains("cursor=tok"),
        "the first page opens a cursor walk with an empty cursor: {}",
        requests[0]
    );
    assert!(
        requests[1].contains("cursor=tok-2"),
        "the second page echoes the server's token verbatim: {}",
        requests[1]
    );
    assert!(
        !requests[1].contains("offset="),
        "a cursor walk never sends an offset: {}",
        requests[1]
    );
    assert!(
        requests[1].contains("networks=testnet"),
        "filters persist across pages"
    );
}

#[tokio::test]
async fn a_rejected_cursor_surfaces_as_a_typed_error() {
    let registry = FakeRegistry::start(vec![Reply::error(
        400,
        r#"{"code":"INVALID_CURSOR","message":"The provided pagination cursor is invalid"}"#,
    )])
    .await;

    let mut walk = registry
        .client()
        .search_paginator(
            ContractSearchRequest::cursor("swap")
                .with_cursor(Some("not-a-real-cursor".to_string())),
            PageLimits::default().with_page_size(2),
        )
        .expect("paginator");

    let err = walk.next_page().await.expect_err("400 must surface");

    assert!(err.is_invalid_cursor(), "unexpected error: {err}");
    assert_eq!(err.status(), Some(400));
    assert!(err.to_string().contains("INVALID_CURSOR"));
    // The malformed token was forwarded untouched — the client never decodes one.
    assert!(registry.requests()[0].contains("cursor=not-a-real-cursor"));
}

#[tokio::test]
async fn a_repeated_cursor_stops_the_walk_with_an_error() {
    let page = format!(
        r#"{{"contracts":[{}],"total":9,"next_cursor":"stuck"}}"#,
        hit("aa")
    );
    let registry = FakeRegistry::start(vec![Reply::ok(&page), Reply::ok(&page)]).await;

    let mut walk = registry
        .client()
        .search_paginator(
            ContractSearchRequest::cursor("swap"),
            PageLimits::default().with_page_size(1),
        )
        .expect("paginator");

    assert!(walk.next_page().await.expect("first page").is_some());
    let err = walk
        .next_page()
        .await
        .expect_err("a repeated cursor must not loop");

    assert!(
        err.to_string().contains("repeated a pagination cursor"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn empty_pages_with_a_continuation_token_cannot_loop_forever() {
    let empty_with_token = r#"{"contracts":[],"total":9,"next_cursor":"tok-a"}"#;
    let registry = FakeRegistry::start(vec![
        Reply::ok(r#"{"contracts":[],"total":9,"next_cursor":"tok-a"}"#),
        Reply::ok(r#"{"contracts":[],"total":9,"next_cursor":"tok-b"}"#),
        Reply::ok(r#"{"contracts":[],"total":9,"next_cursor":"tok-c"}"#),
        Reply::ok(empty_with_token),
    ])
    .await;

    let mut walk = registry
        .client()
        .search_paginator(
            ContractSearchRequest::cursor("swap"),
            PageLimits::default()
                .with_page_size(2)
                .with_max_empty_pages(3),
        )
        .expect("paginator");

    let err = walk
        .collect_all()
        .await
        .expect_err("an endless stream of empty pages must stop");

    assert!(
        err.to_string().contains("consecutive empty page"),
        "unexpected error: {err}"
    );
    assert_eq!(
        registry.requests().len(),
        3,
        "the walk stops instead of requesting more"
    );
}

// ── Offset pagination (Elasticsearch endpoint, `results` + `total`) ────────────

#[tokio::test]
async fn offset_walk_uses_the_advanced_endpoint_and_stops_at_the_total() {
    let registry = FakeRegistry::start(vec![
        Reply::ok(&format!(
            r#"{{"query":"swap","total":3,"limit":2,"offset":0,"took_ms":4,"backend":"elasticsearch","results":[{},{}],"facets":{{"categories":[],"networks":[],"tags":[]}}}}"#,
            hit("aa"),
            hit("bb")
        )),
        Reply::ok(&format!(
            r#"{{"query":"swap","total":3,"limit":2,"offset":2,"took_ms":3,"backend":"elasticsearch","results":[{}],"facets":{{"categories":[],"networks":[],"tags":[]}}}}"#,
            hit("cc")
        )),
    ])
    .await;

    let mut walk = registry
        .client()
        .search_paginator(
            ContractSearchRequest::offset("swap"),
            PageLimits::default().with_page_size(2),
        )
        .expect("paginator");

    let collected = walk.collect_all().await.expect("walk should succeed");

    let names: Vec<String> = collected.items.iter().map(|hit| hit.name.clone()).collect();
    assert_eq!(names, vec!["aa", "bb", "cc"]);
    assert_eq!(collected.total, Some(3));
    assert_eq!(collected.stop_reason, StopReason::Exhausted);

    let requests = registry.requests();
    assert_eq!(
        requests.len(),
        2,
        "the total ends the walk without a probe page"
    );
    assert!(
        requests[0].contains("/api/v1/contracts/search?"),
        "offset mode uses the advanced endpoint: {}",
        requests[0]
    );
    assert!(requests[0].contains("offset=0"), "{}", requests[0]);
    assert!(requests[1].contains("offset=2"), "{}", requests[1]);
    assert!(
        !requests[0].contains("cursor="),
        "an offset walk never sends a cursor: {}",
        requests[0]
    );
}

#[tokio::test]
async fn offset_walk_can_resume_from_a_given_offset() {
    let registry = FakeRegistry::start(vec![Reply::ok(&format!(
        r#"{{"query":"swap","total":41,"limit":2,"offset":40,"results":[{}]}}"#,
        hit("aa")
    ))])
    .await;

    let mut walk = registry
        .client()
        .search_paginator(
            ContractSearchRequest::offset("swap").with_offset(Some(40)),
            PageLimits::default().with_page_size(2),
        )
        .expect("paginator");

    let page = walk
        .next_page()
        .await
        .expect("page fetch")
        .expect("a page of results");

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.total, Some(41));
    assert!(page.next.is_none(), "offset 40 + 1 item reaches total 41");
    assert!(registry.requests()[0].contains("offset=40"));
}

// ── Cross-cutting behaviour ───────────────────────────────────────────────────

#[tokio::test]
async fn a_retried_page_is_emitted_exactly_once() {
    let registry = FakeRegistry::start(vec![
        Reply::error(
            503,
            r#"{"code":"SERVICE_UNAVAILABLE","message":"try again"}"#,
        ),
        Reply::ok(&format!(
            r#"{{"contracts":[{},{}],"total":2,"took_ms":2}}"#,
            hit("aa"),
            hit("bb")
        )),
    ])
    .await;

    let mut walk = registry
        .client()
        .search_paginator(
            ContractSearchRequest::cursor("swap"),
            PageLimits::default().with_page_size(2),
        )
        .expect("paginator");

    let collected = walk.collect_all().await.expect("the retry should succeed");

    let names: Vec<String> = collected.items.iter().map(|hit| hit.name.clone()).collect();
    assert_eq!(
        names,
        vec!["aa", "bb"],
        "the retry does not duplicate items"
    );
    assert_eq!(
        collected.pages_fetched, 1,
        "a retried fetch is still one page"
    );
    assert_eq!(
        registry.requests().len(),
        2,
        "the transient failure was retried"
    );
}

#[tokio::test]
async fn max_items_bounds_a_full_walk_and_reports_where_it_stopped() {
    let page = format!(
        r#"{{"contracts":[{},{}],"total":1000,"next_cursor":"tok-next"}}"#,
        hit("aa"),
        hit("bb")
    );
    let registry = FakeRegistry::start(vec![
        Reply::ok(&page),
        Reply::ok(&page.replace("tok-next", "tok-later")),
        Reply::ok(&page.replace("tok-next", "tok-latest")),
    ])
    .await;

    let mut walk = registry
        .client()
        .search_paginator(
            ContractSearchRequest::cursor("swap"),
            PageLimits::default()
                .with_page_size(2)
                .with_max_items(Some(4)),
        )
        .expect("paginator");

    let collected = walk.collect_all().await.expect("walk should succeed");

    assert_eq!(collected.items.len(), 4);
    assert_eq!(collected.stop_reason, StopReason::MaxItems);
    assert!(!collected.is_complete(), "the result set has 1000 matches");
    assert_eq!(collected.total, Some(1000));
    assert_eq!(
        collected.next,
        Some(PageCursor::Cursor("tok-later".to_string())),
        "the walk reports where a follow-up should resume"
    );
    assert_eq!(registry.requests().len(), 2, "no page beyond the bound");
}

#[tokio::test]
async fn cancelling_between_pages_keeps_what_was_already_fetched() {
    let page = format!(
        r#"{{"contracts":[{}],"total":100,"next_cursor":"tok-2"}}"#,
        hit("aa")
    );
    let registry = FakeRegistry::start(vec![Reply::ok(&page)]).await;

    let mut walk = registry
        .client()
        .search_paginator(
            ContractSearchRequest::cursor("swap"),
            PageLimits::default().with_page_size(1),
        )
        .expect("paginator");
    let cancel = walk.cancel_token();

    let first = walk
        .next_page()
        .await
        .expect("page fetch")
        .expect("a page of results");
    assert_eq!(first.items.len(), 1);

    cancel.cancel();
    assert!(walk
        .next_page()
        .await
        .expect("cancellation is graceful")
        .is_none());
    assert_eq!(walk.stop_reason(), Some(StopReason::Cancelled));
    assert_eq!(
        registry.requests().len(),
        1,
        "no request is made after cancellation"
    );
}

#[tokio::test]
async fn a_cursor_walk_cannot_be_combined_with_an_offset() {
    let registry = FakeRegistry::start(Vec::new()).await;

    let err = registry
        .client()
        .search_paginator(
            ContractSearchRequest::new("swap", PaginationMode::Cursor).with_offset(Some(20)),
            PageLimits::default(),
        )
        .expect_err("mixing the two schemes must be rejected");

    assert!(err.to_string().contains("cannot be combined"), "{err}");
    assert!(
        registry.requests().is_empty(),
        "the request never reaches the server"
    );
}
