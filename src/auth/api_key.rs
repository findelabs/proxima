use serde::{Deserialize, Serialize};
use crate::security::Whitelist;
use crate::error::Error as ProximaError;
//use hyper::header::HeaderValue;
use hyper::Method;
use std::net::SocketAddr;
use hyper::HeaderMap;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct ApiKeyAuth {
    #[serde(skip_serializing)]
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct ApiKeyAuthList(Vec<ApiKeyAuth>);

// Default API Key Header name
const KEY: &str = "x-api-key";

impl ApiKeyAuthList {
    pub async fn authorize(
        &self,
        headers: &HeaderMap,
        method: &Method,
        client_addr: &SocketAddr
    ) -> Result<(), ProximaError> {
        log::debug!("Looping over api key tokens");
        let Self(internal) = self;

        for user in internal.iter() {
            log::debug!("\"Checking if connecting client matches {:?}\"", user);
            match user.authorize(headers, method, client_addr).await {
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

impl ApiKeyAuth {
    pub fn token(&self) -> String {
        self.token.clone()
    }

    pub fn key(&self) -> String {
        match &self.key {
            Some(k) => k.to_string(),
            None => KEY.to_string()
        }
    }

    pub async fn authorize(
        &self,
        headers: &HeaderMap,
        method: &Method,
        client_addr: &SocketAddr
    ) -> Result<(), ProximaError> {
        if let Some(ref whitelist) = self.whitelist {
            log::debug!("Found whitelist");
            whitelist.authorize(method, client_addr)?
        }

        let key = match &self.key {
            Some(k) => k.as_str(),
            None => KEY
        };
            
        let header = match headers.get(key) {
            Some(header) => header,
            None => {
                log::debug!("Endpoint is locked, but no api key authorization header found");
                metrics::increment_counter!(
                    "proxima_security_client_authentication_failed_count",
                    "type" => "api_key"
                );
                return Err(ProximaError::UnmatchedHeader)
            }
        };


        let token = header.to_str().expect("Cannot convert header to string");

        log::debug!("Comparing {} to {}", token, &self.token());
        if token != &self.token() {
            metrics::increment_counter!(
                "proxima_security_client_authentication_failed_count",
                "type" => "api_key"
            );
            return Err(ProximaError::UnauthorizedClient);
        }
        Ok(())
    }
}
