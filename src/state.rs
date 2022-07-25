use axum::{
    extract::BodyStream,
    http::{Response, StatusCode},
};
use clap::ArgMatches;
use hyper::header::FORWARDED;
use hyper::{Body, HeaderMap, Method};
use serde_json::json;
use serde_json::Value;
use std::error::Error;
use std::net::SocketAddr;

use crate::auth::{basic::BasicAuth, server::ServerAuth};
use crate::config;
use crate::config::{Config, Endpoint, Route};
use crate::error::Error as ProximaError;
use crate::https::{ClientBuilder, HttpsClient};
use crate::path::ProxyPath;
use crate::requests::ProxyRequest;
//use crate::auth::traits::Authorize;
use crate::security::EndpointSecurity;

type BoxResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Default, Clone, Debug)]
pub struct State {
    pub config: Config,
    pub client: HttpsClient,
}

// Let's have this instead create client and vault_client, and add config at a later point
//impl Default for State {
//    fn default() -> Self {
//        State {
//            client: HttpsClient::default(),
//            config: Config::default(),
//        }
//    }
//}

impl State {
    // This function is needed to establish the global HttpsClient used by both config to
    // fetch remote configs, as well as the main proxy threads
    pub async fn basic(opts: ArgMatches) -> Self {
        let client = ClientBuilder::new()
            .accept_invalid_hostnames(opts.is_present("insecure"))
            .accept_invalid_certs(opts.is_present("insecure"))
            .build().unwrap();

        State {
            client,
            config: Config::default(),
        }
    }

    pub async fn build(&mut self, opts: ArgMatches) -> BoxResult<()> {

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

        let config_location = opts.value_of("config").unwrap().to_owned();
        let mut config = config::Config::new(
            &config_location,
            config_auth.clone(),
            opts.is_present("username"),
            self.client.clone(),
            vault_client,
        );

        // Get config from file or remote source
        config.update().await?;

        // Update config
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

    //    pub async fn authorize_whitelist<T: EndpointSecurity> (
    //        &mut self,
    //        endpoint: &T,
    //        method: &Method,
    //        client_addr: &SocketAddr,
    //    ) -> Result<(), ProximaError> {
    //        // If endpoint has a method whitelock, verify
    //        if let Some(ref security) = endpoint.security() {
    //            if let Some(ref whitelist) = security.whitelist {
    //                log::debug!("Found whitelist");
    //                whitelist.authorize(method, client_addr)?
    //            }
    //        }
    //        Ok(())
    //    }
    //
    //    pub async fn authenticate_client<T: EndpointSecurity> (
    //        &mut self,
    //        endpoint: &Proxy,
    //        headers: &HeaderMap,
    //        method: &Method,
    //        client_addr: &SocketAddr,
    //    ) -> Result<(), ProximaError> {
    //        // If endpoint is locked down, verify credentials
    //        if let Some(ref security) = endpoint.security {
    //            if let Some(ref clientlist) = security.client {
    //                log::debug!("Proxy is locked");
    //                match self.config.global_authentication {
    //                    true => {
    //                        log::error!("Proxy is locked, but proxima is using global authentication");
    //                    }
    //                    false => clientlist.authorize(headers, method, client_addr).await?,
    //                }
    //            }
    //        }
    //        Ok(())
    //    }

    pub async fn response(
        &mut self,
        method: Method,
        path: ProxyPath,
        query: Option<String>,
        request_headers: HeaderMap,
        payload: Option<BodyStream>,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>, ProximaError> {
        // Check if path exists in config
        match self.config.get(path.clone()).await {
            // Looks like we found a match
            Ok((route, remainder)) => match route {
                // Return these variants without checking for security
                Route::ConfigMap(map) => {
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from(
                            serde_json::to_string(&map).expect("Cannot convert to JSON"),
                        ))
                        .unwrap())
                }
                Route::Endpoint(entry) => {
                    // Detect client IP
                    let client = if let Some(x_forwarded) = &request_headers.get("x-forwarded-for")
                    {
                        match x_forwarded.to_str() {
                            Ok(s) => s.parse().unwrap_or(client_addr),
                            Err(e) => {
                                log::error!("Unable to parse x-forwarded-for header: {}", e);
                                client_addr
                            }
                        }
                    } else if let Some(forwarded) = &request_headers.get(FORWARDED) {
                        match forwarded.to_str() {
                            Ok(s) => s.parse().unwrap_or(client_addr),
                            Err(e) => {
                                log::error!("Unable to parse forwarded header: {}", e);
                                client_addr
                            }
                        }
                    } else {
                        client_addr
                    };

                    // Debug client addr
                    log::debug!("Client socket determined to be {}", &client);

                    match entry {
                        Endpoint::HttpConfig(map) => {
                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .body(Body::from(
                                    serde_json::to_string(&map).expect("Cannot convert to JSON"),
                                ))
                                .unwrap())
                        }
                        Endpoint::Vault(map) => {
                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .body(Body::from(
                                    serde_json::to_string(&map).expect("Cannot convert to JSON"),
                                ))
                                .unwrap())
                        }
                        Endpoint::Proxy(endpoint) => {
                            log::debug!(
                                "Found an endpoint {}, with path {}",
                                endpoint.url.path().await,
                                remainder.suffix()
                            );

                            // Authorize client, and check for client whitelist
                            endpoint.auth(&request_headers, &method, &client).await?;

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
                                endpoint,
                                method,
                                path,
                                body,
                                request_headers,
                                query,
                            };
                            request.go().await
                        }
                        Endpoint::Static(endpoint) => {
                            log::debug!("Found static entry");

                            // Authorize client, and check for client whitelist
                            endpoint.auth(&request_headers, &method, &client).await?;

                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .body(Body::from(endpoint.body))
                                .unwrap())
                        }
                        Endpoint::Redirect(endpoint) => {
                            log::debug!("Found redirect entry");

                            Ok(Response::builder()
                                .status(StatusCode::PERMANENT_REDIRECT)
                                .header("LOCATION", endpoint.url.to_string())
                                .body(Body::empty())
                                .unwrap())
                        }
                    }
                }
            },
            Err(e) => Err(e),
        }
    }
}
