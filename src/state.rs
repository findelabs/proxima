use axum::{
    extract::BodyStream,
    http::{Response, StatusCode},
};
use clap::ArgMatches;
use hyper::{Body, HeaderMap, Method};
use serde_json::json;
use serde_json::Value;
use std::error::Error;

use crate::auth::{basic::BasicAuth, server::ServerAuth};
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

// Let's have this instead create client and vault_client, and add config at a later point
impl Default for State {
    fn default() -> Self {
        State {
            client: HttpsClient::default(),
            config: Config::default(),
        }
    }
}

impl State {
    pub async fn build(&mut self, opts: ArgMatches) -> BoxResult<()> {
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

        let vault_client = match opts.is_present("vault_url") {
            true => {
                //                // This is required in order to set jwt_path to None if the flag was not passed
                //                let jwt_path = match opts.occurrences_of("jwt_path") {
                //                    0 => None,
                //                    _ => opts.value_of("jwt_path")
                //                };
                let mut client = vault_client_rs::client::ClientBuilder::new()
                    .with_mount(opts.value_of("vault_mount").unwrap())
                    .with_url(opts.value_of("vault_url").unwrap())
                    .with_login_path(opts.value_of("vault_login_path").unwrap())
                    .with_kubernetes_role(opts.value_of("vault_kubernetes_role"))
                    .with_role_id(opts.value_of("vault_role_id"))
                    .with_secret_id(opts.value_of("vault_secret_id"))
                    .with_jwt_path(opts.value_of("jwt_path"))
                    .insecure(opts.is_present("insecure"))
                    .build()
                    .unwrap();

                // Ensure we can login to vault
                match client.login().await {
                    Ok(_) => Some(client),
                    Err(_) => panic!("Failed logging in to vault"),
                }
            }
            false => None,
        };

        let client = ClientBuilder::new()
            .timeout(timeout)
            .nodelay(opts.is_present("set_nodelay"))
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
            client.clone(),
            vault_client,
        );
        config.update().await?;

        self.client = client;
        self.config = config;

        Ok(())
    }

    pub async fn config(&mut self) -> Value {
        let _ = self.config.update().await;
        json!(self.config.config_file().await)
    }

    pub async fn cache_get(&mut self) -> Value {
        json!(self.config.get_cache().await)
    }

    pub async fn mappings_get(&mut self) -> Value {
        json!(self.config.get_mappings().await)
    }

    pub async fn cache_clear(&mut self) -> Value {
        self.config.clear_cache().await;
        json!({"msg": "cache has been cleared"})
    }

    pub async fn cache_remove(&mut self, path: &str) -> Value {
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
                        log::error!(
                            "Endpoint is locked, but proxima is using global authentication"
                        );
                    }
                    false => client.authorize(headers, method).await?
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
                        remainder.suffix()
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
                Entry::VaultConfig(map) => {
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
