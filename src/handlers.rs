use axum::{
    async_trait,
    extract::{Extension, FromRequest, OriginalUri, RequestParts},
    http::Response,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use hyper::{Body, HeaderMap};
use serde_json::Value;
use std::convert::Infallible;
use std::sync::Arc;

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
    OriginalUri(original_uri): OriginalUri,
    RequestMethod(method): RequestMethod,
    all_headers: HeaderMap,
) -> Response<Body> {
    let parts = original_uri.into_parts();
    let path_and_query = parts.path_and_query.expect("Missing post path and query");
    log::info!(
        "{{\"fn\": \"pass_through\", \"method\": \"{}\", \"uri\":\"{}\"}}",
        &method.as_str(),
        &path_and_query
    );
    state
        .response(method, &path_and_query.as_str(), all_headers, payload)
        .await
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
