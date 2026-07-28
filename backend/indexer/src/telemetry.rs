use opentelemetry::global;
use opentelemetry::propagation::Injector;
use shared::logging::{init_logging, LogConfig};

pub fn init_tracing(service_name: &str) {
    let config = LogConfig::from_env_with_service(service_name);
    init_logging(config);
}

pub fn inject_current_trace_context(headers: &mut reqwest::header::HeaderMap) {
    let context = opentelemetry::Context::current();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&context, &mut ReqwestHeaderInjector(headers));
    });
}

struct ReqwestHeaderInjector<'a>(&'a mut reqwest::header::HeaderMap);

impl Injector for ReqwestHeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let (Ok(header_name), Ok(header_value)) = (
            reqwest::header::HeaderName::from_bytes(key.as_bytes()),
            reqwest::header::HeaderValue::from_str(&value),
        ) {
            self.0.insert(header_name, header_value);
        }
    }
}
