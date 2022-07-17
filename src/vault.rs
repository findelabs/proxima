use crate::error::Error as ProximaError;
use async_recursion::async_recursion;
use base64;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use vault_client_rs::client::Client as VaultClient;

use crate::cache::Cache;
use crate::config::ConfigMap;
use crate::config::Endpoint;
use crate::config::Proxy;
use crate::config::Route;
use crate::path::ProxyPath;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vault {
    pub template: Option<String>,
    pub secret: String,
}

impl Hash for Vault {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.template.hash(state);
        self.secret.hash(state);
    }
}

impl Vault {
    pub async fn config(
        &self,
        mut vault: VaultClient,
        path: ProxyPath,
        cache: Cache<Proxy>,
    ) -> Result<ConfigMap, ProximaError> {
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
                    map.insert(
                        key_str.to_string(),
                        Route::Endpoint(Endpoint::Proxy(endpoint)),
                    );
                }
                None => {
                    let secret = match vault.get(&secret_path).await {
                        Ok(s) => s,
                        Err(e) => {
                            log::error!("Error getting secret {}: {}", &secret_path, e);
                            continue;
                        }
                    };
                    match self.template(secret.data().await).await {
                        Ok(route) => {
                            // If vault secret is Proxy variant, cache endpoint
                            if let Route::Endpoint(ref entry) = route {
                                if let Endpoint::Proxy(ref endpoint) = entry {
                                    cache.set(&cache_path, &endpoint).await;
                                }
                            }

                            map.insert(key_str.to_string(), route);
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

    pub async fn get(&self, mut vault: VaultClient, secret: &str) -> Result<Route, ProximaError> {
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
    pub async fn template(&self, secret: Map<String, Value>) -> Result<Route, ProximaError> {
        if let Some(_template) = &self.template {
            let handlebars = self.handlebars().await?;
            let output = handlebars.render("secret", &secret)?;
            log::debug!("Rendered string: {}", &output);
            let v: Route = serde_json::from_str(&output)?;
            Ok(v)
        } else {
            let v: Route = serde_json::from_value(serde_json::Value::Object(secret))?;
            Ok(v)
        }
    }

    pub async fn handlebars(&self) -> Result<Handlebars<'_>, ProximaError> {
        let bytes = base64::decode(
            &self
                .template
                .as_ref()
                .expect("fn called without checking that self.template is some"),
        )?;
        let template_decoded = std::str::from_utf8(&bytes)?;
        let mut handlebars = Handlebars::new();
        handlebars.register_template_string("secret", template_decoded)?;
        handlebars.set_strict_mode(true);

        Ok(handlebars)
    }
}
