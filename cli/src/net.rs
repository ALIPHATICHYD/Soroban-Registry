use anyhow::{anyhow, Result};
use reqwest::{Client as ReqwestClient, Method, RequestBuilder, Response, StatusCode, Url};
use std::{future::Future, pin::Pin, time::Duration};

const MAX_ATTEMPTS: usize = 3;
const BACKOFFS_MS: [u64; MAX_ATTEMPTS - 1] = [250, 750];
const DEFAULT_TIMEOUT_SECS: u64 = 30;

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .build()
        .unwrap_or_else(|err| {
            eprintln!(
                "Warning: failed to configure HTTP timeout ({}); using reqwest defaults.",
                err
            );
            ReqwestClient::new()
        })
}

pub trait RequestBuilderExt {
    fn send_with_retry(self) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>>;
}

impl RequestBuilderExt for RequestBuilder {
    fn send_with_retry(self) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>> {
        Box::pin(send_with_retry(self))
    }
}

async fn send_with_retry(builder: RequestBuilder) -> Result<Response> {
    let (method, url) = request_summary(&builder);
    let builder = attach_auth(builder, url.as_ref()).await?;

    if builder.try_clone().is_none() {
        return builder
            .send()
            .await
            .map_err(|err| friendly_network_error(err, method.as_ref(), url.as_ref(), 1));
    }

    let mut last_error = None;

    for attempt in 1..=MAX_ATTEMPTS {
        let response = builder
            .try_clone()
            .ok_or_else(|| anyhow!("Failed to retry request: request body is not reusable"))?
            .send()
            .await;

        match response {
            Ok(response) => {
                if should_retry_status(method.as_ref(), response.status()) && attempt < MAX_ATTEMPTS
                {
                    sleep_before_retry(attempt).await;
                    continue;
                }

                return Ok(response);
            }
            Err(err) => {
                if is_transient_error(&err, method.as_ref()) && attempt < MAX_ATTEMPTS {
                    last_error = Some(err);
                    sleep_before_retry(attempt).await;
                    continue;
                }

                return Err(friendly_network_error(
                    err,
                    method.as_ref(),
                    url.as_ref(),
                    attempt,
                ));
            }
        }
    }

    if let Some(err) = last_error {
        return Err(friendly_network_error(
            err,
            method.as_ref(),
            url.as_ref(),
            MAX_ATTEMPTS,
        ));
    }

    Err(anyhow!(
        "Network request failed after {} attempt(s). Check the API URL, your network connection, and try again.",
        MAX_ATTEMPTS
    ))
}

async fn attach_auth(builder: RequestBuilder, url: Option<&Url>) -> Result<RequestBuilder> {
    if builder.try_clone().is_none() {
        return Ok(builder);
    }

    let Some(base_url) = request_base_url(url) else {
        return Ok(builder);
    };

    let Some(token) = crate::auth::access_token_for_requests(&base_url).await? else {
        return Ok(builder);
    };

    Ok(builder.header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token)))
}

fn request_base_url(url: Option<&Url>) -> Option<String> {
    let url = url?;
    let mut base = format!("{}://{}", url.scheme(), url.host_str()?);
    if let Some(port) = url.port() {
        base.push(':');
        base.push_str(&port.to_string());
    }
    Some(base)
}

fn request_summary(builder: &RequestBuilder) -> (Option<Method>, Option<Url>) {
    builder
        .try_clone()
        .and_then(|cloned| cloned.build().ok())
        .map(|request| (Some(request.method().clone()), Some(request.url().clone())))
        .unwrap_or((None, None))
}

fn is_transient_error(err: &reqwest::Error, method: Option<&Method>) -> bool {
    err.is_connect() || (is_retryable_method(method) && (err.is_timeout() || err.is_request()))
}

fn should_retry_status(method: Option<&Method>, status: StatusCode) -> bool {
    is_retryable_method(method)
        && (status == StatusCode::REQUEST_TIMEOUT
            || status == StatusCode::TOO_MANY_REQUESTS
            || status == StatusCode::INTERNAL_SERVER_ERROR
            || status == StatusCode::BAD_GATEWAY
            || status == StatusCode::SERVICE_UNAVAILABLE
            || status == StatusCode::GATEWAY_TIMEOUT)
}

fn is_retryable_method(method: Option<&Method>) -> bool {
    match method {
        Some(method) => {
            method == Method::GET || method == Method::HEAD || method == Method::OPTIONS
        }
        None => true,
    }
}

async fn sleep_before_retry(attempt: usize) {
    let delay_ms = BACKOFFS_MS[attempt.saturating_sub(1).min(BACKOFFS_MS.len() - 1)];
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
}

fn friendly_network_error(
    err: reqwest::Error,
    method: Option<&Method>,
    url: Option<&Url>,
    attempts: usize,
) -> anyhow::Error {
    let target = match (method, url) {
        (Some(method), Some(url)) => format!("{} {}", method, url),
        (_, Some(url)) => url.to_string(),
        _ => "registry API".to_string(),
    };

    let reason = if err.is_timeout() {
        "the request timed out"
    } else if err.is_connect() {
        "the CLI could not connect to the server"
    } else if err.is_request() {
        "the request could not be sent"
    } else if err.is_decode() {
        "the response could not be decoded"
    } else {
        "the network request failed"
    };

    anyhow!(
        "Network request failed after {} attempt(s): {} ({}). Check the API URL, your network connection, and try again.",
        attempts,
        target,
        reason
    )
}
