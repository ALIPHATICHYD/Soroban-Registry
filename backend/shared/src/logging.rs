use once_cell::sync::OnceCell;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::Resource;
use serde_json::{Map, Value};
use std::backtrace::Backtrace;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

static LOG_GUARD: OnceCell<tracing_appender::non_blocking::WorkerGuard> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct LogConfig {
    pub service_name: String,
    pub log_level: String,
    pub otlp_endpoint: Option<String>,
    pub json_output: bool,
    pub log_dir: Option<String>,
    pub enable_otel: bool,
    pub sample_rate: f64,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            service_name: std::env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "soroban-registry-service".to_string()),
            log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            otlp_endpoint: std::env::var("OTLP_ENDPOINT")
                .or_else(|_| std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT"))
                .ok(),
            json_output: std::env::var("JSON_LOG")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
            log_dir: std::env::var("LOG_DIR").ok(),
            enable_otel: std::env::var("ENABLE_OTEL")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
            sample_rate: std::env::var("OTEL_TRACES_SAMPLER_ARG")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(1.0),
        }
    }
}

impl LogConfig {
    pub fn from_env_with_service(service_name: &str) -> Self {
        Self {
            service_name: std::env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| service_name.to_string()),
            ..Default::default()
        }
    }
}

pub fn init_logging(config: LogConfig) {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .parse_lossy(&config.log_level);

    let writer: BoxMakeWriter = if let Some(log_dir) = &config.log_dir {
        let file_appender =
            tracing_appender::rolling::daily(log_dir, format!("{}.log", &config.service_name));
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        let _ = LOG_GUARD.set(guard);
        BoxMakeWriter::new(non_blocking)
    } else {
        BoxMakeWriter::new(std::io::stdout)
    };

    let mut layers: Vec<Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync>> = Vec::new();

    if config.json_output {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(true)
            .with_writer(writer)
            .with_filter(env_filter.clone());
        layers.push(Box::new(fmt_layer));
    } else {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_writer(writer)
            .with_filter(env_filter.clone());
        layers.push(Box::new(fmt_layer));
    }

    if config.enable_otel {
        if let Some(endpoint) = &config.otlp_endpoint {
            let sampler = opentelemetry_sdk::trace::Sampler::ParentBased(Box::new(
                opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(config.sample_rate),
            ));

            let trace_config = opentelemetry_sdk::trace::Config::default()
                .with_sampler(sampler)
                .with_resource(Resource::new(vec![KeyValue::new(
                    "service.name",
                    config.service_name.clone(),
                )]));

            match opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_trace_config(trace_config)
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .tonic()
                        .with_endpoint(endpoint.clone()),
                )
                .install_batch(opentelemetry_sdk::runtime::Tokio)
            {
                Ok(provider) => {
                    let tracer = provider.tracer(config.service_name.clone());
                    let otel_layer = tracing_opentelemetry::layer()
                        .with_tracer(tracer)
                        .with_filter(env_filter);
                    layers.push(Box::new(otel_layer));
                }
                Err(err) => {
                    tracing::warn!(error = %err, "Failed to initialize OTLP exporter");
                }
            }
        }
    }

    let subscriber = tracing_subscriber::registry().with(layers);
    subscriber.init();
}

pub fn capture_backtrace() -> Option<String> {
    let backtrace = Backtrace::capture();
    if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
        Some(format!("{:#}", backtrace))
    } else {
        None
    }
}

pub fn redact_sensitive_data(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                if is_sensitive_key(&key) {
                    map.insert(key, Value::String("[REDACTED]".to_string()));
                } else if let Some(inner) = map.get_mut(&key) {
                    redact_sensitive_data(inner);
                }
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                redact_sensitive_data(item);
            }
        }
        _ => {}
    }
}

pub fn sanitize_value(value: &Value) -> Value {
    match value {
        Value::Object(obj) => {
            let mut sanitized = Map::new();
            for (key, val) in obj {
                if is_sensitive_key(key) {
                    sanitized.insert(key.clone(), Value::String("[REDACTED]".to_string()));
                } else {
                    sanitized.insert(key.clone(), sanitize_value(val));
                }
            }
            Value::Object(sanitized)
        }
        Value::Array(items) => Value::Array(items.iter().map(sanitize_value).collect()),
        other => other.clone(),
    }
}

pub fn is_sensitive_key(key: &str) -> bool {
    let lowered = key.to_ascii_lowercase();
    lowered.contains("password")
        || lowered.contains("secret")
        || lowered.contains("token")
        || lowered.contains("api_key")
        || lowered.contains("private_key")
        || lowered.contains("authorization")
        || lowered.contains("cookie")
        || lowered.contains("jwt")
        || lowered.contains("session")
        || lowered.contains("credit_card")
        || lowered.contains("ssn")
        || lowered.contains("access_key")
        || lowered.contains("mnemonic")
        || lowered.contains("seed_phrase")
}

pub fn log_error(message: &str, category: &str, status: u16, request_id: Option<&str>) {
    let backtrace = capture_backtrace();
    tracing::error!(
        category = category,
        status = status,
        request_id = request_id,
        backtrace = %backtrace.as_deref().unwrap_or("none"),
        "{message}",
    );
}

pub fn log_warn(message: &str, category: &str, request_id: Option<&str>) {
    tracing::warn!(category = category, request_id = request_id, "{message}",);
}

pub fn log_info(message: &str, category: &str, request_id: Option<&str>) {
    tracing::info!(category = category, request_id = request_id, "{message}",);
}

pub fn log_debug(message: &str, category: &str, request_id: Option<&str>) {
    tracing::debug!(category = category, request_id = request_id, "{message}",);
}

#[macro_export]
macro_rules! fatal {
    ($($arg:tt)+) => {{
        let backtrace = $crate::logging::capture_backtrace();
        tracing::error!(
            fatal = true,
            backtrace = %backtrace.as_deref().unwrap_or("none"),
            $($arg)+
        );
        std::process::abort();
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_redact_sensitive_data() {
        let mut data = json!({
            "username": "alice",
            "password": "supersecret",
            "token": "abc123",
            "nested": {
                "api_key": "key123",
                "normal_field": "hello",
            }
        });

        redact_sensitive_data(&mut data);

        assert_eq!(data["password"], "[REDACTED]");
        assert_eq!(data["token"], "[REDACTED]");
        assert_eq!(data["nested"]["api_key"], "[REDACTED]");
        assert_eq!(data["nested"]["normal_field"], "hello");
        assert_eq!(data["username"], "alice");
    }

    #[test]
    fn test_sanitize_value_does_not_modify_original() {
        let original = json!({ "password": "secret123" });
        let sanitized = sanitize_value(&original);
        assert_eq!(original["password"], "secret123");
        assert_eq!(sanitized["password"], "[REDACTED]");
    }

    #[test]
    fn test_is_sensitive_key() {
        assert!(is_sensitive_key("password"));
        assert!(is_sensitive_key("PASSWORD"));
        assert!(is_sensitive_key("api_key"));
        assert!(is_sensitive_key("authorization"));
        assert!(is_sensitive_key("private_key"));
        assert!(is_sensitive_key("credit_card"));
        assert!(is_sensitive_key("mnemonic"));
        assert!(!is_sensitive_key("username"));
        assert!(!is_sensitive_key("email"));
        assert!(!is_sensitive_key("contract_id"));
    }

    #[test]
    fn test_capture_backtrace() {
        let bt = capture_backtrace();
        if let Some(backtrace) = bt {
            assert!(backtrace.contains("capture_backtrace"));
        }
    }
}
