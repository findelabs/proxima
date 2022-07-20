use crate::security::Whitelist;
use async_trait::async_trait;
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::auth::traits::{AuthList, Authorize, AuthorizeList};
use crate::error::Error as ProximaError;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct BasicAuth {
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>,
}

// Pull in trait
impl AuthorizeList for AuthList<BasicAuth> {}

#[async_trait]
impl Authorize for BasicAuth {
    const AUTHORIZATION_TYPE: Option<&'static str> = Some("basic");

    fn header_name(&self) -> &str {
        "AUTHORIZATION"
    }

    fn authenticate_client(
        &self,
        client_header: &str,
        _headers: &HeaderMap,
    ) -> Result<(), ProximaError> {
        let correct_header = self.base64_value();

        let header_value = match client_header.split_once(' ') {
            None => return Err(ProximaError::UnmatchedHeader),
            Some((_, v)) => v,
        };

        log::debug!("Comparing {} to {}", &header_value, &correct_header);
        if header_value != correct_header {
            Err(ProximaError::UnauthorizedClient)
        } else {
            log::debug!("Client is authenticated");
            Ok(())
        }
    }

    fn whitelist(&self) -> Option<&Whitelist> {
        self.whitelist.as_ref()
    }
}

impl BasicAuth {
    #[allow(dead_code)]
    pub fn username(&self) -> String {
        self.username.clone()
    }

    pub fn base64_value(&self) -> String {
        let user_pass = format!("{}:{}", self.username, self.password);
        base64::encode(user_pass)
    }

    pub fn basic(&self) -> String {
        log::debug!("Generating Basic auth");
        let user_pass = format!("{}:{}", self.username, self.password);
        let encoded = base64::encode(user_pass);
        let basic_auth = format!("basic {}", encoded);
        log::debug!("Using {}", &basic_auth);
        basic_auth
    }
}
