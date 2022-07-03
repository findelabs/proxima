use hyper::Method;
use serde::{Deserialize, Serialize};
use hyper::HeaderMap;
use ipnetwork::IpNetwork;
use std::net::SocketAddr;

//use crate::auth::client::ClientAuthList;
use crate::error::Error as ProximaError;
use crate::auth::basic::BasicAuthList;
use crate::auth::digest::DigestAuthList;
use crate::auth::bearer::BearerAuthList;
use crate::auth::jwks::JwksAuthList;

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
    pub methods: Option<Vec<String>>,
    pub networks: Option<Vec<IpNetwork>>
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct AuthorizedClients {
    pub basic: Option<BasicAuthList>,
    pub digest: Option<DigestAuthList>,
    pub bearer: Option<BearerAuthList>,
    pub jwks: Option<JwksAuthList>
}

impl AuthorizedClients {
    pub async fn authorize(
        &self,
        headers: &HeaderMap,
        method: &Method,
        client_addr: &SocketAddr
    ) -> Result<(), ProximaError> {
        // Check if Authorization header exists
        let header = match headers.get("AUTHORIZATION") {
            Some(header) => header,
            None => {
                log::debug!("Endpoint is locked, but no authorization header found");
                metrics::increment_counter!(
                    "proxima_security_client_authentication_failed_count",
                    "type" => "absent"
                );
 
                // We need to return a special header if the endpoint accepts Digest auth or Basic auth
                if self.digest.is_some() {
                    log::debug!("Returning Digest error header");
                    return Err(ProximaError::UnauthorizedClientDigest)
                } else if self.basic.is_some() {
                    log::debug!("Returning Basic error header");
                    return Err(ProximaError::UnauthorizedClientBasic)
                } else {
                    return Err(ProximaError::Unauthorized)
                }
            }
        };

        // Check what header type is being passed
        let authorize = header.to_str().expect("Cannot convert header to string");
        let auth_scheme_vec: Vec<&str> = authorize.split(' ').collect();
        let scheme = auth_scheme_vec.into_iter().nth(0);

        // Match known schemas
        match scheme {
            Some("Basic") => {
                log::debug!("Found Basic Authorization header");
                match &self.basic {
                    Some(list) => list.authorize(header, method, client_addr).await,
                    None => Err(ProximaError::UnauthorizedClientBasic)
                }
            },
            Some("Digest") => {
                log::debug!("Found Digest Authorization header");
                match &self.digest {
                    Some(list) => list.authorize(header, method, client_addr).await,
                    None => Err(ProximaError::UnauthorizedClientDigest)
                }
            }
            Some("Bearer") => {
                log::debug!("Found Bearer Authorization header");
                if let Some(list) = &self.bearer {
                    if let Err(_) = list.authorize(header, method, client_addr).await {
                        if let Some(list) = &self.jwks {
                            list.authorize(header, method, client_addr).await
                        } else {
                            log::debug!("Client authentication failed for both Bearer and JWKS types");
                            Err(ProximaError::UnauthorizedClient)
                        }
                    } else {
                        Ok(())
                    }
                } else {
                    Err(ProximaError::UnauthorizedClient)
                }
            }
            _ => {
                log::debug!("Found Unknown Authorization header {}", scheme.unwrap());
                Err(ProximaError::Unauthorized)
            }
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
                    log::debug!("\"client IP {} is in IP whitelisted network {}\"", client_addr, network);
                    return Ok(())
                }
            }
            metrics::increment_counter!("proxima_security_network_authorize_blocked_count");
            log::info!("\"Blocked client {}\"", client_addr);
            return Err(ProximaError::Forbidden);
        }
        Ok(())
    }
}
