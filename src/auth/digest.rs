use serde::{Deserialize, Serialize};
use crate::security::Whitelist;
use crate::error::Error as ProximaError;
use hyper::header::HeaderValue;
use digest_auth::{AuthContext, AuthorizationHeader};
use hyper::Method;
use std::net::SocketAddr;


#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct DigestAuth {
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct DigestAuthList(Vec<DigestAuth>);

impl DigestAuthList {
    pub async fn authorize(
        &self,
        header: &HeaderValue,
        method: &Method,
        client_addr: &SocketAddr
    ) -> Result<(), ProximaError> {
        log::debug!("Looping over digest users");
        let Self(internal) = self;

        for user in internal.iter() {
            log::debug!("\"Checking if connecting client matches {:?}\"", user);
            match user.authorize(header, method, client_addr).await {
                Ok(_) => return Ok(()),
                Err(_) => {
                    continue;
                }
            }
        }
        log::debug!("\"Client could not be authenticated\"");
        Err(ProximaError::UnauthorizedClientDigest)
    }
}

impl DigestAuth {
    pub async fn authorize(
        &self,
        header: &HeaderValue,
        method: &Method,
        client_addr: &SocketAddr
    ) -> Result<(), ProximaError> {
        let client_authorization_header =
            match AuthorizationHeader::parse(header.to_str().unwrap()) {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Error converting client authorization header: {}", e);
                    return Err(ProximaError::UnauthorizedClientDigest);
            }
        };

        let context = AuthContext::new(
            self.username.clone(),
            self.password.clone(),
            &client_authorization_header.uri,
        );

        log::trace!("Digest context: {:?}", &context);

        let mut server_authorization_header = client_authorization_header.clone();
        server_authorization_header.digest(&context);

        if server_authorization_header != client_authorization_header {
            metrics::increment_counter!(
                "proxima_security_client_authentication_failed_count",
                "type" => "digest"
            );
            return Err(ProximaError::UnauthorizedClientDigest);
        }

        if let Some(ref whitelist) = self.whitelist {
            log::debug!("Found whitelist");
            whitelist.authorize(method, client_addr)?
        }

        Ok(())
    }
}
