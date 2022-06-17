use async_recursion::async_recursion;
use axum::http::Request;
use chrono::Utc;
use hyper::{Body, Uri};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::prelude::*;
use std::sync::Arc;
use std::fmt;
use tokio::sync::RwLock;
use url::Url;
use vault_client_rs::client::Client as VaultClient;

use crate::auth::server::ServerAuth;
use crate::cache::Cache;
use crate::error::Error as ProximaError;
use crate::https::HttpsClient;
use crate::path::ProxyPath;
use crate::security::{display_security, Security};
use crate::urls::Urls;
use crate::vault::VaultConfig;

type BoxResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub type ConfigMap = BTreeMap<String, Entry>;

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Default)]
pub struct ConfigFile {
    pub static_config: ConfigMap,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub config_file: Arc<RwLock<ConfigFile>>,
    pub location: String,
    pub global_authentication: bool,
    pub config_authentication: Option<ServerAuth>,
    pub last_read: Arc<RwLock<i64>>,
    pub hash: Arc<RwLock<u64>>,
    pub cache: Cache<Endpoint>,
    pub mappings: Cache<String>,
    pub https_client: HttpsClient,
    pub vault_client: Option<VaultClient>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[warn(non_camel_case_types)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum Entry {
    #[allow(non_camel_case_types)]
    ConfigMap(Box<ConfigMap>),
    #[allow(non_camel_case_types)]
    Endpoint(Endpoint),
    #[allow(non_camel_case_types)]
    HttpConfig(HttpConfig),
    #[allow(non_camel_case_types)]
    VaultConfig(VaultConfig),
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct HttpConfig {
    pub remote: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ServerAuth>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub url: Urls,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ServerAuth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "display_security")]
    pub security: Option<Security>,
}

impl fmt::Display for Endpoint {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{}", self.url.to_string())
    }
}


