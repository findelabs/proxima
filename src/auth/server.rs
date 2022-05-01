use axum::http::Request;
use digest_auth::AuthContext;
use hyper::header::{HeaderValue, AUTHORIZATION};
use hyper::{Body, HeaderMap, Uri};
use serde::{Deserialize, Serialize};
use std::hash::Hash;

use crate::auth::auth;
use crate::error::Error as ProximaError;
use crate::https::ClientBuilder;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[warn(non_camel_case_types)]
pub enum ServerAuth {
    #[allow(non_camel_case_types)]
    basic(auth::BasicAuth),
    #[allow(non_camel_case_types)]
    bearer(auth::BearerAuth),
    #[allow(non_camel_case_types)]
    digest(auth::DigestAuth),
}

impl<'a> ServerAuth {
    pub async fn headers(
        &self,
        headers: &'a mut HeaderMap,
        uri: &Uri,
    ) -> Result<&'a mut HeaderMap, ProximaError> {
        match self {
            ServerAuth::basic(auth) => {
                log::debug!("Generating Basic auth headers");
                let basic_auth = auth.basic();
                let header_basic_auth = match HeaderValue::from_str(&basic_auth) {
                    Ok(a) => a,
                    Err(e) => {
                        log::error!("{{\"error\":\"{}\"", e);
                        return Err(ProximaError::BadUserPasswd);
                    }
                };
                headers.insert(AUTHORIZATION, header_basic_auth);
                Ok(headers)
            }
            ServerAuth::bearer(auth) => {
                log::debug!("Generating Bearer auth headers");
                let bearer_auth = format!("Bearer {}", auth.token());
                let header_bearer_auth = match HeaderValue::from_str(&bearer_auth) {
                    Ok(a) => a,
                    Err(e) => {
                        log::error!("{{\"error\":\"{}\"", e);
                        return Err(ProximaError::BadToken);
                    }
                };
                headers.insert(AUTHORIZATION, header_bearer_auth);
                Ok(headers)
            }
            ServerAuth::digest(auth) => {
                log::debug!("Generating Digest auth headers");

                let req = Request::builder()
                    .method("GET")
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request builder");

                let client = ClientBuilder::new()
                    .accept_invalid_certs(true)
                    .build()
                    .unwrap();

                // Send initial request
                let response = match client.request(req).await {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("{{\"error\":\"{}\"", e);
                        return Err(ProximaError::Hyper(e));
                    }
                };

                // Error if status code is not 200
                let mut prompt = match response.status().as_u16() {
                    401 => match response.headers().get("WWW-Authenticate") {
                        Some(www_authenticate) => {
                            match digest_auth::parse(www_authenticate.to_str().unwrap_or("error")) {
                                Ok(p) => p,
                                Err(e) => {
                                    log::error!("error parsing www-authenticate header: {}", e);
                                    return Ok(headers);
                                }
                            }
                        }
                        None => {
                            log::error!("Inital request did not yield www-authenticate header");
                            return Ok(headers);
                        }
                    },
                    _ => return Ok(headers),
                };

                // Generate Digest Header
                let context =
                    AuthContext::new(auth.username.clone(), auth.password.clone(), uri.path());
                let answer = match prompt.respond(&context) {
                    Ok(auth_header) => auth_header.to_string(),
                    Err(e) => {
                        log::error!("error computing header: {}", e);
                        return Ok(headers);
                    }
                };

                let header_digest_auth = match HeaderValue::from_str(&answer) {
                    Ok(a) => a,
                    Err(e) => {
                        log::error!("{{\"error\":\"{}\"", e);
                        return Err(ProximaError::BadToken);
                    }
                };
                headers.insert(AUTHORIZATION, header_digest_auth);
                Ok(headers)
            }
        }
    }
}
