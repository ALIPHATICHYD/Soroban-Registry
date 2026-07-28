// Pagination helpers: RFC 5988 Link headers and PagedJson response type.
//
// Usage in a handler:
//
//   pub async fn list_things(
//       headers: HeaderMap,
//       OriginalUri(uri): OriginalUri,
//       ...
//   ) -> ApiResult<PagedJson<Thing>> {
//       let body = PaginatedResponse::new(items, total, page, limit);
//       Ok(PagedJson::new(body, &headers, &uri))
//   }

use axum::{
    http::{HeaderMap, HeaderValue, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use shared::PaginatedResponse;

// ─── PagedJson ────────────────────────────────────────────────────────────────

/// An Axum response that serialises a [`PaginatedResponse`] as JSON and
/// appends an RFC 5988 `Link` header when more than one page exists.
pub struct PagedJson<T: Serialize + Send>(PaginatedResponse<T>, Option<String>);

impl<T: Serialize + Send> PagedJson<T> {
    /// Build the response, deriving Link header URLs from the incoming request.
    pub fn new(body: PaginatedResponse<T>, req_headers: &HeaderMap, uri: &Uri) -> Self {
        let link = link_header(
            req_headers,
            uri,
            body.page,
            body.total_pages,
            body.page_size,
        );
        Self(body, link)
    }
}

impl<T: Serialize + Send> IntoResponse for PagedJson<T> {
    fn into_response(self) -> Response {
        let PagedJson(body, link) = self;
        let mut headers = HeaderMap::new();
        if let Some(link_val) = link {
            if let Ok(v) = HeaderValue::from_str(&link_val) {
                headers.insert(axum::http::header::LINK, v);
            }
        }
        (StatusCode::OK, headers, Json(body)).into_response()
    }
}

// ─── Link header builder ──────────────────────────────────────────────────────

/// Produces an RFC 5988 `Link` header value for the given page/pages context.
/// Returns `None` when there is only one page (no navigation links needed).
///
/// All existing query parameters except `page`, `per_page`, `limit`, and
/// `offset` are preserved in the generated URLs.
pub fn link_header(
    req_headers: &HeaderMap,
    uri: &Uri,
    page: i64,
    pages: i64,
    per_page: i64,
) -> Option<String> {
    if pages <= 1 {
        return None;
    }

    let scheme = req_headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("https");

    let host = req_headers
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");

    let path = uri.path();
    let existing = uri.query().unwrap_or("");

    let url = |p: i64| -> String {
        format!(
            "{}://{}{}?{}",
            scheme,
            host,
            path,
            paged_query(existing, p, per_page)
        )
    };

    let mut parts = vec![
        format!("<{}>; rel=\"first\"", url(1)),
        format!("<{}>; rel=\"last\"", url(pages)),
    ];
    if page > 1 {
        parts.push(format!("<{}>; rel=\"prev\"", url(page - 1)));
    }
    if page < pages {
        parts.push(format!("<{}>; rel=\"next\"", url(page + 1)));
    }

    Some(parts.join(", "))
}

/// Rebuilds a query string, replacing (or inserting) `page` and `per_page`
/// and stripping out legacy `limit`/`offset` params.
fn paged_query(existing: &str, page: i64, per_page: i64) -> String {
    let mut params: Vec<(String, String)> = existing
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|kv| {
            let mut it = kv.splitn(2, '=');
            let k = it.next()?.to_string();
            let v = it.next().unwrap_or("").to_string();
            Some((k, v))
        })
        .filter(|(k, _)| !matches!(k.as_str(), "page" | "per_page" | "limit" | "offset"))
        .collect();

    params.push(("page".to_string(), page.to_string()));
    params.push(("per_page".to_string(), per_page.to_string()));

    params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&")
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(path_query: &str) -> Uri {
        path_query.parse().unwrap()
    }

    fn headers_with_host(host: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::HOST,
            HeaderValue::from_str(host).unwrap(),
        );
        h.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        h
    }

    #[test]
    fn no_link_header_when_single_page() {
        let h = headers_with_host("api.example.com");
        let u = uri("/api/items?limit=20");
        assert!(link_header(&h, &u, 1, 1, 20).is_none());
    }

    #[test]
    fn first_page_has_no_prev() {
        let h = headers_with_host("api.example.com");
        let u = uri("/api/items?category=defi");
        let header = link_header(&h, &u, 1, 5, 20).unwrap();
        assert!(header.contains("rel=\"next\""));
        assert!(header.contains("rel=\"first\""));
        assert!(header.contains("rel=\"last\""));
        assert!(!header.contains("rel=\"prev\""));
    }

    #[test]
    fn last_page_has_no_next() {
        let h = headers_with_host("api.example.com");
        let u = uri("/api/items");
        let header = link_header(&h, &u, 5, 5, 20).unwrap();
        assert!(header.contains("rel=\"prev\""));
        assert!(!header.contains("rel=\"next\""));
    }

    #[test]
    fn middle_page_has_all_links() {
        let h = headers_with_host("api.example.com");
        let u = uri("/api/items?q=foo&limit=20&offset=40");
        let header = link_header(&h, &u, 3, 5, 20).unwrap();
        assert!(header.contains("rel=\"first\""));
        assert!(header.contains("rel=\"last\""));
        assert!(header.contains("rel=\"prev\""));
        assert!(header.contains("rel=\"next\""));
        // legacy params should be stripped, q should survive
        assert!(header.contains("q=foo"));
        assert!(!header.contains("offset="));
        assert!(!header.contains("limit="));
    }

    #[test]
    fn paged_query_strips_and_replaces() {
        let q = paged_query("limit=20&offset=40&network=testnet", 2, 20);
        assert!(q.contains("page=2"));
        assert!(q.contains("per_page=20"));
        assert!(q.contains("network=testnet"));
        assert!(!q.contains("limit="));
        assert!(!q.contains("offset="));
    }
}
