use crate::config;
use crate::config::{Config, Entry};
use crate::https::HttpsClient;
use async_recursion::async_recursion;
use axum::{
    extract::BodyStream,
    http::uri::Uri,
    http::{Request, Response, StatusCode},
};
use chrono::offset::Utc;
use clap::ArgMatches;
use hyper::{
    header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Body, HeaderMap, Method,
};
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryFrom;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::{ConfigMap, EndpointAuth};
use crate::create_https_client;
use crate::error::Error as RestError;
use crate::path::ProxyPath;
use crate::cache::Cache;

type BoxResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Debug)]
pub struct State {
    pub config_location: String,
    pub config_auth: Option<String>,
    pub config_read: Arc<RwLock<i64>>,
    pub config: Arc<RwLock<Config>>,
    pub client: HttpsClient,
    pub config_hash: Arc<RwLock<u64>>,
    pub config_cache: Cache
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
                let user_pass = format!("{}:{}", config_username, config_password);
                let encoded = base64::encode(user_pass);
                let basic_auth = format!("Basic {}", encoded);
                Some(basic_auth)
            }
            None => None,
        };

        let client = create_https_client(timeout)?;
        let config_location = opts.value_of("config").unwrap().to_owned();
        let config = config::parse(client.clone(), &config_location, config_auth.clone()).await?;
        let config_hash = State::calculate_hash(&config);
        let config_read = Utc::now().timestamp();
        let config_cache = Cache::default();

        Ok(State {
            client,
            config: Arc::new(RwLock::new(config)),
            config_location,
            config_auth,
            config_cache,
            config_read: Arc::new(RwLock::new(config_read)),
            config_hash: Arc::new(RwLock::new(config_hash)),
        })
    }

    #[async_recursion]
    pub async fn get_sub_entry(
        &self,
        map: ConfigMap,
        path: ProxyPath,
    ) -> Option<(Entry, ProxyPath)> {
        let prefix = match path.prefix() {
            Some(pref) => pref,
            None => return None,
        };

        log::debug!(
            "Searching for endpoint: {}, with remainder of {}",
            prefix,
            path.suffix().unwrap_or_else(|| "None".to_string())
        );

        match map.get(&prefix) {
            Some(entry) => {
                match entry {
                    Entry::ConfigMap(map) => {
                        log::debug!("Found configmap fork");

                        match path.next() {
                            Some(next) => {
                                log::debug!("Looks like there is another subfolder specified, calling myself");
                                self.get_sub_entry(*map.clone(), next).await
                            }
                            None => {
                                log::debug!("No more subfolders specified, returning configmap");
                                Some((config::Entry::ConfigMap(map.clone()), path))
                            }
                        }
                    }
                    Entry::Endpoint(e) => {
                        log::debug!(
                            "Returning endpoint: {}, with remainder of {}",
                            &e.url,
                            path.suffix().unwrap_or_else(|| "None".to_string())
                        );
                        Some((
                            config::Entry::Endpoint(e.clone()),
                            path.next().unwrap_or_default(),
                        ))
                    }
                }
            }
            None => None,
        }
    }

    pub async fn get(&mut self, path: ProxyPath) -> Option<(Entry, ProxyPath)> {
        self.renew().await;
        log::debug!("Searching for entry {} in cache", &path.path());
        match self.config_cache.get(&path.path()).await {
            Some((entry, remainder)) => Some((config::Entry::Endpoint(entry), remainder)),
            None => {
                log::debug!("Searching for entry {} in configmap", &path.prefix().unwrap());
                let config = self.config.read().await;
                match self.get_sub_entry(config.static_config.clone(), path.clone()).await {
                    Some((entry,remainder)) => {
                        if let Entry::Endpoint(ref hit) = entry {
                            self.config_cache.set(&path.path(), &remainder, &hit).await;
                        };
                        Some((entry,remainder))
                    },
                    None => None
                }
            }
        }
    }

    pub async fn config(&mut self) -> Value {
        self.renew().await;
        let config = self.config.read().await;
        serde_json::to_value(&*config).expect("Cannot convert to JSON")
    }

    pub async fn cache(&mut self) -> Value {
        let cache = self.config_cache.cache().await;
        serde_json::to_value(&cache).expect("Cannot convert to JSON")
    }

    pub async fn renew(&mut self) {
        let config_read = self.config_read.read().await;
        let diff = Utc::now().timestamp() - *config_read;
        if diff >= 30 {
            log::debug!("cache has expired, kicking off config reload");
            drop(config_read);

            // Kick off background thread to update config
            let mut me = self.clone();
            tokio::spawn(async move {
                log::debug!("Kicking off background thread to reload config");
                me.reload().await;
            });
        } else {
            log::debug!("\"cache has not expired, current age is {} seconds\"", diff);
        }
    }

    pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    pub async fn reload(&mut self) {
        let config = self.config.read().await;
        let new_config = match config::parse(
            self.client.clone(),
            &self.config_location,
            self.config_auth.clone(),
        )
        .await
        {
            Ok(e) => e,
            Err(e) => {
                log::error!("{{\"error\": \"Could not parse config: {}\"}}", e);
                config.clone()
            }
        };

        let config_read_time = Utc::now().timestamp();
        let config_hash_new = State::calculate_hash(&new_config);
        let config_hash = self.config_hash.read().await;

        if config_hash_new != *config_hash {
            log::debug!(
                "Config has been changed, new {} vs old {}",
                &config_hash_new,
                &config_hash
            );
            drop(config);
            drop(config_hash);
            // Get mutable handle on config and config_read
            let mut config = self.config.write().await;
            let mut config_read = self.config_read.write().await;
            let mut config_hash = self.config_hash.write().await;
            self.config_cache.clear().await;

            *config = new_config;
            *config_read = config_read_time;
            *config_hash = config_hash_new;
        } else {
            log::debug!("Config has not changed");
        }
    }

    pub async fn response(
        &mut self,
        method: Method,
        path: ProxyPath,
        query: Option<String>,
        mut all_headers: HeaderMap,
        payload: Option<BodyStream>,
    ) -> Result<Response<Body>, RestError> {
        let (config_entry, path) = match self.get(path.clone()).await {
            Some((entry, remainder)) => match entry {
                Entry::Endpoint(endpoint) => {
                    log::debug!(
                        "Passing on endpoint {}, with path {}",
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
            },
            None => return Err(RestError::UnknownEndpoint),
        };

        let host_and_path = match query {
            Some(q) => format!("{}/{}?{}", &config_entry.url(), path.path(), q),
            None => format!("{}/{}", &config_entry.url(), path.path()),
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
                all_headers.remove(hyper::header::HOST);
                all_headers.remove(hyper::header::USER_AGENT);
                if !all_headers.contains_key(CONTENT_TYPE) {
                    log::debug!("\"Adding content-type: application/json\"");
                    all_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                };
                let headers = req.headers_mut();
                headers.extend(all_headers.clone());

                // Added Basic Auth if username/password exist
                match config_entry.authentication {
                    Some(authentication) => match authentication {
                        EndpointAuth::basic(auth) => {
                            let basic_auth = auth.basic();
                            let header_basic_auth = match HeaderValue::from_str(&basic_auth) {
                                Ok(a) => a,
                                Err(e) => {
                                    log::error!("{{\"error\":\"{}\"", e);
                                    return Err(RestError::BadUserPasswd);
                                }
                            };
                            headers.insert(AUTHORIZATION, header_basic_auth);
                        }
                        EndpointAuth::bearer(auth) => {
                            log::debug!("Generating Bearer auth");
                            let basic_auth = format!("Bearer {}", auth.token());
                            let header_bearer_auth = match HeaderValue::from_str(&basic_auth) {
                                Ok(a) => a,
                                Err(e) => {
                                    log::error!("{{\"error\":\"{}\"", e);
                                    return Err(RestError::BadToken);
                                }
                            };
                            headers.insert(AUTHORIZATION, header_bearer_auth);
                        }
                    },
                    None => log::debug!("No authentication specified for endpoint"),
                };

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
