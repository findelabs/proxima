use axum::extract::BodyStream;
use axum::{
    async_trait,
    extract::{Extension, FromRequest, OriginalUri, Path, RawQuery, RequestParts, ConnectInfo},
    http::Response,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use clap::{crate_description, crate_name, crate_version};
use hyper::{Body, HeaderMap};
use serde_json::json;
use serde_json::Value;
use std::convert::Infallible;
use std::net::SocketAddr;

use crate::error::Error as RestError;
use crate::path::ProxyPath;
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

pub async fn test_proxy(path: ProxyPath) -> Json<Value>{

    Json(json!({"path": path.path(), "prefix": path.prefix(), "suffix": path.suffix()}))
}


pub async fn proxy(
    Extension(mut state): Extension<State>,
    payload: Option<BodyStream>,
    Path((endpoint, path)): Path<(String, String)>,
    RequestMethod(method): RequestMethod,
    all_headers: HeaderMap,
    RawQuery(query): RawQuery,
    ConnectInfo(addr): ConnectInfo<SocketAddr>
) -> Result<Response<Body>, RestError> {
    log::info!(
        "{{\"fn\": \"proxy\", \"method\": \"{}\", \"addr\":\"{}\", \"endpoint\":\"{}\", \"uri\":\"{}\"}}",
        &method.as_str(),
        &addr,
        &endpoint,
        &path
    );
    state
        .response(method, &endpoint, &path, query, all_headers, payload)
        .await
}

pub async fn endpoint(
    Path(endpoint): Path<String>,
    Extension(mut state): Extension<State>,
    RequestMethod(method): RequestMethod,
    ConnectInfo(addr): ConnectInfo<SocketAddr>
) -> Json<Value> {
    log::info!(
        "{{\"fn\": \"endpoint\", \"method\": \"{}\", \"addr\":\"{}\", \"endpoint\":\"{}\"}}",
        &method,
        &addr,
        &endpoint,
    );
    match state.get_entry(&endpoint, "").await {
        Some((e, _)) => Json(json!(e)),
        None => Json(json!({"error": "unknown endpoint"})),
    }
}

pub async fn reload(Extension(mut state): Extension<State>, RequestMethod(method): RequestMethod, ConnectInfo(addr): ConnectInfo<SocketAddr>) {
    log::info!(
        "{{\"fn\": \"reload\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/reload\"}}",
        &method,
        &addr,
    );
    state.reload().await;
}

pub async fn config(Extension(mut state): Extension<State>, RequestMethod(method): RequestMethod, ConnectInfo(addr): ConnectInfo<SocketAddr>) -> Json<Value> {
    log::info!(
        "{{\"fn\": \"config\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/config\"}}",
        &method,
        &addr,
    );
    Json(state.config().await)
}

pub async fn health(RequestMethod(method): RequestMethod, ConnectInfo(addr): ConnectInfo<SocketAddr>) -> Json<Value> {
    log::info!(
        "{{\"fn\": \"health\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/health\"}}",
        &method,
        &addr,
    );
    Json(json!({ "msg": "Healthy"}))
}

pub async fn root(RequestMethod(method): RequestMethod, ConnectInfo(addr): ConnectInfo<SocketAddr>) -> Json<Value> {
    log::info!(
        "{{\"fn\": \"root\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/\"}}",
        &method,
        &addr,
    );
    Json(
        json!({ "version": crate_version!(), "name": crate_name!(), "description": crate_description!()}),
    )
}

pub async fn echo(Json(payload): Json<Value>, RequestMethod(method): RequestMethod, ConnectInfo(addr): ConnectInfo<SocketAddr>) -> Json<Value> {
    log::info!(
        "{{\"fn\": \"echo\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/echo\"}}",
        &method,
        &addr,
    );
    Json(payload)
}

pub async fn help(RequestMethod(method): RequestMethod, ConnectInfo(addr): ConnectInfo<SocketAddr>) -> Json<Value> {
    log::info!(
        "{{\"fn\": \"help\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/help\"}}",
        &method,
        &addr,
    );
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

pub async fn handler_404(OriginalUri(original_uri): OriginalUri, RequestMethod(method): RequestMethod, ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    let parts = original_uri.into_parts();
    let path_and_query = parts.path_and_query.expect("Missing post path and query");
    log::info!(
        "{{\"fn\": \"handler_404\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"{}\"}}",
        &method,
        &addr,
        &path_and_query,
    );
    (
        StatusCode::NOT_FOUND,
        "{\"error_code\": 404, \"message\": \"HTTP 404 Not Found\"}",
    )
}
