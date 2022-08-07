use axum::{
    async_trait,
    extract::{
        BodyStream, ConnectInfo, Extension, FromRequest, OriginalUri, Query, RawQuery, RequestParts,
    },
    http::Response,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use hyper::{Body, HeaderMap};
use metrics_exporter_prometheus::PrometheusHandle;
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::net::SocketAddr;
use http::header::FORWARDED;
use http::header::USER_AGENT;
use http::HeaderValue;

use crate::error::Error as ProximaError;
use crate::path::ProxyPath;
use crate::State;

// This is required in order to get the method from the request
#[derive(Debug)]
pub struct RequestMethod(pub hyper::Method);

// This is for accessing the cache
#[derive(Deserialize)]
pub struct CacheParams {
    key: Option<String>,
}

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

pub async fn metrics(
    Extension(recorder_handle): Extension<PrometheusHandle>,
) -> Result<String, ProximaError> {
    log::debug!("{{\"fn\": \"metrics\", \"method\":\"get\"}}");
    Ok(recorder_handle.render())
}

pub async fn proxy(
    Extension(mut state): Extension<State>,
    payload: Option<BodyStream>,
    path: ProxyPath,
    RequestMethod(method): RequestMethod,
    all_headers: HeaderMap,
    RawQuery(query): RawQuery,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Response<Body>, ProximaError> {

    // Check for forwarded for
    let forwarded_for = if let Some(x_forwarded) = all_headers.get("x-forwarded-for") {
        x_forwarded.to_str().unwrap_or("error").to_owned()
    } else if let Some(forwarded) = all_headers.get(FORWARDED) {
        forwarded.to_str().unwrap_or("error").to_owned()
    } else {
        "none".to_string()
    };

    let user_agent = all_headers
        .get(USER_AGENT)
        .unwrap_or(&HeaderValue::from_str("none").unwrap())
        .to_str()
        .unwrap_or("error")
        .to_owned();

    log::debug!(
        "{{\"type\": \"incoming connection\", \"method\": \"{}\", \"path\":\"{}\", \"query\": \"{}\", \"addr\":\"{}\", \"forwarded_for\": \"{}\", \"user_agent\":\"{}\"}}",
        &method.as_str(),
        &path.path(),
        query.clone().unwrap_or_else(|| "none".to_string()),
        &addr,
        forwarded_for,
        user_agent
    );

    match state
        .response(method.clone(), path.clone(), query.clone(), all_headers, payload, addr.clone())
        .await {
        Ok(s) => {
            log::info!(
                "{{\"type\": \"response\", \"method\": \"{}\", \"path\":\"{}\", \"query\": \"{}\", \"addr\":\"{}\", \"forwarded_for\": \"{}\", \"user_agent\": \"{}\", \"status\":\"{}\"}}",
                &method.as_str(),
                &path.path(),
                query.clone().unwrap_or_else(|| "none".to_string()),
                &addr,
                forwarded_for,
                user_agent,
                s.status()
            );
            Ok(s)
        },
        Err(e) => Err(e)
    }
}

pub async fn reload(
    Extension(mut state): Extension<State>,
    RequestMethod(method): RequestMethod,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) {
    log::debug!(
        "{{\"fn\": \"reload\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/reload\"}}",
        &method,
        &addr,
    );
    state.config.reload().await;
}

pub async fn config(
    Extension(mut state): Extension<State>,
    RequestMethod(method): RequestMethod,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Json<Value> {
    log::debug!(
        "{{\"fn\": \"config\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/config\"}}",
        &method,
        &addr,
    );
    Json(json!(state.config().await.global))
}

pub async fn routes(
    Extension(mut state): Extension<State>,
    RequestMethod(method): RequestMethod,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Json<Value> {
    log::debug!(
        "{{\"fn\": \"routes\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/routes\"}}",
        &method,
        &addr,
    );
    Json(json!(state.config().await.routes))
}

pub async fn mappings_get(
    Extension(mut state): Extension<State>,
    RequestMethod(method): RequestMethod,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Json<Value> {
    log::debug!(
        "{{\"fn\": \"cache\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/cache\"}}",
        &method,
        &addr,
    );
    Json(state.mappings_get().await)
}

pub async fn cache_get(
    Extension(mut state): Extension<State>,
    RequestMethod(method): RequestMethod,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Json<Value> {
    log::debug!(
        "{{\"fn\": \"cache\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/cache\"}}",
        &method,
        &addr,
    );
    Json(state.cache_get().await)
}

pub async fn cache_delete(
    Extension(mut state): Extension<State>,
    RequestMethod(method): RequestMethod,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(params): Query<CacheParams>,
) -> Json<Value> {
    log::debug!(
        "{{\"fn\": \"cache\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/cache\"}}",
        &method,
        &addr
    );

    if let Some(key) = params.key {
        Json(state.cache_remove(&key).await)
    } else {
        Json(state.cache_clear().await)
    }
}

pub async fn health(
    RequestMethod(method): RequestMethod,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Json<Value> {
    log::debug!(
        "{{\"fn\": \"health\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/health\"}}",
        &method,
        &addr,
    );
    Json(json!({ "msg": "Healthy"}))
}

//pub async fn root(
//    Extension(mut state): Extension<State>,
//    payload: Option<BodyStream>,
//    path: ProxyPath,
//    RequestMethod(method): RequestMethod,
//    all_headers: HeaderMap,
//    RawQuery(query): RawQuery,
//    ConnectInfo(addr): ConnectInfo<SocketAddr>,
//) -> Response<Body> {
//    log::debug!(
//        "{{\"fn\": \"root\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"{}\", \"query\": \"{}\"}}",
//        &method.as_str(),
//        &addr,
//        &path.path(),
//        query.clone().unwrap_or_else(|| "".to_string())
//    );
//    match state
//        .response(method, path, query, all_headers, payload, addr)
//        .await
//    {
//        Ok(p) => p,
//        Err(e) => {
//            log::debug!("No root endpoint found: {}", e);
//            let body = json!({ "version": crate_version!(), "name": crate_name!(), "description": crate_description!()}).to_string();
//            Response::new(body.into())
//        }
//    }
//}

pub async fn echo(
    Json(payload): Json<Value>,
    RequestMethod(method): RequestMethod,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Json<Value> {
    log::debug!(
        "{{\"fn\": \"echo\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/echo\"}}",
        &method,
        &addr,
    );
    Json(payload)
}

//pub async fn help(
//    RequestMethod(method): RequestMethod,
//    ConnectInfo(addr): ConnectInfo<SocketAddr>,
//) -> Json<Value> {
//    log::debug!(
//        "{{\"fn\": \"help\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"/help\"}}",
//        &method,
//        &addr,
//    );
//    let payload = json!({"/cache":{"methods":{"get":"Get proxima cache","delete":"Delete proxima cache"}},"/config":{"methods":{"get":"Get proxima configuration"}},"/echo":{"methods":{"get":"Echo back json payload (debugging)"}},"/health":{"methods":{"get":"Get the health of proxima"}},"/help":{"methods":{"get":"Show this help message"}},"/reload":{"methods":{"get":"Reload proxima's config"}},"/:endpoint":{"methods":{"get":"Show config for specific parent"}},"/:endpoint/*path":{"methods":{"get":"Pass through any request to specified endpoint"}}});
//    Json(payload)
//}

pub async fn handler_404(
    OriginalUri(original_uri): OriginalUri,
    RequestMethod(method): RequestMethod,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let parts = original_uri.into_parts();
    let path_and_query = parts.path_and_query.expect("Missing post path and query");
    log::debug!(
        "{{\"fn\": \"handler_404\", \"method\": \"{}\", \"addr\":\"{}\", \"path\":\"{}\"}}",
        &method,
        &addr,
        &path_and_query,
    );
    StatusCode::NOT_FOUND
}
