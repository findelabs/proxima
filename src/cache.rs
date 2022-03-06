use crate::config::Endpoint;
use crate::path::ProxyPath;
use serde_json::map::Map;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type CacheMap = Arc<RwLock<HashMap<String, (Endpoint, ProxyPath)>>>;

#[derive(Debug, Clone)]
pub struct Cache {
    pub cache: CacheMap,
}

impl Cache {
    pub fn default() -> Cache {
        Cache {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn clear(&mut self) {
        log::debug!("Clearing cache");
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub async fn get(&self, key: &str) -> Option<(Endpoint, ProxyPath)> {
        log::debug!("Searching for {} in cache", key);
        let cache = self.cache.read().await;
        cache.get(key).cloned()
    }

    pub async fn cache(&self) -> Map<String, Value> {
        log::debug!("Generating cache");
        let mut map = Map::new();
        let cache = self.cache.read().await;
        for (key, (endpoint, proxypath)) in &*cache {
            let value = format!("{}{}", endpoint.url, proxypath.path());
            map.insert(key.to_string(), serde_json::Value::String(value));
        }
        map
    }

    pub async fn set(&self, key: &str, remainder: &ProxyPath, value: &Endpoint) {
        log::debug!(
            "Adding {} to cache with remainder of {}",
            key,
            remainder.path()
        );
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), (value.clone(), remainder.clone()));
    }
}
