use crate::config;
use crate::config::{Config, Entry};
use crate::https::HttpsClient;
use axum::{
    extract::BodyStream,
    http::uri::Uri,
    http::{Request, Response, StatusCode},
};
use clap::ArgMatches;
use hyper::{Body, HeaderMap, Method};
use serde_json::json;
use serde_json::Value;
use std::convert::TryFrom;
use std::error::Error;

use crate::config::{BasicAuth, EndpointAuth};
use crate::create_https_client;
use crate::error::Error as RestError;
use crate::path::ProxyPath;

type BoxResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Debug)]
pub struct State {
    pub config: Config,
    pub client: HttpsClient,
}

impl State {
    pub async fn new(opts: ArgMatches<'_>) -> BoxResult<Self> {
        // Set timeout
        let timeout: u64 = opts
            .value_of("timeout")
            .unwrap()
            .parse()
            .unwrap_or_else(|_| {
                eprintln!("Supplied timeout not in range, defaulting to 60");
                60
            });

        let config_auth = match opts.value_of("config_username") {
            Some(config_username) => {
                log::debug!("Generating Basic auth for config endpoint");
                let config_password = opts.value_of("config_password").unwrap();
                let basic_auth = EndpointAuth::basic(BasicAuth {
                    username: config_username.to_string(),
                    password: config_password.to_string(),
                });
                Some(basic_auth)
            }
            None => None,
        };

        let client = create_https_client(timeout)?;
        let config_location = opts.value_of("config").unwrap().to_owned();
        let mut config = config::Config::new(&config_location, config_auth.clone());
        config.update().await?;

        Ok(State { client, config })
    }

    pub async fn config(&mut self) -> Value {
        let _ = self.config.update().await;
        json!(self.config.config_file().await)
    }

    pub async fn get_cache(&mut self) -> Value {
        json!(self.config.get_cache().await)
    }

    pub async fn clear_cache(&mut self) -> Value {
        self.config.clear_cache().await;
        json!({"msg": "cache has been cleared"})
    }

    pub async fn remove_cache(&mut self, path: ProxyPath) -> Value {
        match self.config.cache.remove(path).await {
            Some(e) => json!({"msg": "entry remove from cache", "entry": e}),
            None => json!({"msg": "entry not found in cache"})
        }
    }

    pub async fn response(
        &mut self,
        method: Method,
        path: ProxyPath,
        query: Option<String>,
        mut request_headers: HeaderMap,
        payload: Option<BodyStream>,
    ) -> Result<Response<Body>, RestError> {
        let (config_entry, path) = match self.config.get(path.clone()).await {
            // If we receive an entry, forward request.
            // If we receive a ConfigMap, return as json to client
            Ok((entry, remainder)) => match entry {
                Entry::Endpoint(endpoint) => {
                    log::debug!(
                        "Found on endpoint {}, with path {}",
                        endpoint.url,
                        remainder.path()
                    );
                    (endpoint, remainder)
                }
                Entry::ConfigMap(map) => {
                    let config = serde_json::to_string(&map).expect("Cannot convert to JSON");
                    let body = Body::from(config);
                    return Ok(Response::builder()
                        .status(StatusCode::OK)
                        .body(body)
                        .unwrap());
                }
                Entry::HttpConfig(map) => {
                    let config = serde_json::to_string(&map).expect("Cannot convert to JSON");
                    let body = Body::from(config);
                    return Ok(Response::builder()
                        .status(StatusCode::OK)
                        .body(body)
                        .unwrap());
                }
            },
            Err(e) => return Err(e),
        };

        // If endpoint is locked down, verify credentials
        if let Some(lock) = config_entry.lock {
            log::debug!("Endpoint is locked");
            match request_headers.get("AUTHORIZATION") {
                Some(header) => lock.authorize(header)?,
                None => return Err(RestError::UnauthorizedUser),
            }
        };

        let host_and_path = match query {
            Some(q) => format!("{}{}?{}", &config_entry.url, path.path(), q),
            None => format!("{}{}", &config_entry.url, path.path()),
        };

        log::debug!("full uri: {}", host_and_path);

        match Uri::try_from(host_and_path) {
            Ok(u) => {
                let body = match payload {
                    Some(p) => {
                        log::debug!("Received body: {:#?}", &p);
                        Body::wrap_stream(p)
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
                request_headers.remove(hyper::header::HOST);
                request_headers.remove(hyper::header::USER_AGENT);
                let headers = req.headers_mut();
                headers.extend(request_headers.clone());

                // Added Basic Auth if username/password exist
                if let Some(authentication) = config_entry.authentication {
                    authentication.headers(headers)?;
                }

                match self.client.clone().request(req).await {
                    Ok(s) => Ok(s),
                    Err(e) => {
                        log::error!("{{\"error\":\"{}\"", e);
                        Err(RestError::Connection)
                    }
                }
            }
            Err(e) => {
                log::error!("{{\"error\": \"{}\"}}", e);
                Err(RestError::UnparseableUrl)
            }
        }
    }
}
