use axum::http::Request;
use digest_auth::{AuthContext, AuthorizationHeader};
use hyper::header::{HeaderValue, AUTHORIZATION};
use hyper::{Body, HeaderMap, Uri};
use serde::{Deserialize, Serialize};

use crate::error::Error as ProximaError;
use crate::https::HttpsClient;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[warn(non_camel_case_types)]
pub enum EndpointAuth {
    #[allow(non_camel_case_types)]
    basic(BasicAuth),
    #[allow(non_camel_case_types)]
    bearer(BearerAuth),
    #[allow(non_camel_case_types)]
    digest(DigestAuth),
}

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

impl<'a> EndpointAuth {
    pub fn authorize(&self, header: &HeaderValue) -> Result<(), ProximaError> {
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

                let client = HttpsClient::default();

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
