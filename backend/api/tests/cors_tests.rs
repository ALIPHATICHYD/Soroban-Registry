use axum::http::{HeaderName, HeaderValue};
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    routing::get,
    Router,
};
use std::time::Duration;
use tower::ServiceExt;
use tower_http::cors::CorsLayer;

#[tokio::test]
async fn test_cors_allowed_origin() {
    let origins = vec![
        HeaderValue::from_static("http://localhost:3000"),
        HeaderValue::from_static("https://soroban-registry.vercel.app"),
    ];
    let cors = CorsLayer::new().allow_origin(origins).allow_methods([
        Method::GET,
        Method::POST,
        Method::OPTIONS,
    ]);

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .layer(cors);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:3000")
                .method(Method::GET)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap(),
        "http://localhost:3000"
    );
}

#[tokio::test]
async fn test_cors_blocked_origin() {
    let origins = vec![
        HeaderValue::from_static("http://localhost:3000"),
        HeaderValue::from_static("https://soroban-registry.vercel.app"),
    ];
    let cors = CorsLayer::new().allow_origin(origins).allow_methods([
        Method::GET,
        Method::POST,
        Method::OPTIONS,
    ]);

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .layer(cors);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header(header::ORIGIN, "http://malicious.com")
                .method(Method::GET)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    // Unauthorized origins should NOT have the Access-Control-Allow-Origin header
    assert!(response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .is_none());
}

#[tokio::test]
async fn test_cors_preflight_advertises_max_age_and_exposed_headers() {
    let origins = vec![HeaderValue::from_static("http://localhost:3000")];
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .expose_headers([
            header::RETRY_AFTER,
            HeaderName::from_static("x-ratelimit-limit"),
            HeaderName::from_static("x-ratelimit-remaining"),
            HeaderName::from_static("x-ratelimit-reset"),
            HeaderName::from_static("x-ratelimit-tier"),
        ])
        .max_age(Duration::from_secs(3600));

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .layer(cors);

    let preflight = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .method(Method::OPTIONS)
                .header(header::ORIGIN, "http://localhost:3000")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        preflight
            .headers()
            .get(header::ACCESS_CONTROL_MAX_AGE)
            .unwrap(),
        "3600"
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:3000")
                .method(Method::GET)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let exposed = response
        .headers()
        .get(header::ACCESS_CONTROL_EXPOSE_HEADERS)
        .expect("expose-headers must be set for allowed origins")
        .to_str()
        .unwrap()
        .to_lowercase();

    for expected in [
        "retry-after",
        "x-ratelimit-limit",
        "x-ratelimit-remaining",
        "x-ratelimit-reset",
        "x-ratelimit-tier",
    ] {
        assert!(
            exposed.contains(expected),
            "expected '{expected}' to be exposed via CORS, got: {exposed}"
        );
    }
}

#[tokio::test]
async fn test_cors_preflight_blocked() {
    let origins = vec![HeaderValue::from_static("http://localhost:3000")];
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/api/test", get(|| async { "ok" }))
        .layer(cors);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/test")
                .method(Method::OPTIONS)
                .header(header::ORIGIN, "http://malicious.com")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Preflight for unauthorized origin should not return 200 OK with CORS headers
    // Actually Tower CORS returns 200 OK but without CORS headers for blocked origins
    assert!(response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .is_none());
}
