use axum::{
    extract::BodyStream,
    http::{Response, StatusCode},
};
use clap::ArgMatches;
use hyper::{Body, HeaderMap, Method};
use serde_json::json;
use serde_json::Value;
use std::error::Error;

use crate::auth::{auth::BasicAuth, server::ServerAuth};
use crate::config;
use crate::config::{Config, Endpoint, Entry};
use crate::error::Error as ProximaError;
use crate::https::{ClientBuilder, HttpsClient};
use crate::path::ProxyPath;
use crate::requests::ProxyRequest;

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
                let basic_auth = ServerAuth::basic(BasicAuth {
                    username: config_username.to_string(),
                    password: config_password.to_string(),
                    whitelist: None,
                });
                Some(basic_auth)
            }
            None => None,
        };

        let client = ClientBuilder::new()
            .timeout(timeout)
            .nodelay(opts.is_present("nodelay"))
            .enforce_http(opts.is_present("enforce_http"))
            .reuse_address(opts.is_present("set_reuse_address"))
            .accept_invalid_hostnames(opts.is_present("accept_invalid_hostnames"))
            .accept_invalid_certs(opts.is_present("accept_invalid_certs"))
            .import_cert(opts.value_of("import_cert"))
            .build()?;

        let config_location = opts.value_of("config").unwrap().to_owned();
        let mut config = config::Config::new(
            &config_location,
            config_auth.clone(),
            opts.is_present("username"),
        );
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
            None => json!({"msg": "entry not found in cache"}),
        }
    }

    pub async fn whitelist(
        &mut self,
        endpoint: &Endpoint,
        method: &Method,
    ) -> Result<(), ProximaError> {
        // If endpoint has a method whitelock, verify
        if let Some(ref security) = endpoint.security {
            if let Some(ref whitelist) = security.whitelist {
                log::debug!("Found whitelist");
                whitelist.authorize(method)?
            }
        }
        Ok(())
    }

    pub async fn authorize_client(
        &mut self,
        endpoint: &Endpoint,
        headers: &HeaderMap,
        method: &Method,
    ) -> Result<(), ProximaError> {
        // If endpoint is locked down, verify credentials
        if let Some(ref security) = endpoint.security {
            if let Some(ref client) = security.client {
                log::debug!("Endpoint is locked");
                match self.config.global_authentication {
                    true => {
                        log::info!(
                            "Endpoint is locked, but proxima is using global authentication"
                        );
                    }
                    false => match headers.get("AUTHORIZATION") {
                        Some(header) => client.authorize(header, method).await?,
                        None => return Err(ProximaError::UnauthorizedUser),
                    },
                }
            }
        }
        Ok(())
    }

    pub async fn response(
        &mut self,
        method: Method,
        path: ProxyPath,
        query: Option<String>,
        request_headers: HeaderMap,
        payload: Option<BodyStream>,
    ) -> Result<Response<Body>, ProximaError> {
        let (config_entry, path) = match self.config.get(path.clone()).await {
            // If we receive an entry, forward request.
            // If we receive a ConfigMap, return as json to client
            Ok((entry, remainder)) => match entry {
                Entry::Endpoint(endpoint) => {
                    log::debug!(
                        "Found an endpoint {}, with path {}",
                        endpoint.url().await,
                        remainder.path().unwrap_or("None")
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

        // Authorize clients
        self.authorize_client(&config_entry, &request_headers, &method)
            .await?;

        // Verify Whitelists
        self.whitelist(&config_entry, &method).await?;

        // Wrap Body if there is one
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

        let request = ProxyRequest {
            client: self.client.clone(),
            endpoint: config_entry,
            method,
            path,
            body,
            request_headers,
            query,
        };
        request.go().await
    }
}
