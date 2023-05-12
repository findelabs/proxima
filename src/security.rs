use async_trait::async_trait;
use hyper::HeaderMap;
use hyper::Method;
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::auth::anonymous::AnonymousAuth;
use crate::auth::api_key::ApiKeyAuth;
use crate::auth::basic::BasicAuth;
use crate::auth::bearer::BearerAuth;
use crate::auth::digest::DigestAuth;
use crate::auth::jwks::JwksAuthList;
use crate::auth::traits::{AuthList, Authorize, AuthorizeList};
use crate::error::Error as ProximaError;

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
    pub basic: Option<AuthList<BasicAuth>>,
    pub digest: Option<AuthList<DigestAuth>>,
    pub bearer: Option<AuthList<BearerAuth>>,
    pub jwks: Option<JwksAuthList>,
    pub api_key: Option<AuthList<ApiKeyAuth>>,
    pub anonymous: Option<AnonymousAuth>,
}

impl EndpointSecurity for Security {
    fn security(&self) -> Option<&Security> {
        Some(self)
    }
}

#[async_trait]
pub trait EndpointSecurity {
    fn security(&self) -> Option<&Security>;

    fn whitelist(&self) -> Option<&Whitelist> {
        if let Some(security) = self.security() {
            security.whitelist.as_ref()
        } else {
            None
        }
    }

    async fn auth(
        &self,
        headers: &HeaderMap,
        method: &Method,
        client: &SocketAddr,
    ) -> Result<(), ProximaError> {
        self.authorize_whitelist(method, client).await?;
        self.authenticate_client(headers, method, client).await?;
        Ok(())
    }

    async fn authorize_whitelist(
        &self,
        method: &Method,
        client_addr: &SocketAddr,
    ) -> Result<(), ProximaError> {
        // If endpoint has a method whitelock, verify
        if let Some(whitelist) = self.whitelist() {
            log::debug!("Found whitelist");
            whitelist.authorize(method, client_addr)?
        }
        Ok(())
    }

    async fn authenticate_client(
        &self,
        headers: &HeaderMap,
        method: &Method,
        client_addr: &SocketAddr,
    ) -> Result<(), ProximaError> {
        // If endpoint is locked down, verify credentials
        let security = self.security();
        if let Some(security) = security {
            if let Some(clientlist) = &security.client {
                log::debug!("Proxy is locked");
                clientlist.authorize(headers, method, client_addr).await?
            }
        }
        Ok(())
    }
}

impl AuthorizedClients {
    pub async fn authorize(
        &self,
        headers: &HeaderMap,
        method: &Method,
        client_addr: &SocketAddr,
    ) -> Result<(), ProximaError> {
        // Test for Anonymous authorization
        if let Some(auth) = &self.anonymous {
            match auth.authorize(headers, method, client_addr).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    log::debug!("Anonymous client was blocked: {e}");
                }
            }
        }

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
        if self.basic.is_some() {
            Err(ProximaError::UnauthorizedClientBasic)
        } else {
            Err(ProximaError::Unauthorized)
        }
    }
}

pub fn display_security(item: &Option<Security>) -> bool {
    if let Some(security) = item {
        security.whitelist.is_none()
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
                    log::debug!("\"Blocked {} method\"", method);
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
            log::debug!("\"Blocked client {}\"", client_addr);
            return Err(ProximaError::Forbidden);
        }
        Ok(())
    }
}
