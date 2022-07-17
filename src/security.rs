use hyper::HeaderMap;
use hyper::Method;
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

//use crate::auth::client::ClientAuthList;
use crate::auth::api_key::ApiKeyAuth;
use crate::auth::basic::BasicAuthList;
use crate::auth::bearer::BearerAuthList;
use crate::auth::digest::DigestAuthList;
use crate::auth::jwks::JwksAuthList;
use crate::auth::traits::{AuthList, AuthorizeList};
use crate::error::Error as ProximaError;

//pub trait Authorize {
//    fn security(&self) -> Option<Security>;
//
//    fn authorize(&self, method: &Method, client_addr: &SocketAddr) -> Result<(), ProximaError> {
//        match &self.security() {
//            Some(security) => {
//                match security.whitelist {
//                    Some(whitelist) => whitelist.authorize(method, client_addr),
//                    None => Ok(())
//                }
//            }
//            None => Ok(())
//        }
//    }
//}
//
//pub trait Authenticate {
//    fn security(&self) -> Option<Security>;
//
//    async fn authenticate(&self, headers: &HeaderMap, method: &Method, client_addr: &SocketAddr) -> Result<(), ProximaError> {
//        match &self.security() {
//            Some(security) => {
//                match security.client {
//                    Some(clientlist) => clientlist.authorize(method, client_addr).await,
//                    None => Ok(())
//                }
//            }
//            None => Ok(())
//        }
//    }
//}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct Security {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>,
    #[serde(skip_serializing)]
    pub client: Option<AuthorizedClients>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct Whitelist {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub methods: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<Vec<IpNetwork>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct AuthorizedClients {
    pub basic: Option<BasicAuthList>,
    pub digest: Option<DigestAuthList>,
    pub bearer: Option<BearerAuthList>,
    pub jwks: Option<JwksAuthList>,
    pub api_key: Option<AuthList<ApiKeyAuth>>,
}

impl AuthorizedClients {
    pub async fn authorize(
        &self,
        headers: &HeaderMap,
        method: &Method,
        client_addr: &SocketAddr,
    ) -> Result<(), ProximaError> {
        // Test for Basic authorization
        if let Some(auth) = &self.basic {
            if let Err(e) = auth.authorize(headers, method, client_addr).await {
                match e {
                    ProximaError::UnmatchedHeader => {
                        log::debug!("Could not match header for Basic auth");
                    }
                    _ => return Err(e),
                }
            } else {
                return Ok(());
            }
        }

        // Test for API Key authorization
        if let Some(auth) = &self.api_key {
            log::debug!("Got to api keys");
            if let Err(e) = auth.authorize(headers, method, client_addr).await {
                match e {
                    ProximaError::UnmatchedHeader => {
                        log::debug!("Could not match header for API key auth");
                    }
                    _ => return Err(e),
                }
            } else {
                return Ok(());
            }
        }

        // Test for Bearer authorization
        if let Some(auth) = &self.bearer {
            if let Err(e) = auth.authorize(headers, method, client_addr).await {
                match e {
                    ProximaError::UnmatchedHeader => {
                        log::debug!("Could not match header for Bearer auth");
                    }
                    _ => {
                        if self.jwks.is_some() {
                            log::debug!("Bearer token could not be authenticated, but jwks is also enabled on this endpoint, continuing");
                        } else {
                            return Err(e);
                        }
                    }
                }
            } else {
                return Ok(());
            }
        }

        // Test for JWKS authorization
        if let Some(auth) = &self.jwks {
            if let Err(e) = auth.authorize(headers, method, client_addr).await {
                match e {
                    ProximaError::UnmatchedHeader => {
                        log::debug!("Could not match header for JWKS auth");
                    }
                    _ => return Err(e),
                }
            } else {
                return Ok(());
            }
        }

        // Test for Digest authorization. Because Digest auth requires specific headers be returned to the client,
        // this test must always be last, at least for now
        if let Some(auth) = &self.digest {
            if let Err(e) = auth.authorize(headers, method, client_addr).await {
                match e {
                    ProximaError::UnmatchedHeader => {
                        log::debug!("Could not match header for Digest auth");
                    }
                    // This is a unique response, as bad digest logins require special response headers
                    _ => return Err(ProximaError::UnauthorizedClientDigest),
                }
            } else {
                return Ok(());
            }
        }

        // If we get here, no authentication was matched, return error.
        // If Basic auth is included, pass along x-auth header
        if let Some(_) = &self.basic {
            Err(ProximaError::UnauthorizedClientBasic)
        } else {
            Err(ProximaError::Unauthorized)
        }
    }
}

pub fn display_security(item: &Option<Security>) -> bool {
    if let Some(security) = item {
        if security.whitelist.is_some() {
            false
        } else {
            true
        }
    } else {
        true
    }
}

impl Whitelist {
    pub fn authorize(&self, method: &Method, client_addr: &SocketAddr) -> Result<(), ProximaError> {
        // Authorize methods
        if let Some(ref methods) = self.methods {
            log::debug!("\"The method whitelist allows: {:?}\"", methods);
            metrics::increment_counter!("proxima_security_method_authorize_attempts_total");
            match methods.contains(&method.to_string()) {
                true => {
                    log::debug!("\"{} is in whitelist\"", method);
                }
                false => {
                    metrics::increment_counter!("proxima_security_method_authorize_blocked_count");
                    log::info!("\"Blocked {} method\"", method);
                    return Err(ProximaError::Forbidden);
                }
            }
        }

        // Authorize client IP, placeholder to compile
        if let Some(ref networks) = self.networks {
            log::debug!("\"The IP whitelist allows: {:?}\"", networks);
            metrics::increment_counter!("proxima_security_network_authorize_attempts_total");
            for network in networks {
                if network.contains(client_addr.ip()) {
                    log::debug!(
                        "\"client IP {} is in IP whitelisted network {}\"",
                        client_addr,
                        network
                    );
                    return Ok(());
                }
            }
            metrics::increment_counter!("proxima_security_network_authorize_blocked_count");
            log::info!("\"Blocked client {}\"", client_addr);
            return Err(ProximaError::Forbidden);
        }
        Ok(())
    }
}
