use digest_auth::{AuthContext, AuthorizationHeader};
use hyper::header::HeaderValue;
use hyper::Method;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

use crate::auth::auth;
use crate::error::Error as ProximaError;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[warn(non_camel_case_types)]
pub enum ClientAuth {
    #[allow(non_camel_case_types)]
    basic(auth::BasicAuth),
    #[allow(non_camel_case_types)]
    bearer(auth::BearerAuth),
    #[allow(non_camel_case_types)]
    digest(auth::DigestAuth),
    #[allow(non_camel_case_types)]
    jwks(auth::JwksAuth),
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
                if let Some(ref whitelist) = auth.whitelist {
                    log::debug!("Found whitelist");
                    whitelist.authorize(method)?
                }
                if HeaderValue::from_str(&auth.basic()).unwrap() != header {
                    metrics::increment_counter!(
                        "proxima_security_client_authentication_failed_count",
                        "type" => "basic"
                    );
                    return Err(ProximaError::UnauthorizedClientBasic);
                }
            }
            ClientAuth::bearer(auth) => {
                if let Some(ref whitelist) = auth.whitelist {
                    log::debug!("Found whitelist");
                    whitelist.authorize(method)?
                }
                if HeaderValue::from_str(&auth.token()).unwrap() != header {
                    metrics::increment_counter!(
                        "proxima_security_client_authentication_failed_count",
                        "type" => "bearer"
                    );
                    return Err(ProximaError::UnauthorizedClient);
                }
            }
            ClientAuth::digest(auth) => {
                if let Some(ref whitelist) = auth.whitelist {
                    log::debug!("Found whitelist");
                    whitelist.authorize(method)?
                }
                let client_authorization_header =
                    match AuthorizationHeader::parse(header.to_str().unwrap()) {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!("Error converting client authorization header: {}", e);
                            return Err(ProximaError::UnauthorizedClientDigest);
                        }
                    };

                let context = AuthContext::new(
                    auth.username.clone(),
                    auth.password.clone(),
                    &client_authorization_header.uri,
                );
                let mut server_authorization_header = client_authorization_header.clone();
                server_authorization_header.digest(&context);

                if server_authorization_header != client_authorization_header {
                    metrics::increment_counter!(
                        "proxima_security_client_authentication_failed_count",
                        "type" => "digest"
                    );
                    return Err(ProximaError::UnauthorizedClientDigest);
                }
            }
            ClientAuth::jwks(auth) => {
                if let Some(ref whitelist) = auth.whitelist {
                    log::debug!("Found whitelist");
                    whitelist.authorize(method)?
                }
                let authorize = header.to_str().expect("Cannot convert header to string");
                let token: Vec<&str> = authorize.split(' ').collect();
                if (auth.validate(token[1]).await).is_err() {
                    metrics::increment_counter!(
                        "proxima_security_client_authentication_failed_count",
                        "type" => "jwks"
                    );
                    return Err(ProximaError::UnauthorizedClient);
                }
            }
        }
        Ok(())
    }
}
