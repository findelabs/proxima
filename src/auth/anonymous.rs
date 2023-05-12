use crate::security::Whitelist;
use async_trait::async_trait;
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::auth::traits::Authorize;
use crate::error::Error as ProximaError;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct AnonymousAuth {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>,
}

// Pull in trait
//impl AuthorizeList for AuthList<AnonymousAuth> {}

#[async_trait]
impl Authorize for AnonymousAuth {
    const AUTHORIZATION_TYPE: Option<&'static str> = None;

    fn header_name(&self) -> &str {
        "HOST"
    }

    fn authenticate_client(
        &self,
        _client_header: &str,
        _headers: &HeaderMap,
    ) -> Result<(), ProximaError> {
        Ok(())
    }

    fn whitelist(&self) -> Option<&Whitelist> {
        self.whitelist.as_ref()
    }
}
