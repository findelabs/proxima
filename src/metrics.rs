use axum::{http::Request, middleware::Next, response::IntoResponse};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use hyper::body::HttpBody;
use std::time::Instant;

pub fn setup_metrics_recorder() -> PrometheusHandle {
    const EXPONENTIAL_SECONDS: &[f64] = &[
        0.001, 0.01, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0,
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

pub async fn track_metrics<B: HttpBody>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    let start = Instant::now();
    let path = req.uri().path().to_owned();
    let method = req.method().clone();
    let receive = req.body().size_hint().upper().unwrap_or(0) as f64;

    let response = next.run(req).await;
    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();
    let transmit = response.body().size_hint().upper().unwrap_or(0) as f64;

    let labels = [
        ("method", method.to_string()),
        ("path", path),
        ("status", status)
    ];

    metrics::increment_counter!("proxima_requests_total", &labels);
    metrics::increment_gauge!("proxima_requests_receive_bytes", receive, &labels);
    metrics::increment_gauge!("proxima_requests_transmit_bytes", transmit, &labels);
    metrics::histogram!("proxima_requests_duration_seconds", latency, &labels);
    response
}
