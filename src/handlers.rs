use axum::{
    async_trait,
    extract::{Extension, FromRequest, OriginalUri, RequestParts, Path, RawQuery},
    http::Response,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use hyper::{Body, HeaderMap};
use serde_json::Value;
use std::convert::Infallible;
use std::sync::Arc;
use serde_json::json;

use crate::State;

// This is required in order to get the method from the request
#[derive(Debug)]
pub struct RequestMethod(pub hyper::Method);

#[async_trait]
impl<B> FromRequest<B> for RequestMethod
where
    B: Send,
{
    type Rejection = Infallible;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let method = req.method().to_owned();
        Ok(Self(method))
    }
}

pub async fn pass_through(
    Extension(state): Extension<Arc<State>>,
    payload: Option<Json<Value>>,
    Path((endpoint, path)): Path<(String, String)>,
    RequestMethod(method): RequestMethod,
    all_headers: HeaderMap,
    RawQuery(query): RawQuery
) -> Response<Body> {
    log::info!(
        "{{\"fn\": \"pass_through\", \"method\": \"{}\", \"endpoint\":\"{}\",\"uri\":\"{}\"}}",
        &method.as_str(),
        &endpoint,
        &path
    );
    state
        .response(method, &endpoint, &path, query, all_headers, payload)
        .await
}

pub async fn get_endpoint(Path(endpoint): Path<String>, Extension(state): Extension<Arc<State>>) -> Json<Value> {
    log::info!("\"GET /{}\"", endpoint);
	match state.get_entry(&endpoint).await {
		Some(e) => Json(json!(e)),
		None => Json(json!({"error": "unknown endpoint"}))
	}
}

pub async fn help(Extension(state): Extension<Arc<State>>) -> Json<Value> {
    log::info!("\"GET /\"");
	Json(state.config().await)
}

pub async fn health() -> Json<Value> {
    log::info!("\"GET /health\"");
    Json(json!({ "msg": "Healthy"}))
}

pub async fn echo(Json(payload): Json<Value>) -> Json<Value> {
    log::info!("\"POST /echo\"");
    Json(payload)
}

pub async fn handler_404(OriginalUri(original_uri): OriginalUri) -> impl IntoResponse {
    let parts = original_uri.into_parts();
    let path_and_query = parts.path_and_query.expect("Missing post path and query");
    log::info!("\"Bad path: {}\"", path_and_query);
    (
        StatusCode::NOT_FOUND,
        "{\"error_code\": 404, \"message\": \"HTTP 404 Not Found\"}",
    )
}
