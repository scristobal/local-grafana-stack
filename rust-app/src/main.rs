use axum::{
    extract::Path,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use opentelemetry::{
    global,
    metrics::{Counter, Histogram},
    trace::{Span, Tracer, TracerProvider as _},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
    Resource,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use pyroscope::PyroscopeAgent;
use pyroscope_pprofrs::{pprof_backend, PprofConfig};

#[derive(Clone)]
struct AppMetrics {
    request_counter: Counter<u64>,
    request_duration: Histogram<f64>,
    error_counter: Counter<u64>,
}

impl AppMetrics {
    fn new() -> Self {
        let meter = global::meter("observability-demo");

        Self {
            request_counter: meter
                .u64_counter("http_requests_total")
                .with_description("Total number of HTTP requests")
                .build(),
            request_duration: meter
                .f64_histogram("http_request_duration_seconds")
                .with_description("HTTP request duration in seconds")
                .build(),
            error_counter: meter
                .u64_counter("errors_total")
                .with_description("Total number of errors")
                .build(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct CalculateRequest {
    a: f64,
    b: f64,
}

#[derive(Serialize)]
struct CalculateResponse {
    result: f64,
    operation: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_telemetry()?;

    info!("Starting observability demo application");

    let pyroscope_url = std::env::var("PYROSCOPE_URL")
        .unwrap_or_else(|_| "http://localhost:4040".to_string());

    let app_name = "rust-observability-demo".to_string();

    let agent = PyroscopeAgent::builder(&pyroscope_url, &app_name)
        .tags(vec![
            ("service", "rust-observability-demo"),
            ("environment", "development"),
        ])
        .backend(pprof_backend(PprofConfig::new().sample_rate(100)))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to initialize Pyroscope: {}", e))?;

    let agent_running = agent.start().map_err(|e| anyhow::anyhow!("Failed to start Pyroscope agent: {}", e))?;
    info!("Pyroscope continuous profiling started");

    let metrics = AppMetrics::new();

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/calculate/add", post(add_handler))
        .route("/calculate/divide", post(divide_handler))
        .route("/simulate/slow", get(slow_handler))
        .route("/simulate/error", get(error_handler))
        .route("/user/:id", get(user_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(metrics);

    let addr = "0.0.0.0:8080";
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    let agent_ready = agent_running.stop()?;
    agent_ready.shutdown();
    info!("Pyroscope profiling stopped");

    Ok(())
}

fn init_telemetry() -> anyhow::Result<()> {
    let otlp_endpoint = std::env::var("OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let resource = Resource::builder()
        .with_service_name("rust-observability-demo")
        .with_attribute(KeyValue::new("service.version", "0.1.0"))
        .with_attribute(KeyValue::new("deployment.environment", "development"))
        .build();

    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(
            opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_endpoint(&otlp_endpoint)
                .with_timeout(Duration::from_secs(3))
                .build()?,
        )
        .with_resource(resource.clone())
        .with_id_generator(RandomIdGenerator::default())
        .with_sampler(Sampler::AlwaysOn)
        .build();

    global::set_tracer_provider(tracer_provider.clone());

    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(&otlp_endpoint)
        .with_timeout(Duration::from_secs(3))
        .build()?;

    let reader = PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(10))
        .build();

    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource.clone())
        .with_reader(reader)
        .build();

    global::set_meter_provider(meter_provider);

    let log_exporter = opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .with_endpoint(&otlp_endpoint)
        .with_timeout(Duration::from_secs(3))
        .build()?;

    let logger_provider = opentelemetry_sdk::logs::SdkLoggerProvider::builder()
        .with_batch_exporter(log_exporter)
        .with_resource(resource.clone())
        .build();

    let telemetry_layer = tracing_opentelemetry::layer()
        .with_tracer(tracer_provider.tracer("observability-demo"));

    let otel_log_layer = opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(
        &logger_provider
    );

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,observability_demo=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .with(telemetry_layer)
        .with(otel_log_layer)
        .init();

    Ok(())
}

async fn root_handler() -> Html<&'static str> {
    info!("Root endpoint accessed");
    Html(
        r#"
        <h1>Rust Observability Demo</h1>
        <p>This application demonstrates integration with Grafana's observability stack:</p>
        <ul>
            <li><strong>Metrics</strong>: Sent to Mimir via OTLP</li>
            <li><strong>Logs</strong>: Sent to Loki via OTLP</li>
            <li><strong>Traces</strong>: Sent to Tempo via OTLP</li>
        </ul>
        <h2>Available Endpoints:</h2>
        <ul>
            <li>GET /health - Health check</li>
            <li>POST /calculate/add - Add two numbers</li>
            <li>POST /calculate/divide - Divide two numbers (can error)</li>
            <li>GET /simulate/slow - Simulate slow request</li>
            <li>GET /simulate/error - Simulate error</li>
            <li>GET /user/:id - Get user by ID</li>
        </ul>
        "#,
    )
}

#[axum::debug_handler]
async fn health_handler(axum::extract::State(metrics): axum::extract::State<AppMetrics>) -> impl IntoResponse {
    let start = std::time::Instant::now();

    info!("Health check requested");

    metrics.request_counter.add(
        1,
        &[KeyValue::new("endpoint", "/health"), KeyValue::new("method", "GET")],
    );

    let duration = start.elapsed().as_secs_f64();
    metrics.request_duration.record(
        duration,
        &[KeyValue::new("endpoint", "/health")],
    );

    Json(serde_json::json!({
        "status": "healthy",
        "service": "rust-observability-demo"
    }))
}

#[axum::debug_handler]
async fn add_handler(
    axum::extract::State(metrics): axum::extract::State<AppMetrics>,
    Json(payload): Json<CalculateRequest>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let tracer = global::tracer("observability-demo");
    let mut span = tracer.start("calculate_add");

    span.set_attribute(KeyValue::new("operation", "add"));
    span.set_attribute(KeyValue::new("input.a", payload.a));
    span.set_attribute(KeyValue::new("input.b", payload.b));

    info!(a = payload.a, b = payload.b, "Adding two numbers");

    let result = payload.a + payload.b;
    span.set_attribute(KeyValue::new("result", result));

    metrics.request_counter.add(
        1,
        &[KeyValue::new("endpoint", "/calculate/add"), KeyValue::new("method", "POST")],
    );

    let duration = start.elapsed().as_secs_f64();
    metrics.request_duration.record(
        duration,
        &[KeyValue::new("endpoint", "/calculate/add")],
    );

    span.end();

    Json(CalculateResponse {
        result,
        operation: "addition".to_string(),
    })
}

#[axum::debug_handler]
async fn divide_handler(
    axum::extract::State(metrics): axum::extract::State<AppMetrics>,
    Json(payload): Json<CalculateRequest>,
) -> Result<Json<CalculateResponse>, (axum::http::StatusCode, String)> {
    let start = std::time::Instant::now();
    let tracer = global::tracer("observability-demo");
    let mut span = tracer.start("calculate_divide");

    span.set_attribute(KeyValue::new("operation", "divide"));
    span.set_attribute(KeyValue::new("input.a", payload.a));
    span.set_attribute(KeyValue::new("input.b", payload.b));

    info!(a = payload.a, b = payload.b, "Dividing two numbers");

    if payload.b == 0.0 {
        error!("Division by zero attempted");
        span.set_attribute(KeyValue::new("error", true));
        span.set_attribute(KeyValue::new("error.message", "division by zero"));

        metrics.error_counter.add(
            1,
            &[KeyValue::new("error_type", "division_by_zero")],
        );

        span.end();
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Cannot divide by zero".to_string(),
        ));
    }

    let result = payload.a / payload.b;
    span.set_attribute(KeyValue::new("result", result));

    metrics.request_counter.add(
        1,
        &[KeyValue::new("endpoint", "/calculate/divide"), KeyValue::new("method", "POST")],
    );

    let duration = start.elapsed().as_secs_f64();
    metrics.request_duration.record(
        duration,
        &[KeyValue::new("endpoint", "/calculate/divide")],
    );

    span.end();

    Ok(Json(CalculateResponse {
        result,
        operation: "division".to_string(),
    }))
}

#[axum::debug_handler]
async fn slow_handler(axum::extract::State(metrics): axum::extract::State<AppMetrics>) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let tracer = global::tracer("observability-demo");
    let mut span = tracer.start("slow_operation");

    warn!("Simulating slow request");

    tokio::time::sleep(Duration::from_secs(2)).await;

    metrics.request_counter.add(
        1,
        &[KeyValue::new("endpoint", "/simulate/slow"), KeyValue::new("method", "GET")],
    );

    let duration = start.elapsed().as_secs_f64();
    metrics.request_duration.record(
        duration,
        &[KeyValue::new("endpoint", "/simulate/slow")],
    );

    span.end();

    Json(serde_json::json!({
        "message": "Slow operation completed",
        "duration_seconds": duration
    }))
}

#[axum::debug_handler]
async fn error_handler(
    axum::extract::State(metrics): axum::extract::State<AppMetrics>,
) -> Result<(), (axum::http::StatusCode, String)> {
    let tracer = global::tracer("observability-demo");
    let mut span = tracer.start("error_operation");

    error!("Simulating error condition");

    span.set_attribute(KeyValue::new("error", true));
    span.set_attribute(KeyValue::new("error.message", "simulated error"));

    metrics.error_counter.add(
        1,
        &[KeyValue::new("error_type", "simulated")],
    );

    span.end();

    Err((
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "Simulated error occurred".to_string(),
    ))
}

#[axum::debug_handler]
async fn user_handler(
    axum::extract::State(metrics): axum::extract::State<AppMetrics>,
    Path(user_id): Path<u64>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let tracer = global::tracer("observability-demo");
    let mut span = tracer.start("get_user");

    span.set_attribute(KeyValue::new("user.id", user_id as i64));

    info!(user_id, "Fetching user information");

    // Simulate database lookup
    tokio::time::sleep(Duration::from_millis(100)).await;

    metrics.request_counter.add(
        1,
        &[KeyValue::new("endpoint", "/user/:id"), KeyValue::new("method", "GET")],
    );

    let duration = start.elapsed().as_secs_f64();
    metrics.request_duration.record(
        duration,
        &[KeyValue::new("endpoint", "/user/:id")],
    );

    span.end();

    Json(serde_json::json!({
        "id": user_id,
        "name": format!("User {}", user_id),
        "email": format!("user{}@example.com", user_id)
    }))
}
