use hyper::header::HeaderValue;
use hyper::Method;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

use crate::auth::basic;
use crate::auth::digest;
use crate::auth::bearer;
use crate::auth::jwks;
use crate::error::Error as ProximaError;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[warn(non_camel_case_types)]
pub enum ClientAuth {
    #[allow(non_camel_case_types)]
    basic(basic::BasicAuth),
    #[allow(non_camel_case_types)]
    bearer(bearer::BearerAuth),
    #[allow(non_camel_case_types)]
    digest(digest::DigestAuth),
    #[allow(non_camel_case_types)]
    jwks(jwks::JwksAuth),
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct ClientAuthList(Vec<ClientAuth>);

impl ClientAuthList {
    pub async fn authorize(
        &self,
        header: &HeaderValue,
        method: &Method,
    ) -> Result<(), ProximaError> {
        log::debug!("Looping over users");
        let Self(internal) = self;

        // We need to check if any of the client auths are Digest or Basic users
        // This is becuase we need to return the proper www-authenticate headers
        let mut digest_found = false;
        let mut basic_found = false;

        for user in internal.iter() {
            log::debug!("\"Checking if connecting client matches {:?}\"", user);
            match user.authorize(header, method).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    match e {
                        ProximaError::UnauthorizedClientBasic => basic_found = true,
                        ProximaError::UnauthorizedClientDigest => digest_found = true,
                        _ => log::trace!("Error variant not matched"),
                    }
                    continue;
                }
            }
        }
        log::debug!("\"Client could not be authenticated\"");
        match (digest_found, basic_found) {
            (true, false) => Err(ProximaError::UnauthorizedClientDigest),
            (false, true) => Err(ProximaError::UnauthorizedClientBasic),
            _ => Err(ProximaError::UnauthorizedClient),
        }
    }
}

impl<'a> ClientAuth {
    pub async fn authorize(
        &self,
        header: &HeaderValue,
        method: &Method,
    ) -> Result<(), ProximaError> {
        metrics::increment_counter!("proxima_security_client_authentication_total");
        match self {
            ClientAuth::basic(auth) => {
                auth.authorize(header, method).await?;
                if let Some(ref whitelist) = auth.whitelist {
                    log::debug!("Found whitelist");
                    whitelist.authorize(method)?
                }
            }
            ClientAuth::bearer(auth) => {
                auth.authorize(header, method).await?;
                if let Some(ref whitelist) = auth.whitelist {
                    log::debug!("Found whitelist");
                    whitelist.authorize(method)?
                }
            }
            ClientAuth::digest(auth) => {
                auth.authorize(header, method).await?;
                if let Some(ref whitelist) = auth.whitelist {
                    log::debug!("Found whitelist");
                    whitelist.authorize(method)?
                }
            }
            ClientAuth::jwks(auth) => {
                auth.authorize(header, method).await?;
                if let Some(ref whitelist) = auth.whitelist {
                    log::debug!("Found whitelist");
                    whitelist.authorize(method)?
                }
            }
        }
        Ok(())
    }
}
