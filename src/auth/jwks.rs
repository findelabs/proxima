use crate::https::HttpsClient;
use async_recursion::async_recursion;
use axum::http::Request;
use chrono::Utc;
use http::header::AUTHORIZATION;
use hyper::header::HeaderValue;
use hyper::HeaderMap;
use hyper::Method;
use hyper::{Body, Uri};
use jsonwebtoken::jwk::AlgorithmParameters;
use jsonwebtoken::{decode, decode_header, jwk, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use url::Url;

use crate::error::Error as ProximaError;
use crate::security::Whitelist;

const VALIDATE_DEFAULT: bool = true;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
#[warn(non_camel_case_types)]
pub struct JwksAuth {
    url: Url,
    audience: String,
    scopes: Option<Vec<String>>,
    #[serde(default)]
    #[serde(skip_serializing)]
    jwks: Arc<Mutex<Value>>,
    #[serde(default)]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    client: HttpsClient,
    #[serde(default = "validate_default")]
    validate_audience: bool,
    #[serde(default = "validate_default")]
    validate_expiration: bool,
    #[serde(default = "validate_default")]
    validate_scopes: bool,
    #[serde(default)]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    last_read: Arc<Mutex<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct JwksAuthList(Vec<JwksAuth>);

impl JwksAuthList {
    pub async fn authorize(
        &self,
        headers: &HeaderMap,
        method: &Method,
        client_addr: &SocketAddr,
    ) -> Result<(), ProximaError> {
        log::debug!("Looping over jwks users");
        let Self(internal) = self;

        let header = match headers.get(AUTHORIZATION) {
            Some(header) => header,
            None => {
                log::debug!("Endpoint is locked, but no bearer authorization header found");
                metrics::increment_counter!(
                    "proxima_security_client_authentication_failed_count",
                    "type" => "absent"
                );
                return Err(ProximaError::UnauthorizedClient);
            }
        };

        // Check if the header is JWKS
        let authorize = header.to_str().expect("Cannot convert header to string");
        let auth_scheme_vec: Vec<&str> = authorize.split(' ').collect();
        let scheme = auth_scheme_vec.into_iter().nth(0);

        // If header is not Bearer, return err
        if let Some(k) = scheme {
            if k.to_lowercase() == "bearer" {
                log::debug!("Found correct authorization scheme for JWKS: bearer");
            } else {
                log::debug!(
                    "Auth type {} does not match required 'bearer'",
                    k.to_lowercase()
                );
                return Err(ProximaError::UnmatchedHeader);
            }
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
        log::warn!("\"Client could not be authenticated\"");
        Err(ProximaError::UnauthorizedClient)
    }
}

fn validate_default() -> bool {
    VALIDATE_DEFAULT
}

impl Hash for JwksAuth {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.url.hash(state);
        self.audience.hash(state);
    }
}

impl JwksAuth {
    pub async fn authorize(
        &self,
        header: &HeaderValue,
        method: &Method,
        client_addr: &SocketAddr,
    ) -> Result<(), ProximaError> {
        let authorize = header.to_str().expect("Cannot convert header to string");
        let token: Vec<&str> = authorize.split(' ').collect();
        if (self.validate(token[1]).await).is_err() {
            metrics::increment_counter!(
                "proxima_security_client_authentication_failed_count",
                "type" => "jwks"
            );
            return Err(ProximaError::UnauthorizedClient);
        }
        if let Some(ref whitelist) = self.whitelist {
            log::debug!("Found whitelist");
            whitelist.authorize(method, client_addr)?
        }
        Ok(())
    }

    pub async fn get_keys(&self) -> Result<(), ProximaError> {
        let uri = Uri::try_from(self.url.to_string())?;

        let req = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .expect("request builder");

        let response = self.client.request(req).await?;

        let body = match response.status().as_u16() {
            200 => {
                let contents = hyper::body::to_bytes(response.into_body()).await?;
                let string: Value = serde_json::from_slice(&contents)?;
                string
            }
            _ => {
                log::debug!(
                    "Got bad status code getting config: {}",
                    response.status().as_u16()
                );
                return Err(ProximaError::Unknown);
            }
        };

        // Save jwks
        let mut jwks = self.jwks.lock().unwrap();
        *jwks = body;

        // Set last_read field
        let now = Utc::now().timestamp();
        let mut last_read = self.last_read.lock().unwrap();
        *last_read = now;

        Ok(())
    }

    #[async_recursion]
    pub async fn keys(&self) -> Result<jwk::JwkSet, ProximaError> {
        let jwks = self.jwks.lock().unwrap().clone();
        match jwks {
            Value::Null => {
                log::debug!("Getting keys");
                self.get_keys().await?;
                self.keys().await
            }
            _ => {
                log::debug!("Returning known keys");
                let j: jwk::JwkSet = serde_json::from_value(jwks)?;
                Ok(j)
            }
        }
    }

    pub async fn renew(&self) {
        let last_read = self.last_read.lock().expect("Error getting last_read");
        let diff = Utc::now().timestamp() - *last_read;
        if diff >= 360 {
            log::debug!("jwks has expired, kicking off job to get keys");
            metrics::increment_counter!("proxima_jwks_renew_attempts_total");
            drop(last_read);

            // Kick off background thread to update config
            let me = self.clone();
            tokio::spawn(async move {
                log::debug!("Kicking off background thread to renew jwks");
                if let Err(e) = me.get_keys().await {
                    log::error!("Error gettings updated jwks: {}", e);
                    metrics::increment_counter!("proxima_jwks_renew_failures_total");
                }
            });
        } else {
            log::debug!("\"jwks has not expired, current age is {} seconds\"", diff);
        }
    }

    pub async fn validate(&self, token: &str) -> Result<(), ProximaError> {
        self.renew().await;
        let jwks = self.keys().await?;
        let header = decode_header(token)?;
        let kid = match header.kid {
            Some(k) => k,
            None => {
                log::warn!("\"Token doesn't have a `kid` header field\"");
                return Err(ProximaError::JwtDecode);
            }
        };

        if let Some(j) = jwks.find(&kid) {
            match j.algorithm {
                AlgorithmParameters::RSA(ref rsa) => {
                    let decoding_key = match DecodingKey::from_rsa_components(&rsa.n, &rsa.e) {
                        Ok(k) => k,
                        Err(e) => {
                            log::warn!("\"Error decoding key: {}\"", e);
                            return Err(ProximaError::JwtDecode);
                        }
                    };
                    let algo = j.common.algorithm.expect("missing algorithm");
                    let mut validation = Validation::new(algo);

                    // Ensure token is not expired
                    if self.validate_expiration {
                        log::debug!("Will validate expiration");
                        validation.validate_exp = true;
                    }

                    // Ensure token is not born yet
                    validation.validate_nbf = true;

                    // Validate audience
                    if self.validate_audience {
                        log::debug!("Will validate audience");
                        validation.set_audience(&[&self.audience]);
                    }

                    log::debug!("Attempting to decode token");
                    let decoded_token = match decode::<HashMap<String, serde_json::Value>>(
                        token,
                        &decoding_key,
                        &validation,
                    ) {
                        Ok(e) => Ok(e),
                        Err(e) => {
                            log::debug!("Unable to decode token: {}", e);
                            Err(e)
                        }
                    }?;
                    log::debug!("decoded token: {:?}", decoded_token);

                    if self.scopes.is_some() && self.validate_scopes {
                        let scp = match decoded_token.claims.get("scp") {
                            Some(scopes) => {
                                let vec_values =
                                    scopes.as_array().expect("Unable to convert to array");
                                let vec_string = vec_values
                                    .iter()
                                    .map(|s| s.to_string().replace('"', ""))
                                    .collect();
                                vec_string
                            }
                            None => Vec::new(),
                        };

                        // Ensure that all required scopes are contained with the JWT.scp field
                        for scope in self.scopes.as_ref().unwrap().iter() {
                            if !scp.contains(scope) {
                                log::debug!(
                                    "\"Blocking client as JWT.scp does not contain {}\"",
                                    scope
                                );
                                return Err(ProximaError::UnauthorizedClient);
                            }
                        }
                    }
                    Ok(())
                }
                _ => Err(ProximaError::JwtDecode),
            }
        } else {
            log::warn!("\"No matching JWK found for the given kid\"");
            Err(ProximaError::JwtDecode)
        }
    }
}
