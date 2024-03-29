use async_recursion::async_recursion;
use axum::http::Request;
use chrono::Utc;
use clap::{crate_description, crate_name, crate_version};
use hyper::header::{HeaderName, HeaderValue};
use hyper::HeaderMap;
use hyper::{Body, Uri};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::RwLock;
use url::Url;
use vault_client_rs::client::Client as VaultClient;

use crate::auth::server::ServerAuth;
use crate::cache::Cache;
use crate::config_global::GlobalConfig;
use crate::error::Error as ProximaError;
use crate::https::ClientBuilder;
use crate::https::HttpsClient;
use crate::path::ProxyPath;
use crate::security::EndpointSecurity;
use crate::security::{display_security, Security};
use crate::urls::Urls;
use crate::vault::Vault;

type BoxResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub type ConfigMap = BTreeMap<String, Route>;

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Default)]
pub struct ConfigFile {
    #[serde(default)]
    pub global: GlobalConfig,
    pub routes: ConfigMap,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub config_file: Arc<RwLock<ConfigFile>>,
    pub location: String,
    pub config_authentication: Option<ServerAuth>,
    pub last_read: Arc<RwLock<i64>>,
    pub refresh_lock: RefreshLock,
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
pub enum Route {
    #[allow(non_camel_case_types)]
    ConfigMap(Box<ConfigMap>),
    #[allow(non_camel_case_types)]
    Endpoint(Endpoint),
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct HttpConfig {
    pub url: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ServerAuth>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct Static {
    pub body: String,
    #[serde(skip_serializing_if = "display_security")]
    pub security: Option<Security>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct Proxy {
    pub url: Urls,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ServerAuth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "display_security")]
    pub security: Option<Security>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ProxyConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct ProxyConfig {
    #[serde(default)]
    pub preserve_host_header: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct Redirect {
    pub url: Url,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum Endpoint {
    #[allow(non_camel_case_types)]
    HttpConfig(HttpConfig),
    #[allow(non_camel_case_types)]
    Vault(Vault),
    #[allow(non_camel_case_types)]
    Proxy(Proxy),
    #[allow(non_camel_case_types)]
    Static(Static),
    #[allow(non_camel_case_types)]
    Redirect(Redirect),
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RefreshLock {
    pub lock: Arc<Mutex<bool>>,
}

impl Hash for RefreshLock {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Do not change the hash ever
        "".hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct Headers(Vec<Header>);

impl EndpointSecurity for Proxy {
    fn security(&self) -> Option<&Security> {
        self.security.as_ref()
    }
}

impl Default for RefreshLock {
    fn default() -> Self {
        RefreshLock {
            lock: Arc::new(Mutex::new(true)),
        }
    }
}

impl RefreshLock {
    pub fn acquire(&self) -> Result<(), ProximaError> {
        match self.lock.try_lock() {
            Ok(_) => Ok(()),
            Err(e) => {
                log::debug!("\"Not able to acquire refresh lock: {}\"", e);
                Err(ProximaError::RefreshLock)
            }
        }
    }
}

impl EndpointSecurity for Static {
    fn security(&self) -> Option<&Security> {
        self.security.as_ref()
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Endpoint::Proxy(p) => write!(f, "{}", p),
            Endpoint::Static(p) => write!(f, "{}", p),
            Endpoint::HttpConfig(p) => write!(f, "{}", p),
            Endpoint::Vault(p) => write!(f, "{}", p),
            Endpoint::Redirect(p) => write!(f, "{}", p),
        }
    }
}

impl fmt::Display for HttpConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "http_config={}", self.url)
    }
}

impl fmt::Display for Vault {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "vault={}", self.secret)
    }
}

impl fmt::Display for Proxy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "proxy={}", self.url)
    }
}

impl fmt::Display for Redirect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "redirect={}", self.url)
    }
}

impl fmt::Display for Static {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "static={}", self.body)
    }
}

