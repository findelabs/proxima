use serde::{Deserialize, Serialize};
use hyper::{Method};

use crate::error::Error as ProximaError;
use crate::auth::client::ClientAuthList;



#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct Whitelist {
    pub methods: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct Security {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>,
    #[serde(skip_serializing)]
    pub client: Option<ClientAuthList>,
}

pub fn display_security(item: &Option<Security>) -> bool {
    if let Some(security) = item {
        if security.whitelist.is_some() {
            false
        } else {
            true
        }
    } else {
        true
    }
}

impl Whitelist {
    pub fn authorize(&self, method: &Method) -> Result<(), ProximaError> {
        if let Some(ref methods) = self.methods {
            log::debug!("The method whitelist allows: {:?}", methods);
            metrics::increment_counter!(
                "proxima_security_method_whitelist_total"
            );
            match methods.contains(&method.to_string()) {
                true => {
                    log::debug!("{} is in whitelist", method);
                }
                false => {
                    metrics::increment_counter!(
                        "proxima_security_method_blocked_total"
                    );
                    log::info!("Blocked {} method", method);
                    return Err(ProximaError::Forbidden);
                }
            }
        }
        Ok(())
    }
}
