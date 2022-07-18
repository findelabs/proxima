use async_trait::async_trait;
use hyper::HeaderMap;
use hyper::Method;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::security::Whitelist;
use crate::error::Error as ProximaError;

#[async_trait]
pub trait AuthorizeList: IntoIterator + Clone {
    async fn authorize(
        &self,
        headers: &HeaderMap,
        method: &Method,
        client_addr: &SocketAddr,
    ) -> Result<(), ProximaError>
    where
        Self: Sync + Send + Sized,
        <Self as IntoIterator>::Item: Send + std::fmt::Debug + Authorize + Sync,
        <Self as IntoIterator>::IntoIter: Send,
    {

        for user in self.clone() {
            log::debug!("\"Checking if connecting client matches {:?}\"", user);
            match user.authorize(headers, method, client_addr).await {
                Ok(_) => return Ok(()),
                Err(e) => match e {
                    ProximaError::UnmatchedHeader => continue,
                    _ => return Err(e),
                },
            }
        }

        // We return an unmatched header error here, as if a header did match, and failed
        // to match a token, we would have already returned that error
        log::debug!("\"Client could not be authenticated\"");
        Err(ProximaError::UnmatchedHeader)
    }
}

#[async_trait]
pub trait Authorize {

    const AUTHORIZATION_TYPE: Option<&'static str>;

    fn header_name(&self) -> &str;

    fn correct_header(&self) -> String;

    fn whitelist(&self) -> Option<&Whitelist>;

    async fn authorize(
        &self,
        headers: &HeaderMap,
        method: &Method,
        client_addr: &SocketAddr,
    ) -> Result<(), ProximaError> {

        let client_header_value = self.client_header_value(headers)?;
        let correct_header = self.correct_header();

        // If we require a specific authorization header type, check for type
        let parsed_header_value = if let Some(auth_type) = Self::AUTHORIZATION_TYPE {
            let (header_sub_type, header_sub_value) = match client_header_value.split_once(' ') {
                None => {
                    log::debug!("Failed getting header sub type");
                    return Err(ProximaError::UnmatchedHeader)
                },
                Some((t, v)) => (t.to_lowercase(),v),
            };

            log::debug!("Checking if clients header sub type {} matches auth type {}", &header_sub_type, &auth_type);

            // We didn't find a matching auth type, return err
            if header_sub_type != auth_type {
                return Err(ProximaError::UnmatchedHeader);
            }

            // Return sub value, since AUTHORIZATION_TYPE is defined
            header_sub_value

        } else {

            // Else, we return the full client header value, since AUTHORIZATION_TYPE is not set
            client_header_value
        };

        log::debug!("Comparing {} to {}", &parsed_header_value, &correct_header);
        if parsed_header_value != correct_header {
            log::debug!("Looks like these headers do not match");
            let labels = [
                ("type", self.header_name().to_owned()),
            ];
            metrics::increment_counter!(
                "proxima_security_client_authentication_failed_count",
                &labels
            );
            return Err(ProximaError::UnauthorizedClient);
        }

        if let Some(ref whitelist) = self.whitelist() {
            log::debug!("Found whitelist");
            whitelist.authorize(method, client_addr)?
        }

        Ok(())
    }

    fn client_header_value<'a>(&'a self, headers: &'a HeaderMap) -> Result<&str, ProximaError> {
        let header_name = self.header_name();
        log::debug!("Attempting to get {} header", &header_name);
        let header = match headers.get(header_name) {
            Some(header) => header,
            None => {
                log::debug!("Endpoint is locked, but no matching header found");
                let labels = [
                    ("type", header_name.to_string()),
                ];
                metrics::increment_counter!(
                    "proxima_security_client_authentication_failed_count",
                    &labels
                );
                return Err(ProximaError::UnmatchedHeader);
            }
        };

        Ok(header.to_str().expect("Cannot convert header to string"))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct AuthList<I>(Vec<I>);

// and we'll implement IntoIterator
impl<I> IntoIterator for AuthList<I> {
    type Item = I;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
