use axum::http::header::{HeaderValue, FORWARDED, USER_AGENT};
use axum::{http::Request, middleware::Next, response::IntoResponse};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::time::Instant;

pub fn setup_metrics_recorder() -> PrometheusHandle {
    const EXPONENTIAL_SECONDS: &[f64] = &[
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];

    PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("proxima_requests_duration_seconds".to_string()),
            EXPONENTIAL_SECONDS,
        )
        .unwrap()
        .install_recorder()
        .unwrap()
}

pub async fn track_metrics<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    let start = Instant::now();
    let path = req.uri().path().to_owned();
    let method = req.method().clone();
    let headers = req.headers().clone();
    let user_agent = headers
        .get(USER_AGENT)
        .unwrap_or(&HeaderValue::from_str("missing").unwrap())
        .to_str()
        .unwrap_or("error")
        .to_owned();

    let client = if let Some(x_forwarded) = headers.get("x-forwarded-for") {
        x_forwarded.to_str().unwrap_or("error").to_owned()
    } else if let Some(forwarded) = headers.get(FORWARDED) {
        forwarded.to_str().unwrap_or("error").to_owned()
    } else {
        "missing".to_string()
    };

    let response = next.run(req).await;
    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    let labels = [
        ("method", method.to_string()),
        ("path", path),
        ("status", status),
        ("user-agent", user_agent),
        ("client", client),
    ];

    metrics::increment_counter!("proxima_requests_total", &labels);
    metrics::histogram!("proxima_requests_duration_seconds", latency, &labels);

    response
}
