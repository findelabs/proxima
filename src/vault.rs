use std::sync::Arc;
use serde_json::{Value, Map};
use std::collections::BTreeMap;
use handlebars::Handlebars;
use crate::error::Error as ProximaError;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use base64;
use async_recursion::async_recursion;
use vault_client_rs::client::Client as VaultClient;

//type BoxResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

use crate::config::ConfigMap;
use crate::config::{Entry};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultConfig {
    pub template: String,
    pub secret: String,
//    #[serde(skip)]
//    pub handlebars: Option<Arc<Mutex<Handlebars<'a>>>>,
    pub insecure: bool,
    pub recursive: bool
}

impl VaultConfig {
    pub async fn config(&mut self, mut vault: VaultClient) -> Result<ConfigMap, ProximaError> {
        let list = vault.list(&self.secret).await?;
        let mut map = BTreeMap::new();
        for key in list.keys().await {
            let key_str = key.as_str().expect("Could not extract string");
            let secret_path = format!("{}{}", self.secret, &key_str);
            let secret = match vault.get(&secret_path).await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Error getting secret: {}", e);
                    continue
                }
            };
            match self.template(secret.data().await).await {
                Ok(t) => {
                    map.insert(key_str.to_string(), t);
                },
                Err(e) => {
                    log::error!("Error generating template: {}", e);
                    continue
                }
            }
        }
        Ok(map)
    }

    #[async_recursion]
    pub async fn template(&mut self, secret: Map<String, Value>) -> Result<Entry, ProximaError> {
        let handlebars = self.handlebars().await?;
        let output = handlebars.render("secret", &secret)?;
        log::debug!("Rendered string: {}", &output);

        let v: Entry = serde_json::from_str(&output)?;
        Ok(v)
    }

    pub async fn handlebars(&mut self) -> Result<Handlebars<'_>, ProximaError> {
        let bytes = base64::decode(&self.template)?;
        let template_decoded = std::str::from_utf8(&bytes)?;
        let mut handlebars = Handlebars::new();
        handlebars.register_template_string("secret", template_decoded)?;
        handlebars.set_strict_mode(true);
        
        Ok(handlebars)
    }
}
