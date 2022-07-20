use crate::security::Whitelist;
use async_trait::async_trait;
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::auth::traits::{AuthList, Authorize, AuthorizeList};
use crate::error::Error as ProximaError;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct BearerAuth {
    #[serde(skip_serializing)]
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>,
}

// Pull in trait
impl AuthorizeList for AuthList<BearerAuth> {}

#[async_trait]
impl Authorize for BearerAuth {
    const AUTHORIZATION_TYPE: Option<&'static str> = Some("bearer");

    fn header_name(&self) -> &str {
        "AUTHORIZATION"
    }

    fn authenticate_client(
        &self,
        client_header: &str,
        _headers: &HeaderMap,
    ) -> Result<(), ProximaError> {
        let header_value = match client_header.split_once(' ') {
            None => return Err(ProximaError::UnmatchedHeader),
            Some((_, v)) => v,
        };

        let correct_header = self.token();

        log::debug!("Comparing {} to {}", &header_value, &correct_header);
        if header_value != correct_header {
            return Err(ProximaError::UnauthorizedClient);
        } else {
            log::debug!("Client is authenticated");
            Ok(())
        }
    }

    fn whitelist(&self) -> Option<&Whitelist> {
        self.whitelist.as_ref()
    }
}

impl BearerAuth {
    pub fn token(&self) -> String {
        self.token.clone()
    }
}
