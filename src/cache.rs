use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::Endpoint;

pub type CacheMap = Arc<RwLock<HashMap<String, (Endpoint, String)>>>;

#[derive(Debug, Clone)]
pub struct Cache {
    pub cache: CacheMap
}

impl Cache {
    pub fn default() -> Cache {
        Cache { cache: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub async fn clear(&mut self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub async fn get(&self, key: &str) -> Option<(Endpoint, &str)> {
        log::info!("Searching for {} in cache", key);
        let cache = self.cache.read().await;
        cache.get(key).cloned()
    }

    pub async fn set(&self, key: &str, remainder: &str, value: &Endpoint) {
        log::info!("Adding {} to cache with remainder of {}", key, remainder);
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), (value.clone(), remainder.to_string()));
    }
}
