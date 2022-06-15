use crate::config::Endpoint;
use crate::path::ProxyPath;
use serde_json::map::Map;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type CacheMap = Arc<RwLock<HashMap<String, Endpoint>>>;

#[derive(Debug, Clone)]
pub struct Cache {
    pub cache: CacheMap,
}

impl Default for Cache {
    fn default() -> Cache {
        Cache {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<'a> Cache {
    pub async fn clear(&mut self) {
        log::debug!("Clearing cache");
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub async fn get(&self, key: &str) -> Option<Endpoint> {
        log::debug!("Searching for {} in cache", key);
        metrics::increment_counter!("proxima_cache_attempt_total");
        let cache = self.cache.read().await;
        match cache.get(key).cloned() {
            Some(h) => Some(h),
            None => {
                log::debug!("Cache miss for {}", &key);
                metrics::increment_counter!("proxima_cache_miss_total");
                None
            }
        }
    }

    pub async fn remove(&self, path: ProxyPath) -> Option<String> {
        log::debug!("Removing {} from cache", &path.path());
        let mut cache = self.cache.write().await;
        cache.remove_entry(path.path()).map(|(key, _)| key)
    }

    pub async fn cache(&self) -> Map<String, Value> {
        log::debug!("Generating cache");
        let mut map = Map::new();
        let cache = self.cache.read().await;
        for (key, endpoint) in &*cache {
            let value = endpoint.url().await;
            map.insert(key.to_string(), serde_json::Value::String(value));
        }
        map
    }

    pub async fn set(&self, key: &str, endpoint: &Endpoint) {
        log::debug!(
            "Adding {} to cache",
            key
        );
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), endpoint.clone());
    }
}
