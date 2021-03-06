use crate::security::Whitelist;
use async_trait::async_trait;
use hyper::header::HeaderName;
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::auth::traits::{AuthList, Authorize, AuthorizeList};
use crate::error::Error as ProximaError;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct ApiKeyAuth {
    #[serde(skip_serializing)]
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>,
}

// Default API Key Header name
pub const KEY: &str = "x-api-key";

// Pull in trait
impl AuthorizeList for AuthList<ApiKeyAuth> {}

#[async_trait]
impl Authorize for ApiKeyAuth {
    const AUTHORIZATION_TYPE: Option<&'static str> = None;

    fn header_name(&self) -> &str {
        match &self.key {
            Some(k) => k.as_str(),
            None => KEY,
        }
    }

    fn whitelist(&self) -> Option<&Whitelist> {
        self.whitelist.as_ref()
    }

    fn authenticate_client(
        &self,
        client_header: &str,
        _headers: &HeaderMap,
    ) -> Result<(), ProximaError> {
        let correct_header = self.token();

        log::debug!("Comparing {} to {}", &client_header, &correct_header);
        if client_header != correct_header {
            Err(ProximaError::UnauthorizedClient)
        } else {
            log::debug!("Client is authenticated");
            Ok(())
        }
    }
}

impl ApiKeyAuth {
    pub fn token(&self) -> String {
        self.token.clone()
    }

    pub fn headername(&self) -> HeaderName {
        let default = HeaderName::from_bytes(KEY.as_bytes()).unwrap();
        match &self.key {
            Some(k) => {
                let lowercase = k.to_lowercase();
                let bytes = lowercase.as_bytes();
                match HeaderName::from_bytes(bytes) {
                    Ok(n) => n,
                    Err(e) => {
                        log::error!("Error converting api key to header: {}", e);
                        default
                    }
                }
            }
            None => default,
        }
    }
}
