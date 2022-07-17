use crate::error::Error as ProximaError;
use crate::security::Whitelist;
use hyper::header::HeaderValue;
use hyper::HeaderMap;
use hyper::Method;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct BearerAuth {
    #[serde(skip_serializing)]
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct BearerAuthList(Vec<BearerAuth>);

impl BearerAuthList {
    pub async fn authorize(
        &self,
        headers: &HeaderMap,
        method: &Method,
        client_addr: &SocketAddr,
    ) -> Result<(), ProximaError> {
        log::debug!("Looping over bearer tokens");
        let Self(internal) = self;

        let header = match headers.get("AUTHORIZATION") {
            Some(header) => header,
            None => {
                log::debug!("Endpoint is locked, but no bearer authorization header found");
                metrics::increment_counter!(
                    "proxima_security_client_authentication_failed_count",
                    "type" => "absent"
                );
                return Err(ProximaError::UnmatchedHeader);
            }
        };

        // Check if the header is Bearer
        let authorize = header.to_str().expect("Cannot convert header to string");
        let auth_scheme_vec: Vec<&str> = authorize.split(' ').collect();
        let scheme = auth_scheme_vec.into_iter().nth(0);

        // If header is not Bearer , return err
        if let Some("Bearer") = scheme {
            log::debug!("Found correct scheme for auth type: Bearer");
        } else {
            return Err(ProximaError::UnmatchedHeader);
        }

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
        Err(ProximaError::UnauthorizedClient)
    }
}

impl BearerAuth {
    pub fn token(&self) -> String {
        self.token.clone()
    }

    pub async fn authorize(
        &self,
        header: &HeaderValue,
        method: &Method,
        client_addr: &SocketAddr,
    ) -> Result<(), ProximaError> {
        if let Some(ref whitelist) = self.whitelist {
            log::debug!("Found whitelist");
            whitelist.authorize(method, client_addr)?
        }

        let header_str = header.to_str().expect("Cannot convert header to string");
        let header_split: Vec<&str> = header_str.split(' ').collect();
        let token = match header_split.into_iter().nth(1) {
            None => return Err(ProximaError::Unauthorized),
            Some(t) => t,
        };

        log::debug!("Comparing {} to {}", token, &self.token());
        if token != &self.token() {
            metrics::increment_counter!(
                "proxima_security_client_authentication_failed_count",
                "type" => "bearer"
            );
            return Err(ProximaError::UnauthorizedClient);
        }
        Ok(())
    }
}
