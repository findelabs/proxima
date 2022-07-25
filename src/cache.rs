//use crate::config::Proxy;
use serde_json::map::Map;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type CacheMap<T> = Arc<RwLock<HashMap<String, T>>>;

#[derive(Debug, Clone)]
pub struct Cache<T> {
    pub name: String,
    pub cache: CacheMap<T>,
}

impl<T> Default for Cache<T> {
    fn default() -> Cache<T> {
        Cache {
            // Used for metric tracking
            name: String::default(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<'a, T: std::clone::Clone + std::fmt::Display> Cache<T> {
    pub fn new(name: Option<String>) -> Cache<T> {
        Cache {
            name: name.unwrap_or_default(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn clear(&mut self) {
        log::debug!("\"Clearing cache\"");
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub async fn get(&self, key: &str) -> Option<T> {
        log::debug!("Searching for {} in cache", key);
        metrics::increment_counter!("proxima_cache_attempt_total", "name" => self.name.clone());
        let cache = self.cache.read().await;
        match cache.get(key).cloned() {
            Some(h) => Some(h),
            None => {
                log::debug!("Cache miss for {}", &key);
                metrics::increment_counter!("proxima_cache_miss_total", "name" => self.name.clone());
                None
            }
        }
    }

    pub async fn remove(&self, key: &str) -> Option<String> {
        log::debug!("Removing {} from cache", &key);
        let mut cache = self.cache.write().await;
        cache.remove_entry(key).map(|(key, _)| key)
    }

    pub async fn cache(&self) -> Map<String, Value> {
        log::debug!("Generating cache");
        let mut map = Map::new();
        let cache = self.cache.read().await;
        for (key, value) in &*cache {
            map.insert(
                key.to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
        map
    }

    pub async fn set(&self, key: &str, endpoint: &T) {
        log::debug!("Adding {} to cache", key);
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), endpoint.clone());
        let count = cache.len() as f64;

        metrics::gauge!("proxima_cache_keys", count, "name" => self.name.clone());
    }
}
