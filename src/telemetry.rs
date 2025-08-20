// Модуль для телеметрии: инициализация трейсера, shutdown, импорты

use opentelemetry::sdk::Resource;
use opentelemetry::sdk::trace as sdktrace;
use opentelemetry::trace::TraceError;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry::global;
use opentelemetry::global::shutdown_tracer_provider;
use opentelemetry::KeyValue;
use dotenv::dotenv;

#[cfg(feature = "telemetry")]
pub fn init_tracer() -> Result<sdktrace::Tracer, TraceError> {
    let signoz_endpoint = std::env::var("SIGNOZ_ENDPOINT").expect("SIGNOZ_ENDPOINT not set");
    let http_endpoint = if signoz_endpoint.ends_with("/v1/traces") {
        signoz_endpoint
    } else {
        format!("{}/v1/traces", signoz_endpoint.trim_end_matches('/'))
    };
    println!("Connecting to SigNoz at: {}", http_endpoint);
    let exporter = opentelemetry_otlp::new_exporter()
        .http()
        .with_endpoint(http_endpoint);
    let pipeline = opentelemetry_otlp::new_pipeline().tracing();
    if let Ok(api_key) = std::env::var("SIGNOZ_API_KEY") {
        unsafe {
            std::env::set_var("OTEL_EXPORTER_OTLP_HEADERS", format!("signoz-ingestion-key={}", api_key));
        }
        println!("Using API key authentication");
    }
    pipeline
        .with_exporter(exporter)
        .with_trace_config(
            sdktrace::config().with_resource(Resource::new(vec![
                KeyValue::new(
                    opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                    std::env::var("APP_NAME").unwrap_or_else(|_| "chainlink_multicall_signoz".to_string()),
                ),
            ])),
        )
        .install_batch(opentelemetry::runtime::Tokio)
}