impl<'a> Endpoint {
    pub async fn url(&self) -> String {
        match self.url.path().await {
            "/" => self.url.to_string(),
            _ => {
                log::debug!("Adding / suffix to path");
                let mut path = self.url.to_string();
                path.push('/');
                path
            }
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

    pub async fn get_mappings(&self) -> Map<String, Value> {
        self.mappings.cache().await
    }

    pub async fn clear_cache(&mut self) {
        self.cache.clear().await;
    }

    pub fn vault_client(&self) -> Result<VaultClient, ProximaError> {
        match &self.vault_client {
            Some(c) => Ok(c.clone()),
            None => Err(ProximaError::MissingVaultClient),
        }
    }

    pub async fn renew(&self) {
        let last_read = self.last_read.read().await;
        let diff = Utc::now().timestamp() - *last_read;
        if diff >= 30 {
            log::debug!("cache has expired, kicking off config reload");
            metrics::increment_counter!("proxima_config_renew_attempts_total");
            drop(last_read);

            // Kick off background thread to update config
            let mut me = self.clone();
            tokio::spawn(async move {
                log::debug!("Kicking off background thread to reload config");
                if let Err(e) = me.update().await {
                    log::error!("Error updating config: {}", e);
                    metrics::increment_counter!("proxima_config_renew_failures_total");
                }
            });
        } else {
            log::debug!("\"cache has not expired, current age is {} seconds\"", diff);
        }
    }

    pub fn new(
        location: &str,
        config_authentication: Option<ServerAuth>,
        global_authentication: bool,
        https_client: HttpsClient,
        vault_client: Option<VaultClient>,
    ) -> Config {
        Config {
            config_file: Arc::new(RwLock::new(ConfigFile {
                static_config: BTreeMap::new(),
            })),
            location: location.to_string(),
            global_authentication,
            config_authentication,
            last_read: Arc::new(RwLock::new(i64::default())),
            hash: Arc::new(RwLock::new(u64::default())),
            cache: Cache::new(Some("cache".to_string())),
            mappings: Cache::new(Some("mappings".to_string())),
            https_client,
            vault_client,
        }
    }

    pub async fn cache_get(&self, mut path: ProxyPath) -> Result<(Entry, ProxyPath), ProximaError> {
        let path_str = path.path();
        log::debug!("Starting cache_get for {}", &path_str);
        if let Some(mapping) = self.mappings.get(&path_str).await {
            log::debug!("Found cached mapping for {}", &path_str);

            if let Some(endpoint) = self.cache.get(&mapping).await {
                log::debug!("Found cache entry for {}", &mapping);
                
                // Spin path fowward, so that only the remainder is passed along
                path.forward(&mapping)?;
                return Ok((Entry::Endpoint(endpoint), path))
            }
        }

        let result = self.fetch(path, self.config_file().await.static_config).await?;

        if let (Entry::Endpoint(_), ref path) = result {
            // Set cache and mappings
            log::debug!("Adding {} to mappings", path.path());
            self.mappings.set(&path.path(), &path.key().expect("weird")).await;
        }

        Ok(result)
    }

    #[async_recursion]
    // Fetch should check the cache, then the ConfigMap
    pub async fn fetch(
        &self,
        mut path: ProxyPath,
        config: ConfigMap,
    ) -> Result<(Entry, ProxyPath), ProximaError> {
        // If there are no more hops, return configmap
        if path.next_hop().is_some() {
            path.next()?;
        } else {
            return Ok((Entry::ConfigMap(Box::new(config)), path));
        };

        // Check if cache contains endpoint
        if let Some(key) = path.key() {
            log::debug!("Starting fetch with cache search for {}", &key);
            if let Some(hit) = self.cache.get(&key).await {
                log::debug!("Got cache hit for {}", &key);
                return Ok((Entry::Endpoint(hit), path));
            }
        };

        // If endpoint is not found in cache, check configmap
        match config.get(&path.current()) {
            Some(Entry::ConfigMap(entry)) => {
                log::debug!(
                    "Found ConfigMap at {}",
                    &path.key().unwrap_or_else(|| "None".to_string())
                );

                // Check if cache has the next key
                if let Some(key) = path.next_key() {
                    log::debug!("Searching cache for next hop of {}", &key);
                    if let Some(hit) = self.cache.get(&key).await {
                        log::debug!("Got cache hit for {}", &key);
                        // Move path forward
                        path.next()?;
                        return Ok((Entry::Endpoint(hit), path));
                    }
                };

                self.fetch(path, *entry.clone()).await
            }
            Some(Entry::HttpConfig(entry)) => {
                log::debug!(
                    "Found HttpConfig at {}",
                    &path.key().unwrap_or_else(|| "None".to_string())
                );
                if let Some(key) = path.next_key() {
                    log::debug!("Searching cache for next hop of {}", &key);
                    if let Some(hit) = self.cache.get(&key).await {
                        log::debug!("Got cache hit for {}", &key);
                        // Move path forward
                        path.next()?;

                        return Ok((Entry::Endpoint(hit), path));
                    }
                };

                // Get the http config as config_file
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
                self.fetch(path, config_file.static_config).await
            }
            Some(Entry::VaultConfig(entry)) => {
                log::debug!(
                    "Found VaultConfig at {}",
                    &path.key().unwrap_or_else(|| "None".to_string())
                );
                if let Some(key) = path.next_key() {
                    log::debug!("Searching cache for next hop of {}", &key);
                    if let Some(hit) = self.cache.get(&key).await {
                        log::debug!("Got cache hit for {}", &key);
                        // Move path forward
                        path.next()?;
                        return Ok((Entry::Endpoint(hit), path));
                    }
                };

                // Check to see if there are any other subfolders requested,
                // or else return the full vault config
                match path.next_hop() {
                    Some(h) => {
                        let entry = entry.get(self.vault_client()?, &h).await?;
                        // Move path forward
                        path.next()?;

                        // If vault secret is Endpoint variant, cache endpoint
                        if let Entry::Endpoint(ref e) = entry {
                            self.cache.set(&path.key().expect("odd"), &e).await;
                        };

                        Ok((entry, path))
                    }
                    None => {
                        let configmap = entry.config(self.vault_client()?).await?;
                        Ok((Entry::ConfigMap(Box::new(configmap)), path))
                    }
                }
            }
            Some(Entry::Endpoint(entry)) => {
                log::debug!(
                    "Found Endpoint at {}",
                    &path.key().unwrap_or_else(|| "None".to_string())
                );

                // Save entry into cache
                let key = &path.key().expect("weird");
                log::debug!("Adding {} to cache", key);
                self.cache.set(key, &entry).await;

                // Return endpoint
                Ok((Entry::Endpoint(entry.clone()), path))
            }
            None => Err(ProximaError::UnknownEndpoint),
        }
    }

    pub async fn get(&self, path: ProxyPath) -> Result<(Entry, ProxyPath), ProximaError> {
        self.renew().await;
        self.cache_get(path).await
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
        let new_config = self.parse(None, self.config_authentication.clone()).await?;
        let current_config = self.config_file().await;
        let now = Utc::now().timestamp();
        let new_config_hash = Config::calculate_hash(&new_config);
        let current_config_hash = Config::calculate_hash(&current_config);

        if current_config_hash != new_config_hash {
            log::info!("\"Config has been updated\"");
            log::debug!(
                "\"Config has been changed, new {} vs old {}\"",
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
        config_authentication: Option<ServerAuth>,
    ) -> Result<ConfigFile, ProximaError> {
        let location = match remote {
            Some(url) => url.to_string(),
            None => self.location.clone(),
        };

        // Test if config flag is url
        match url::Url::parse(&location) {
            Ok(url) => {
                log::debug!("config location is url: {}", &url);

                let uri = &location.parse::<Uri>().expect("could not parse url to uri");

                // Create new get request
                let mut req = Request::builder()
                    .method("GET")
                    .uri(location)
                    .body(Body::empty())
                    .expect("request builder");

                // Add in basic auth if required
                let headers = req.headers_mut();
                if let Some(authentication) = config_authentication {
                    log::debug!("Inserting auth for config endpoint");
                    authentication.headers(headers, uri).await?;
                }

                // Send request
                let response = match self.https_client.request(req).await {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("{{\"error\":\"{}\"", e);
                        return Err(ProximaError::Hyper(e));
                    }
                };

                // Error if status code is not 200
                match response.status().as_u16() {
                    404 => Err(ProximaError::NotFound),
                    403 => Err(ProximaError::Forbidden),
                    401 => Err(ProximaError::Unauthorized),
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
                        Err(ProximaError::Unknown)
                    }
                }
            }
            Err(e) => {
                log::debug!("\"config location {} is not Url: {}\"", &self.location, e);
                let mut file = File::open(self.location.clone())?;
                let mut contents = String::new();

                file.read_to_string(&mut contents)?;

                Ok(serde_yaml::from_str(&contents)?)
            }
        }
    }
}
