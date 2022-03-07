use async_recursion::async_recursion;
use axum::http::Request;
use chrono::Utc;
use hyper::header::{HeaderValue, AUTHORIZATION};
use hyper::{Body, HeaderMap};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

use crate::cache::Cache;
use crate::create_https_client;
use crate::error::Error as RestError;
use crate::https::HttpsClient;
use crate::path::ProxyPath;

type BoxResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub type ConfigMap = BTreeMap<String, Entry>;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct ConfigFile {
    pub static_config: ConfigMap,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub config_file: Arc<RwLock<ConfigFile>>,
    pub location: String,
    pub authentication: Option<EndpointAuth>,
    pub last_read: Arc<RwLock<i64>>,
    pub hash: Arc<RwLock<u64>>,
    pub cache: Cache,
    pub client: HttpsClient,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct BasicAuth {
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct BearerAuth {
    #[serde(skip_serializing)]
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[warn(non_camel_case_types)]
#[serde(untagged)]
pub enum Entry {
    #[allow(non_camel_case_types)]
    ConfigMap(Box<ConfigMap>),
    #[allow(non_camel_case_types)]
    Endpoint(Endpoint),
    #[allow(non_camel_case_types)]
    HttpConfig(HttpConfig),
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct HttpConfig {
    pub remote: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<EndpointAuth>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[warn(non_camel_case_types)]
pub enum EndpointAuth {
    #[allow(non_camel_case_types)]
    basic(BasicAuth),
    #[allow(non_camel_case_types)]
    bearer(BearerAuth),
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct Endpoint {
    pub url: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<EndpointAuth>,
    #[serde(skip_serializing)]
    pub lock: Option<EndpointAuth>,
}

impl<'a> EndpointAuth {
    pub fn header_value(&self) -> HeaderValue {
        match self {
            EndpointAuth::basic(auth) => HeaderValue::from_str(&auth.basic()).unwrap(),
            EndpointAuth::bearer(auth) => HeaderValue::from_str(&auth.token()).unwrap(),
        }
    }

    pub fn authorize(&self, header: &HeaderValue) -> Result<(), RestError> {
        if self.header_value() != header {
            return Err(RestError::UnauthorizedUser);
        }
        Ok(())
    }

    pub fn headers(&self, headers: &'a mut HeaderMap) -> Result<&'a mut HeaderMap, RestError> {
        match self {
            EndpointAuth::basic(auth) => {
                log::debug!("Generating Basic auth headers");
                let basic_auth = auth.basic();
                let header_basic_auth = match HeaderValue::from_str(&basic_auth) {
                    Ok(a) => a,
                    Err(e) => {
                        log::error!("{{\"error\":\"{}\"", e);
                        return Err(RestError::BadUserPasswd);
                    }
                };
                headers.insert(AUTHORIZATION, header_basic_auth);
                Ok(headers)
            }
            EndpointAuth::bearer(auth) => {
                log::debug!("Generating Bearer auth headers");
                let bearer_auth = format!("Bearer {}", auth.token());
                let header_bearer_auth = match HeaderValue::from_str(&bearer_auth) {
                    Ok(a) => a,
                    Err(e) => {
                        log::error!("{{\"error\":\"{}\"", e);
                        return Err(RestError::BadToken);
                    }
                };
                headers.insert(AUTHORIZATION, header_bearer_auth);
                Ok(headers)
            }
        }
    }
}

impl BearerAuth {
    pub fn token(&self) -> String {
        self.token.clone()
    }
}

impl BasicAuth {
    #[allow(dead_code)]
    pub fn username(&self) -> String {
        self.username.clone()
    }

    #[allow(dead_code)]
    pub fn password(&self) -> String {
        self.password.clone()
    }

    pub fn basic(&self) -> String {
        log::debug!("Generating Basic auth");
        let user_pass = format!("{}:{}", self.username, self.password);
        let encoded = base64::encode(user_pass);
        let basic_auth = format!("Basic {}", encoded);
        log::debug!("Using {}", &basic_auth);
        basic_auth
    }
}

impl Config {
    pub async fn config_file(&self) -> ConfigFile {
        self.config_file.read().await.clone()
    }

    pub async fn get_cache(&self) -> Map<String, Value> {
        self.cache.cache().await
    }

    pub async fn clear_cache(&mut self) {
        self.cache.clear().await;
    }

    pub async fn renew(&self) {
        let last_read = self.last_read.read().await;
        let diff = Utc::now().timestamp() - *last_read;
        if diff >= 30 {
            log::debug!("cache has expired, kicking off config reload");
            drop(last_read);

            // Kick off background thread to update config
            let mut me = self.clone();
            tokio::spawn(async move {
                log::debug!("Kicking off background thread to reload config");
                if let Err(e) = me.update().await {
                    log::error!("Error updating config: {}", e);
                }
            });
        } else {
            log::debug!("\"cache has not expired, current age is {} seconds\"", diff);
        }
    }

    pub fn new(location: &str, authentication: Option<EndpointAuth>) -> Config {
        Config {
            config_file: Arc::new(RwLock::new(ConfigFile {
                static_config: BTreeMap::new(),
            })),
            location: location.to_string(),
            authentication,
            last_read: Arc::new(RwLock::new(i64::default())),
            hash: Arc::new(RwLock::new(u64::default())),
            cache: Cache::default(),
            client: create_https_client(60u64).unwrap(),
        }
    }

    #[async_recursion]
    pub async fn get_sub_entry(
        &self,
        map: ConfigMap,
        path: ProxyPath,
    ) -> Result<(Entry, ProxyPath), RestError> {
        let prefix = match path.prefix() {
            Some(pref) => pref,
            None => return Err(RestError::UnknownEndpoint),
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
                                Ok((Entry::ConfigMap(map.clone()), path))
                            }
                        }
                    }
                    Entry::HttpConfig(entry) => {
                        log::debug!("Found http config fork");
                        let config_file = match self
                            .parse(Some(entry.remote.clone()), entry.authentication.clone())
                            .await
                        {
                            Ok(c) => c,
                            Err(e) => {
                                log::error!("Error parsing remote config: {}", e.to_string());
                                return Err(e);
                            }
                        };
                        match path.next() {
                            Some(next) => {
                                log::debug!("Sub folder specified for httpconfig, calling self");
                                self.get_sub_entry(config_file.static_config, next).await
                            }
                            None => {
                                log::debug!("No more subfolders specified for httpconfig, returning httpconfig");
                                Ok((Entry::ConfigMap(Box::new(config_file.static_config)), path))
                            }
                        }
                    }
                    Entry::Endpoint(e) => {
                        log::debug!(
                            "Returning endpoint: {}, with remainder of {}",
                            &e.url,
                            path.suffix().unwrap_or_else(|| "None".to_string())
                        );
                        Ok((Entry::Endpoint(e.clone()), path.next().unwrap_or_default()))
                    }
                }
            }
            None => Err(RestError::UnknownEndpoint),
        }
    }

    pub async fn get(&mut self, path: ProxyPath) -> Result<(Entry, ProxyPath), RestError> {
        self.renew().await;
        log::debug!("Searching for entry {} in cache", &path.path());
        match self.cache.get(&path.path()).await {
            Some((entry, remainder)) => Ok((Entry::Endpoint(entry), remainder)),
            None => {
                log::debug!(
                    "Searching for entry {} in configmap",
                    &path.prefix().unwrap()
                );
                match self
                    .get_sub_entry(self.config_file().await.static_config, path.clone())
                    .await
                {
                    Ok((entry, remainder)) => {
                        if let Entry::Endpoint(ref hit) = entry {
                            self.cache.set(&path.path(), &remainder, hit).await;
                        };
                        Ok((entry, remainder))
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    pub async fn reload(&mut self) -> Value {
        match self.update().await {
            Ok(_) => json!({"msg": "Renewed config"}),
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    pub async fn update(&mut self) -> BoxResult<()> {
        let new_config = self.parse(None, self.authentication.clone()).await?;
        let current_config = self.config_file().await;
        let now = Utc::now().timestamp();
        let new_config_hash = Config::calculate_hash(&new_config);
        let current_config_hash = Config::calculate_hash(&current_config);

        if current_config_hash != new_config_hash {
            log::debug!(
                "Config has been changed, new {} vs old {}",
                &new_config_hash,
                &current_config_hash
            );

            // Get mutable handle on config and config_read
            let mut config_file = self.config_file.write().await;
            let mut hash = self.hash.write().await;
            self.cache.clear().await;

            *config_file = new_config;
            *hash = new_config_hash;
        } else {
            log::debug!("Config has not changed");
        };

        let mut last_read = self.last_read.write().await;
        *last_read = now;

        Ok(())
    }

    pub async fn parse(
        &self,
        remote: Option<Url>,
        authentication: Option<EndpointAuth>,
    ) -> Result<ConfigFile, RestError> {
        let location = match remote {
            Some(url) => url.to_string(),
            None => self.location.clone(),
        };

        // Test if config flag is url
        match url::Url::parse(&location) {
            Ok(url) => {
                log::debug!("config location is url: {}", &url);

                // Create new get request
                let mut req = Request::builder()
                    .method("GET")
                    .uri(url.to_string())
                    .body(Body::empty())
                    .expect("request builder");

                // Add in basic auth if required
                let headers = req.headers_mut();
                if let Some(authentication) = authentication {
                    log::debug!("Inserting auth for config endpoint");
                    authentication.headers(headers)?;
                }

                // Send request
                let response = match self.client.request(req).await {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("{{\"error\":\"{}\"", e);
                        return Err(RestError::Hyper(e));
                    }
                };

                // Error if status code is not 200
                match response.status().as_u16() {
                    404 => Err(RestError::NotFound),
                    403 => Err(RestError::Forbidden),
                    401 => Err(RestError::Unauthorized),
                    200 => {
                        let contents = hyper::body::to_bytes(response.into_body()).await?;
                        let body = serde_json::from_slice(&contents)?;
                        Ok(body)
                    }
                    _ => {
                        log::error!(
                            "Got bad status code getting config: {}",
                            response.status().as_u16()
                        );
                        Err(RestError::Unknown)
                    }
                }
            }
            Err(e) => {
                log::debug!("\"config location {} is not Url: {}\"", &self.location, e);
                let mut file = File::open(self.location.clone()).expect("Unable to open config");
                let mut contents = String::new();

                file.read_to_string(&mut contents)
                    .expect("Unable to read config");

                Ok(serde_yaml::from_str(&contents)?)
            }
        }
    }
}
