use axum::extract::BodyStream;
use axum::{
    async_trait,
    extract::{Extension, FromRequest, OriginalUri, Path, RawQuery, RequestParts},
    http::Response,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_debug::debug_handler;
use hyper::{Body, HeaderMap};
use serde_json::json;
use serde_json::Value;
use std::convert::Infallible;
use clap::{crate_version, crate_name, crate_description};

use crate::State;

// This is required in order to get the method from the request
#[derive(Debug)]
pub struct RequestMethod(pub hyper::Method);

// This is required in order to get the username/password from the request
#[derive(Debug)]
pub struct BasicAuth(pub String);

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

#[async_trait]
impl<B> FromRequest<B> for BasicAuth
where
    B: Send,
{
    type Rejection = Infallible;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let user_pass = match req.uri().authority() {
            Some(authority) => {
                println!("authority: {}", authority);
                let string = authority.as_str();
                let left = match string.split_once('@') {
                    Some((left, _)) => left,
                    None => return Ok(Self("".to_string())),
                };

                let user_pass = match left.split_once(r#"://"#) {
                    Some((_, right)) => right,
                    None => return Ok(Self("".to_string())),
                };

                user_pass
            }
            None => "",
        };

        Ok(Self(user_pass.to_string()))
    }
}

#[debug_handler]
pub async fn proxy(
    Extension(mut state): Extension<State>,
    payload: Option<BodyStream>,
    Path((endpoint, path)): Path<(String, String)>,
    RequestMethod(method): RequestMethod,
    all_headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Response<Body> {
    log::info!(
        "{{\"fn\": \"proxy\", \"method\": \"{}\", \"endpoint\":\"{}\",\"uri\":\"{}\"}}",
        &method.as_str(),
        &endpoint,
        &path
    );
    state
        .response(method, &endpoint, &path, query, all_headers, payload)
        .await
}

pub async fn get_endpoint(
    Path(endpoint): Path<String>,
    Extension(mut state): Extension<State>,
) -> Json<Value> {
    log::info!(
        "{{\"fn\": \"get_endpoint\", \"endpoint\":\"{}\"}}",
        endpoint
    );
    match state.get_entry(&endpoint).await {
        Some(e) => Json(json!(e)),
        None => Json(json!({"error": "unknown endpoint"})),
    }
}

pub async fn reload(Extension(mut state): Extension<State>) {
    log::info!("{{\"fn\": \"reload\", \"method\":\"get\"}}");
    state.reload().await;
}

pub async fn config(Extension(mut state): Extension<State>) -> Json<Value> {
    log::info!("{{\"fn\": \"config\", \"method\":\"get\"}}");
    Json(state.config().await)
}

pub async fn health() -> Json<Value> {
    log::info!("{{\"fn\": \"health\", \"method\":\"get\"}}");
    Json(json!({ "msg": "Healthy"}))
}

pub async fn root() -> Json<Value> {
    log::info!("{{\"fn\": \"root\", \"method\":\"get\"}}");
    Json(json!({ "version": crate_version!(), "name": crate_name!(), "description": crate_description!()}))
}

#[debug_handler]
pub async fn echo(Json(payload): Json<Value>) -> Json<Value> {
    log::info!("{{\"fn\": \"echo\", \"method\":\"post\"}}");
    Json(payload)
}

pub async fn help() -> Json<Value> {
    log::info!("{{\"fn\": \"help\", \"method\":\"get\"}}");
    let payload = json!({"paths": {
            "/health": "Get the health of the api",
            "/config": "Get config of api",
            "/reload": "Reload the api's config",
            "/echo": "Echo back json payload (debugging)",
            "/help": "Show this help message",
            "/:endpoint": "Show config for specific endpoint",
            "/:endpoint/*path": "Pass through any request to specified endpoint"
        }
    });
    Json(payload)
}

pub async fn handler_404(OriginalUri(original_uri): OriginalUri) -> impl IntoResponse {
    let parts = original_uri.into_parts();
    let path_and_query = parts.path_and_query.expect("Missing post path and query");
    log::info!(
        "{{\"fn\": \"handler_404\", \"method\":\"get\", \"path\":\"{}\"}}",
        path_and_query
    );
    (
        StatusCode::NOT_FOUND,
        "{\"error_code\": 404, \"message\": \"HTTP 404 Not Found\"}",
    )
}