impl Headers {
    pub fn insert_headers(&self, map: &mut HeaderMap) -> Result<(), ProximaError> {
        for header in &self.0 {
            log::debug!("Inserting {} into map", header.name);
            let name = HeaderName::from_lowercase(header.name.to_lowercase().as_bytes())?;
            let value = HeaderValue::from_str(&header.value)?;
            map.insert(name, value);
        }
        Ok(())
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
            // Only allow one config refresh to be active at once
            if let Ok(()) = self.refresh_lock.acquire() {
                log::debug!("\"Cache has expired, kicking off config reload\"");
                metrics::increment_counter!("proxima_config_renew_attempts_total");
                drop(last_read);

                // Kick off background thread to update config
                let mut me = self.clone();
                tokio::spawn(async move {
                    log::debug!("\"Kicking off background thread to reload config\"");
                    if let Err(e) = me.update().await {
                        log::error!("Error updating config: {}", e);
                        metrics::increment_counter!("proxima_config_renew_failures_total");
                    }
                });
            } else {
                log::debug!("\"Active refresh found, deferring\"");
            }
        } else {
            log::debug!("\"cache has not expired, current age is {} seconds\"", diff);
        }
    }

    #[allow(dead_code)]
    pub async fn create_https_client(&self) -> Result<HttpsClient, ProximaError> {
        let config = self.config_file().await;
        let client = ClientBuilder::new()
            .timeout(config.global.network.timeout.value())
            .nodelay(config.global.network.nodelay)
            .enforce_http(config.global.network.enforce_http)
            .reuse_address(config.global.network.reuse_address)
            .accept_invalid_hostnames(config.global.security.tls.accept_invalid_hostnames)
            .accept_invalid_certs(config.global.security.tls.insecure)
            .import_cert(config.global.security.tls.import_cert.as_deref())
            .build()?;

        Ok(client)
    }

    pub fn new(
        location: &str,
        config_authentication: Option<ServerAuth>,
        https_client: HttpsClient,
        vault_client: Option<VaultClient>,
    ) -> Config {
        Config {
            config_file: Arc::new(RwLock::new(ConfigFile {
                global: GlobalConfig::default(),
                routes: BTreeMap::new(),
            })),
            location: location.to_string(),
            config_authentication,
            last_read: Arc::new(RwLock::new(i64::default())),
            refresh_lock: RefreshLock::default(),
            hash: Arc::new(RwLock::new(u64::default())),
            cache: Cache::new(Some("cache".to_string())),
            mappings: Cache::new(Some("mappings".to_string())),
            https_client,
            vault_client,
        }
    }

    pub async fn cache_get(
        &mut self,
        mut path: ProxyPath,
    ) -> Result<(Route, ProxyPath), ProximaError> {
        let path_str = path.path();
        log::debug!("\"Starting cache_get for {}\"", &path_str);
        if let Some(mapping) = self.mappings.get(path_str).await {
            log::debug!("\"Found cached mapping for {}\"", &path_str);

            if let Some(endpoint) = self.cache.get(&mapping).await {
                log::debug!("Found cache entry for {}", &mapping);

                // Spin path fowward, so that only the remainder is passed along
                path.forward(&mapping)?;
                return Ok((Route::Endpoint(endpoint), path));
            }
        }

        // If path is just root, if there is no / root set, return default
        // This needs to be checked here, before we enter into recursion
        if path_str == "/" {
            match self
                .fetch(path.clone(), self.config_file().await.routes)
                .await
            {
                Ok((r, v)) => return Ok((r, v)),
                _ => {
                    let body = json!({ "version": crate_version!(), "name": crate_name!(), "description": crate_description!()}).to_string();
                    let stat = Static {
                        body,
                        security: None,
                        headers: None,
                    };
                    return Ok((Route::Endpoint(Endpoint::Static(stat)), path));
                }
            }
        }

        let result = self.fetch(path, self.config_file().await.routes).await?;

        if let (Route::Endpoint(_), ref path) = result {
            // Set cache and mappings
            log::debug!("Adding {} to mappings", path.path());
            self.mappings
                .set(path.path(), &path.key().expect("weird"))
                .await;
        }

        Ok(result)
    }

    #[async_recursion]
    // Fetch should check the cache, then the ConfigMap
    pub async fn fetch(
        &mut self,
        mut path: ProxyPath,
        config: ConfigMap,
    ) -> Result<(Route, ProxyPath), ProximaError> {
        // If there are no more hops, return configmap
        if path.next_hop().is_some() {
            path.next()?;
        } else {
            // If hide_folders is true, return back 403
            if self.config_file().await.global.security.config.hide_folders {
                return Err(ProximaError::Forbidden);
            } else {
                return Ok((Route::ConfigMap(Box::new(config)), path));
            }
        };

        // Check if cache contains endpoint
        if let Some(key) = path.key() {
            log::debug!("Starting fetch with cache search for {}", &key);
            if let Some(hit) = self.cache.get(&key).await {
                log::debug!("Got cache hit for {}", &key);
                return Ok((Route::Endpoint(hit), path));
            }
        };

        // If endpoint is not found in cache, check configmap
        let key = path.current();
        match config.get(&key) {
            Some(Route::ConfigMap(entry)) => {
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
                        return Ok((Route::Endpoint(hit), path));
                    }
                };

                self.fetch(path, *entry.clone()).await
            }
            Some(Route::Endpoint(entry)) => {
                match entry {
                    Endpoint::HttpConfig(entry) => {
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

                                return Ok((Route::Endpoint(hit), path));
                            }
                        };

                        // let wrapper = Endpoint::HttpConfig(entry.clone());
                        // self.cache.set(&key, &wrapper).await;

                        // Get the http config as config_file
                        let config_file = match self
                            .parse(Some(entry.url.clone()), entry.authentication.clone())
                            .await
                        {
                            Ok(c) => c,
                            Err(e) => {
                                log::error!("Error parsing remote config: {}", e.to_string());
                                return Err(e);
                            }
                        };
                        self.fetch(path, config_file.routes).await
                    }
                    Endpoint::Vault(entry) => {
                        log::debug!(
                            "Found Vault at {}",
                            &path.key().unwrap_or_else(|| "None".to_string())
                        );

                        // This needs to stay commented out until we impl Recurse for Vault
                        // Recurse will return back the body based on the ProxyPath
                        // let wrapper = Endpoint::Vault(entry.clone());
                        // self.cache.set(&key, &wrapper).await;

                        // Get last char from secret
                        let last_char = &entry
                            .secret
                            .chars()
                            .last()
                            .expect("Unable to get last char");

                        // Here we are if we find a single secret
                        if last_char != &'/' {
                            log::debug!("Single secret Vault Proxy found, attempting to get secret from cache");

                            // Check the cache for the secret
                            if let Some(key) = path.key() {
                                if let Some(hit) = self.cache.get(&key).await {
                                    log::debug!("Got cache hit for {}", &key);
                                    // Move path forward
                                    path.next()?;
                                    return Ok((Route::Endpoint(hit), path));
                                }

                                log::debug!(
                                    "Attempting to get secret at path {} from Vault",
                                    &entry.secret
                                );
                                let route = entry.get(self.vault_client()?, "").await?;

                                // If vault secret is Endpoint variant, cache endpoint
                                if let Route::Endpoint(ref endpoint) = route {
                                    self.cache.set(&path.key().expect("odd"), endpoint).await;
                                };

                                Ok((route, path))
                            } else {
                                Err(ProximaError::UnknownProxy)
                            }
                        } else {
                            // We found a directory of vault secrets
                            if let Some(key) = path.next_key() {
                                log::debug!("Searching cache for next hop of {}", &key);
                                if let Some(hit) = self.cache.get(&key).await {
                                    log::debug!("Got cache hit for {}", &key);
                                    // Move path forward
                                    path.next()?;
                                    return Ok((Route::Endpoint(hit), path));
                                }
                            };

                            // Check to see if there are any other subfolders requested,
                            // or else return the full vault config
                            match path.next_hop() {
                                Some(h) => {
                                    let route = entry.get(self.vault_client()?, &h).await?;
                                    // Move path forward
                                    path.next()?;

                                    // If vault secret is Endpoint variant, cache endpoint
                                    if let Route::Endpoint(ref endpoint) = route {
                                        self.cache.set(&path.key().expect("odd"), endpoint).await;
                                    };

                                    Ok((route, path))
                                }
                                None => {
                                    let configmap = entry
                                        .config(
                                            self.vault_client()?,
                                            path.clone(),
                                            self.cache.clone(),
                                        )
                                        .await?;
                                    Ok((Route::ConfigMap(Box::new(configmap)), path))
                                }
                            }
                        }
                    }
                    Endpoint::Proxy(entry) => {
                        log::debug!(
                            "Found Proxy at {}",
                            &path.key().unwrap_or_else(|| "None".to_string())
                        );

                        // Save entry into cache
                        let key = &path.key().expect("weird");
                        log::debug!("Adding {} to cache", key);

                        let wrapper = Endpoint::Proxy(entry.clone());
                        self.cache.set(key, &wrapper).await;

                        // Return endpoint
                        Ok((Route::Endpoint(wrapper), path))
                    }
                    Endpoint::Static(entry) => {
                        log::debug!(
                            "Found Static at {}",
                            &path.key().unwrap_or_else(|| "None".to_string())
                        );

                        let wrapper = Endpoint::Static(entry.clone());
                        self.cache.set(&key, &wrapper).await;

                        // Return endpoint
                        Ok((Route::Endpoint(Endpoint::Static(entry.clone())), path))
                    }
                    Endpoint::Redirect(entry) => {
                        log::debug!(
                            "Found Redirect at {}",
                            &path.key().unwrap_or_else(|| "None".to_string())
                        );

                        let wrapper = Endpoint::Redirect(entry.clone());
                        self.cache.set(&key, &wrapper).await;

                        // Return endpoint
                        Ok((Route::Endpoint(Endpoint::Redirect(entry.clone())), path))
                    }
                }
            }
            None => Err(ProximaError::UnknownProxy),
        }
    }

    pub async fn get(&mut self, path: ProxyPath) -> Result<(Route, ProxyPath), ProximaError> {
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

            // Update https_client live
            self.https_client.reconfigure(&new_config.global).await;

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
