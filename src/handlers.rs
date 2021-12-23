use axum::{
    extract::{OriginalUri, RequestParts, Extension, FromRequest},
    async_trait,
    http::{StatusCode},
    http::{Response},
    response::IntoResponse,
    Json,
};
use hyper::{Body};
use serde_json::{Value};
use std::sync::Arc;
use std::convert::Infallible;

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

pub async fn pass_through(Extension(state): Extension<Arc<State>>, payload: Option<Json<Value>>, OriginalUri(original_uri): OriginalUri, RequestMethod(method): RequestMethod) -> Response<Body> {
    let parts = original_uri.into_parts();
    let path_and_query = parts.path_and_query.expect("Missing post path and query");
    log::info!("{{\"fn\": \"pass_through\", \"method\": \"{}\", \"uri\":\"{}\"}}", &method.as_str(), &path_and_query);
    state.response(method, &path_and_query.as_str(), payload).await
}

pub async fn handler_404(OriginalUri(original_uri): OriginalUri) -> impl IntoResponse {
    let parts = original_uri.into_parts();
    let path_and_query = parts.path_and_query.expect("Missing post path and query");
    log::info!("\"Bad path: {}\"", path_and_query);
    (StatusCode::NOT_FOUND, "{\"error_code\": 404, \"message\": \"HTTP 404 Not Found\"}")
}
