use crate::error::Error as ProximaError;
use crate::security::Whitelist;
use async_trait::async_trait;
use digest_auth::{AuthContext, AuthorizationHeader};
use hyper::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::auth::traits::{AuthList, Authorize, AuthorizeList};

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct DigestAuth {
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>,
}

// Pull in trait
impl AuthorizeList for AuthList<DigestAuth> {}

#[async_trait]
impl Authorize for DigestAuth {
    const AUTHORIZATION_TYPE: Option<&'static str> = Some("digest");

    fn authenticate_client(
        &self,
        client_header: &str,
        _headers: &HeaderMap,
    ) -> Result<(), ProximaError> {
        let client_authorization_header = match AuthorizationHeader::parse(client_header) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Error converting client authorization header: {}", e);
                return Err(ProximaError::UnauthorizedClientDigest);
            }
        };

        let context = AuthContext::new(
            self.username.clone(),
            self.password.clone(),
            client_authorization_header.clone().uri,
        );

        log::trace!("Digest context: {:?}", &context);

        let mut server_authorization_header = client_authorization_header.clone();
        server_authorization_header.digest(&context);

        if server_authorization_header != client_authorization_header {
            return Err(ProximaError::UnauthorizedClientDigest);
        }

        Ok(())
    }

    fn header_name(&self) -> &str {
        "AUTHORIZATION"
    }

    fn whitelist(&self) -> Option<&Whitelist> {
        self.whitelist.as_ref()
    }
}
