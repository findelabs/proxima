use axum::http::Request;
use digest_auth::{AuthContext, AuthorizationHeader};
use hyper::header::{HeaderValue, AUTHORIZATION};
use hyper::{Body, HeaderMap, Uri};
use serde::{Deserialize, Serialize};
use jsonwebtoken::jwk::AlgorithmParameters;
use jsonwebtoken::{decode, decode_header, jwk, DecodingKey, Validation};
use url::Url;
use std::sync::{Arc, Mutex};
use serde_json::Value;
use async_recursion::async_recursion;
use crate::https::HttpsClient;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::error::Error as ProximaError;
use crate::https::ClientBuilder;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[warn(non_camel_case_types)]
pub enum EndpointAuth {
    #[allow(non_camel_case_types)]
    basic(BasicAuth),
    #[allow(non_camel_case_types)]
    bearer(BearerAuth),
    #[allow(non_camel_case_types)]
    digest(DigestAuth),
    #[allow(non_camel_case_types)]
    jwks(JwksAuth),
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct EndpointAuthArray(Vec<EndpointAuth>);

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct BasicAuth {
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct DigestAuth {
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct BearerAuth {
    #[serde(skip_serializing)]
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[warn(non_camel_case_types)]
pub struct JwksAuth {
    url: Url,
    audience: String,
    scopes: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing)]
    jwks: Arc<Mutex<Value>>,
    #[serde(default)]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    client: HttpsClient,
    #[serde(default)]
    validate_audience: bool,
    #[serde(default)]
    validate_expiration: bool,
    #[serde(default)]
    validate_scopes: bool
}

impl Hash for JwksAuth {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.url.hash(state);
        self.audience.hash(state);
    }
}

impl EndpointAuthArray {
    pub async fn authorize(&self, header: &HeaderValue) -> Result<(), ProximaError> {
        let Self(internal) = self;
        for user in internal.iter() {
            log::debug!("\"Checking if client auth against {:?}\"", user);
            match user.authorize(header).await {
                Ok(_) => return Ok(()),
                Err(_) => continue,
            }
        }
        log::debug!("\"Client could not be authenticated\"");
        Err(ProximaError::UnauthorizedUser)
    }
}

impl<'a> EndpointAuth {
    pub async fn authorize(&self, header: &HeaderValue) -> Result<(), ProximaError> {
        metrics::increment_counter!("proxima_endpoint_authentication_total");
        match self {
            EndpointAuth::basic(auth) => {
                if HeaderValue::from_str(&auth.basic()).unwrap() != header {
                    metrics::increment_counter!(
                        "proxima_endpoint_authentication_basic_failed_total"
                    );
                    return Err(ProximaError::UnauthorizedUser);
                }
            }
            EndpointAuth::bearer(auth) => {
                if HeaderValue::from_str(&auth.token()).unwrap() != header {
                    metrics::increment_counter!(
                        "proxima_endpoint_authentication_bearer_failed_total"
                    );
                    return Err(ProximaError::UnauthorizedUser);
                }
            }
            EndpointAuth::digest(auth) => {
                let client_authorization_header =
                    match AuthorizationHeader::parse(header.to_str().unwrap()) {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!("Error converting client authorization header: {}", e);
                            return Err(ProximaError::UnauthorizedDigestUser);
                        }
                    };

                let context = AuthContext::new(
                    auth.username.clone(),
                    auth.password.clone(),
                    &client_authorization_header.uri,
                );
                let mut server_authorization_header = client_authorization_header.clone();
                server_authorization_header.digest(&context);

                if server_authorization_header != client_authorization_header {
                    metrics::increment_counter!(
                        "proxima_endpoint_authentication_digest_failed_total"
                    );
                    return Err(ProximaError::UnauthorizedDigestUser);
                }
            }
            EndpointAuth::jwks(auth) => {
                let authorize = header.to_str().expect("Cannot convert header to string");
                let token: Vec<&str> = authorize.split(" ").collect();
                if let Err(_) = auth.validate(token[1]).await {
                    metrics::increment_counter!(
                        "proxima_endpoint_authentication_bearer_failed_total"
                    );
                    return Err(ProximaError::UnauthorizedUser)
                }
            }
        }
        Ok(())
    }

    pub async fn headers(
        &self,
        headers: &'a mut HeaderMap,
        uri: &Uri,
    ) -> Result<&'a mut HeaderMap, ProximaError> {
        match self {
            EndpointAuth::basic(auth) => {
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
            EndpointAuth::bearer(auth) => {
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
            EndpointAuth::digest(auth) => {
                log::debug!("Generating Digest auth headers");

                let req = Request::builder()
                    .method("GET")
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request builder");

                let client = ClientBuilder::new().accept_invalid_certs(true).build().unwrap();

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
            EndpointAuth::jwks(_auth) => {
                log::debug!("Jwts is not currently usable for remote url auth");
                let auth = format!("Bearer xxxxxxx");
                let header_auth = match HeaderValue::from_str(&auth) {
                    Ok(a) => a,
                    Err(e) => {
                        log::error!("{{\"error\":\"{}\"", e);
                        return Err(ProximaError::BadToken);
                    }
                };
                headers.insert(AUTHORIZATION, header_auth);
                Ok(headers)
            }
        }
    }
}

impl BearerAuth {
    pub fn token(&self) -> String {
        self.token.clone()
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
}

impl JwksAuth {
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
                let string: Value  = serde_json::from_slice(&contents)?;
                string
            }
            _ => {
                println!(
                    "Got bad status code getting config: {}",
                    response.status().as_u16()
                );
                return Err(ProximaError::Unknown)
            }
        };

        let mut jwks = self.jwks.lock().unwrap();
        *jwks = body;

        Ok(())
    }

    #[async_recursion]
    pub async fn keys(&self) -> Result<jwk::JwkSet, ProximaError> {
        let jwks = self.jwks.lock().unwrap().clone();
        match jwks {
            Value::Null => {
                println!("Getting keys");
                self.get_keys().await?;
                self.keys().await
            },
            _ => {
                println!("Returning known keys");
                let j: jwk::JwkSet = serde_json::from_value(jwks)?;
                Ok(j)
            }
        }
    }

    pub async fn validate(&self, token: &str) -> Result<(), Box<dyn std::error::Error>> {
        let jwks = self.keys().await?;
        let header = decode_header(&token)?;
        let kid = match header.kid {
            Some(k) => k,
            None => return Err("Token doesn't have a `kid` header field".into()),
        };

        if let Some(j) = jwks.find(&kid) {
            match j.algorithm {
                AlgorithmParameters::RSA(ref rsa) => {
                    let decoding_key = DecodingKey::from_rsa_components(&rsa.n, &rsa.e).expect("Unable to generate decoding key");
                    let algo = j.common.algorithm.expect("missing algorithm");
                    let mut validation = Validation::new(algo);

                    // Ensure token is not expired
                    if self.validate_expiration {
                        println!("Will validate expiration");
                        validation.validate_exp = true;
                    }

                    // Ensure token is not born yet
                    validation.validate_nbf = true;

                    // Validate audience
                    if self.validate_audience {
                        println!("Will validate audience");
                        validation.set_audience(&vec!(&self.audience));
                    }

                    let decoded_token =
                        decode::<HashMap<String, serde_json::Value>>(&token, &decoding_key, &validation)?;
                    println!("{:?}", decoded_token);
                    Ok(())
                }
                _ => unreachable!("this should be a RSA"),
            }
        } else {
            return Err("No matching JWK found for the given kid".into());
        }
    }
}
