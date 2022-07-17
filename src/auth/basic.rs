use crate::error::Error as ProximaError;
use crate::security::Whitelist;
use hyper::header::HeaderValue;
use hyper::HeaderMap;
use hyper::Method;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct BasicAuth {
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct BasicAuthList(Vec<BasicAuth>);

impl BasicAuthList {
    pub async fn authorize(
        &self,
        headers: &HeaderMap,
        method: &Method,
        client_addr: &SocketAddr,
    ) -> Result<(), ProximaError> {
        log::debug!("Looping over basic users");
        let Self(internal) = self;

        let header = match headers.get("AUTHORIZATION") {
            Some(header) => header,
            None => {
                log::debug!("Endpoint is locked, but no basic authorization header found");
                metrics::increment_counter!(
                    "proxima_security_client_authentication_failed_count",
                    "type" => "absent"
                );
                return Err(ProximaError::UnmatchedHeader);
            }
        };

        // Check if the header is Basic
        let authorize = header.to_str().expect("Cannot convert header to string");
        let auth_scheme_vec: Vec<&str> = authorize.split(' ').collect();
        let scheme = auth_scheme_vec.into_iter().nth(0);

        // If header is not Basic, return err
        if let Some("Basic") = scheme {
            log::debug!("Found correct scheme for auth type: Basic");
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
        Err(ProximaError::UnauthorizedClientBasic)
    }
}

impl BasicAuth {
    #[allow(dead_code)]
    pub fn username(&self) -> String {
        self.username.clone()
    }

    #[allow(dead_code)]
    pub fn password(&self) -> String {
        self.password.clone()
    }

    pub fn basic(&self) -> String {
        log::debug!("Generating Basic auth");
        let user_pass = format!("{}:{}", self.username, self.password);
        let encoded = base64::encode(user_pass);
        let basic_auth = format!("Basic {}", encoded);
        log::debug!("Using {}", &basic_auth);
        basic_auth
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
        if HeaderValue::from_str(&self.basic()).unwrap() != header {
            metrics::increment_counter!(
                "proxima_security_client_authentication_failed_count",
                "type" => "basic"
            );
            return Err(ProximaError::UnauthorizedClientBasic);
        }
        Ok(())
    }
}
