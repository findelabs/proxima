use serde::{Deserialize, Serialize};
use crate::security::Whitelist;
use crate::error::Error as ProximaError;
use hyper::header::HeaderValue;
use hyper::Method;

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
        header: &HeaderValue,
        method: &Method
    ) -> Result<(), ProximaError> {
        log::debug!("Looping over bearer tokens");
        let Self(internal) = self;

        for user in internal.iter() {
            log::debug!("\"Checking if connecting client matches {:?}\"", user);
            match user.authorize(header, method).await {
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
        method: &Method
    ) -> Result<(), ProximaError> {
        if let Some(ref whitelist) = self.whitelist {
            log::debug!("Found whitelist");
            whitelist.authorize(method)?
        }

        let header_str = header.to_str().expect("Cannot convert header to string");
        let header_split: Vec<&str> = header_str.split(' ').collect();
        let token = match header_split.into_iter().nth(1) {
            None => return Err(ProximaError::Unauthorized),
            Some(t) => t
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