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
use tokio::sync::RwLock;

use crate::State;

type SharedState = Arc<RwLock<State>>;

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
                    Some((left,_)) => left,
                    None => return Ok(Self("".to_string()))
                };

                let user_pass = match left.split_once(r#"://"#) {
                    Some((_,right)) => right,
                    None => return Ok(Self("".to_string()))
                };

                user_pass
            },
            None => ""
        };

        Ok(Self(user_pass.to_string()))
    }
}

pub async fn pass_through(
    Extension(state): Extension<SharedState>,
    payload: Option<Json<Value>>,
    Path((endpoint, path)): Path<(String, String)>,
    RequestMethod(method): RequestMethod,
    all_headers: HeaderMap,
    RawQuery(query): RawQuery
) -> Response<Body> {
    let me = state.read().await;
    log::info!(
        "{{\"fn\": \"pass_through\", \"method\": \"{}\", \"endpoint\":\"{}\",\"uri\":\"{}\"}}",
        &method.as_str(),
        &endpoint,
        &path
    );
    me 
        .response(method, &endpoint, &path, query, all_headers, payload)
        .await
}

pub async fn get_endpoint(Path(endpoint): Path<String>, Extension(state): Extension<SharedState>) -> Json<Value> {
    log::info!("\"GET /{}\"", endpoint);
    let me = state.read().await;
	match me.get_entry(&endpoint).await {
		Some(e) => Json(json!(e)),
		None => Json(json!({"error": "unknown endpoint"}))
	}
}

pub async fn reload(Extension(state): Extension<SharedState>) {
    log::info!("\"GET /reload\"");
    let mut me = state.write().await;
	me.reload().await;
}

pub async fn config(Extension(state): Extension<SharedState>) -> Json<Value> {
    log::info!("\"GET /\"");
    let me = state.read().await;
	Json(me.config().await)
}

pub async fn health() -> Json<Value> {
    log::info!("\"GET /health\"");
    Json(json!({ "msg": "Healthy"}))
}

pub async fn echo(Json(payload): Json<Value>) -> Json<Value> {
    log::info!("\"POST /echo\"");
    Json(payload)
}

pub async fn help() -> Json<Value> {
    log::info!("\"GET /help\"");
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
    log::info!("\"Bad path: {}\"", path_and_query);
    (
        StatusCode::NOT_FOUND,
        "{\"error_code\": 404, \"message\": \"HTTP 404 Not Found\"}",
    )
}
