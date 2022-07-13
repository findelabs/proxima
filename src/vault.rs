use crate::error::Error as ProximaError;
use async_recursion::async_recursion;
use base64;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use vault_client_rs::client::Client as VaultClient;

use crate::config::Endpoint;
use crate::path::ProxyPath;
use crate::config::ConfigMap;
use crate::config::Entry;
use crate::cache::Cache;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultConfig {
    pub template: Option<String>,
    pub secret: String,
}

impl Hash for VaultConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.template.hash(state);
        self.secret.hash(state);
    }
}

impl VaultConfig {
    pub async fn config(&self, mut vault: VaultClient, path: ProxyPath, cache: Cache<Endpoint>) -> Result<ConfigMap, ProximaError> {
        let list = vault.list(&self.secret).await?;

        let cache_prefix = path.key().unwrap_or("/".to_owned());

        // Create new map
        let mut map = BTreeMap::new();

        // Loop over keys in folder
        for key in list.keys().await {
            let key_str = key.as_str().expect("Could not extract string");
            let secret_path = format!("{}{}", self.secret, &key_str);

            let cache_path = format!("{}/{}", cache_prefix, key_str);
            log::debug!("Attempting to get {} from cache", &cache_path);

            // Attempt to find key in cache, before pulling from vault
            match cache.get(&cache_path).await {
                Some(endpoint) => {
                    log::debug!("Found {} in cache", &cache_path);
                    map.insert(key_str.to_string(), Entry::Endpoint(endpoint));
                },
                None => {
                    let secret = match vault.get(&secret_path).await {
                        Ok(s) => s,
                        Err(e) => {
                            log::error!("Error getting secret {}: {}", &secret_path, e);
                            continue;
                        }
                    };
                    match self.template(secret.data().await).await {
                        Ok(endpoint) => {
                            // If vault secret is Endpoint variant, cache endpoint
                            if let Entry::Endpoint(ref e) = endpoint  {
                                cache.set(&cache_path, &e).await;
                            }

                            map.insert(key_str.to_string(), endpoint);
                        }
                        Err(e) => {
                            log::error!("Error generating template: {}", e);
                            continue;
                        }
                    }

                }
            };
        }
        Ok(map)
    }

    pub async fn get(&self, mut vault: VaultClient, secret: &str) -> Result<Entry, ProximaError> {
        let secret_path = format!("{}{}", self.secret, secret);
        let secret = vault.get(&secret_path).await?;
        match self.template(secret.data().await).await {
            Ok(templated) => Ok(templated),
            Err(e) => {
                log::error!("Error generating template: {}", e);
                Err(e)
            }
        }
    }

    #[async_recursion]
    pub async fn template(&self, secret: Map<String, Value>) -> Result<Entry, ProximaError> {
        if let Some(_template) = &self.template {
            let handlebars = self.handlebars().await?;
            let output = handlebars.render("secret", &secret)?;
            log::debug!("Rendered string: {}", &output);
            let v: Entry = serde_json::from_str(&output)?;
            Ok(v)
        } else {
            let v: Entry = serde_json::from_value(serde_json::Value::Object(secret))?;
            Ok(v)
        }
    }

    pub async fn handlebars(&self) -> Result<Handlebars<'_>, ProximaError> {
        let bytes = base64::decode(&self.template.as_ref().expect("fn called without checking that self.template is some"))?;
        let template_decoded = std::str::from_utf8(&bytes)?;
        let mut handlebars = Handlebars::new();
        handlebars.register_template_string("secret", template_decoded)?;
        handlebars.set_strict_mode(true);

        Ok(handlebars)
    }
}
