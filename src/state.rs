use axum::{
    http::{uri::Uri, StatusCode},
    http::{Request, Response},
    Json,
};
use hyper::{
    header::{HeaderValue, CONTENT_TYPE},
    Body, HeaderMap, Method,
};
use serde_json::Value;
use std::convert::TryFrom;
use crate::config::{ConfigEntry, ConfigHash};
use crate::https::HttpsClient;
use crate::config;

pub struct State {
    pub config_path: String,
    pub config: ConfigHash,
    pub client: HttpsClient,
}

impl State {
    pub async fn get_entry(&self, item: &str) -> Option<ConfigEntry> {
        log::debug!("Getting {} from ConfigHash", &item);
        let entry = self.config.get(item);
        entry.cloned()
    }

    pub async fn config(&self) -> Value {
        serde_json::to_value(&self.config).expect("Cannot convert to JSON")
    }

    pub async fn reload(&mut self) {
        let config = match config::parse(&self.config_path) {
            Ok(e) => e,
            Err(e) => {
                log::error!("Could not parse config: {}", e);
                self.config.clone()
            }
        };
        self.config = config;
    }

    pub async fn response(
        &self,
        method: Method,
        endpoint: &str,
        path: &str,
        query: Option<String>,
        mut all_headers: HeaderMap,
        payload: Option<Json<Value>>,
    ) -> Response<Body> {

        let config_entry = match self.get_entry(endpoint).await {
            Some(e) => e,
            None => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("{\"error\": \"please specify known endpoint\"}"))
                    .unwrap()
            }
        };

        let path = path.replace(" ", "%20");
        let host_and_path = match query {
            Some(q) => format!("{}/{}?{}", config_entry.url, path, q),
            None => format!("{}/{}", config_entry.url, path)
        };
        log::debug!("full uri: {}", host_and_path);

        match Uri::try_from(host_and_path) {
            Ok(u) => {
                let body = match payload {
                    Some(p) => {
                        log::debug!("Received body: {:?}", &p);
                        Body::from(p.to_string())
                    }
                    None => {
                        log::debug!("Did not receive a body");
                        Body::empty()
                    }
                };

                let mut req = Request::builder()
                    .method(method)
                    .uri(u)
                    .body(body)
                    .expect("request builder");

                // Append to request the headers passed by client
                all_headers.remove(hyper::header::HOST);
                all_headers.remove(hyper::header::USER_AGENT);
                if !all_headers.contains_key(CONTENT_TYPE) {
                    all_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                };
                let headers = req.headers_mut();
                headers.extend(all_headers.clone());

                match self.client.request(req).await {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("{}", e);
                        Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from(
                                "{\"error\": \"Error connecting to rest endpoint\"}",
                            ))
                            .unwrap()
                    }
                }
            }
            Err(_) => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("{\"error\": \"Error parsing uri\"}"))
                .unwrap(),
        }
    }
}
