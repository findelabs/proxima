use crate::error::Error as RestError;
use crate::https::HttpsClient;
use async_recursion::async_recursion;
use axum::http::Request;
use chrono::Utc;
use hyper::header::{HeaderValue, AUTHORIZATION};
use hyper::Body;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use serde_json::{Map, Value};
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fs::File;
use std::hash::Hash;
use std::hash::Hasher;
use std::io::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

use crate::cache::Cache;
use crate::create_https_client;
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
    pub authentication: Option<String>,
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
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[warn(non_camel_case_types)]
pub enum EndpointAuth {
    #[allow(non_camel_case_types)]
    basic(BasicAuth),
    #[allow(non_camel_case_types)]
    bearer(BearerAuth),
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
#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct Endpoint {
    pub url: Url,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<EndpointAuth>,
}

#[allow(dead_code)]
fn hide_string<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    let hidden: String = s
        .chars()
        .enumerate()
        .filter(|(i, _)| i < &16)
        .map(|_| '*')
        .collect();
    Ok(hidden)
}

// This can probably go away
impl Endpoint {
    pub fn url(&self) -> String {
        // Clean up url, so that there are no trailing forward slashes
        match self.url.to_string().chars().last() {
            Some('/') => {
                let mut url = self.url.to_string();
                url.pop();
                url
            }
            _ => self.url.to_string(),
        }
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

    pub fn new(location: &str, authentication: Option<String>) -> Config {
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
                                Some((Entry::ConfigMap(map.clone()), path))
                            }
                        }
                    }
                    Entry::Endpoint(e) => {
                        log::debug!(
                            "Returning endpoint: {}, with remainder of {}",
                            &e.url,
                            path.suffix().unwrap_or_else(|| "None".to_string())
                        );
                        Some((Entry::Endpoint(e.clone()), path.next().unwrap_or_default()))
                    }
                }
            }
            None => None,
        }
    }

    pub async fn get(&mut self, path: ProxyPath) -> Option<(Entry, ProxyPath)> {
        self.renew().await;
        log::debug!("Searching for entry {} in cache", &path.path());
        match self.cache.get(&path.path()).await {
            Some((entry, remainder)) => Some((Entry::Endpoint(entry), remainder)),
            None => {
                log::debug!(
                    "Searching for entry {} in configmap",
                    &path.prefix().unwrap()
                );
                match self
                    .get_sub_entry(self.config_file().await.static_config, path.clone())
                    .await
                {
                    Some((entry, remainder)) => {
                        if let Entry::Endpoint(ref hit) = entry {
                            self.cache.set(&path.path(), &remainder, hit).await;
                        };
                        Some((entry, remainder))
                    }
                    None => None,
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
        let new_config = self.parse().await?;
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
            let mut last_read = self.last_read.write().await;
            let mut hash = self.hash.write().await;
            self.cache.clear().await;

            *config_file = new_config;
            *last_read = now;
            *hash = new_config_hash;
        } else {
            log::debug!("Config has not changed");
        };
        Ok(())
    }

    pub async fn parse(&mut self) -> BoxResult<ConfigFile> {
        // Test if config flag is url
        match url::Url::parse(&self.location) {
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
                if self.authentication.is_some() {
                    log::debug!("Inserting basic auth for config endpoint");
                    let header_basic_auth =
                        HeaderValue::from_str(self.authentication.as_ref().unwrap())?;
                    headers.insert(AUTHORIZATION, header_basic_auth);
                };

                // Send request
                let response = match self.client.request(req).await {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("{{\"error\":\"{}\"", e);
                        return Err(Box::new(e));
                    }
                };

                // Error if status code is not 200
                match response.status().as_u16() {
                    404 => Err(Box::new(RestError::NotFound)),
                    403 => Err(Box::new(RestError::Forbidden)),
                    401 => Err(Box::new(RestError::Unauthorized)),
                    200 => {
                        let contents = hyper::body::to_bytes(response.into_body()).await?;
                        Ok(serde_json::from_slice(&contents)?)
                    }
                    _ => Err(Box::new(RestError::Unknown)),
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
