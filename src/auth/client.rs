use digest_auth::{AuthContext, AuthorizationHeader};
use hyper::header::HeaderValue;
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
    pub async fn authorize(&self, header: &HeaderValue) -> Result<(), ProximaError> {
        let Self(internal) = self;
        for user in internal.iter() {
            log::debug!("\"Checking if client auth against {:?}\"", user);
            match user.authorize(header).await {
                Ok(_) => return Ok(()),
                Err(_) => continue,
            }
        }
        log::debug!("\"Client could not be authenticated\"");
        Err(ProximaError::UnauthorizedUser)
    }
}

impl<'a> ClientAuth {
    pub async fn authorize(&self, header: &HeaderValue) -> Result<(), ProximaError> {
        metrics::increment_counter!("proxima_endpoint_authentication_total");
        match self {
            ClientAuth::basic(auth) => {
                if HeaderValue::from_str(&auth.basic()).unwrap() != header {
                    metrics::increment_counter!(
                        "proxima_endpoint_authentication_basic_failed_total"
                    );
                    return Err(ProximaError::UnauthorizedUser);
                }
            }
            ClientAuth::bearer(auth) => {
                if HeaderValue::from_str(&auth.token()).unwrap() != header {
                    metrics::increment_counter!(
                        "proxima_endpoint_authentication_bearer_failed_total"
                    );
                    return Err(ProximaError::UnauthorizedUser);
                }
            }
            ClientAuth::digest(auth) => {
                let client_authorization_header =
                    match AuthorizationHeader::parse(header.to_str().unwrap()) {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!("Error converting client authorization header: {}", e);
                            return Err(ProximaError::UnauthorizedDigestUser);
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
                        "proxima_endpoint_authentication_digest_failed_total"
                    );
                    return Err(ProximaError::UnauthorizedDigestUser);
                }
            }
            ClientAuth::jwks(auth) => {
                let authorize = header.to_str().expect("Cannot convert header to string");
                let token: Vec<&str> = authorize.split(' ').collect();
                if (auth.validate(token[1]).await).is_err() {
                    metrics::increment_counter!(
                        "proxima_endpoint_authentication_bearer_failed_total"
                    );
                    return Err(ProximaError::UnauthorizedUser);
                }
            }
        }
        Ok(())
    }
}
