use crate::https::HttpsClient;
use axum::http::Request;
use chrono::Utc;
use hyper::{Body, Uri};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use url::Url;

use crate::error::Error as ProximaError;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
#[warn(non_camel_case_types)]
pub struct JwtAuth {
    url: Url,
    audience: String,
    scopes: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    client: HttpsClient,
    #[serde(default)]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    expires: Arc<Mutex<i64>>,
    client_id: String,
    #[serde(skip_serializing)]
    client_secret: String,
    grant_type: String,
    #[serde(default)]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    jwt: Arc<Mutex<Value>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[warn(non_camel_case_types)]
pub struct JwtQueries {
    audience: String,
    scopes: String,
    client_id: String,
    client_secret: String,
    grant_type: String
}

impl Hash for JwtAuth {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.url.hash(state);
        self.audience.hash(state);
        self.scopes.hash(state);
        self.client_id.hash(state);
        self.client_secret.hash(state);
        self.grant_type.hash(state);
    }
}

impl JwtAuth {
    pub async fn token(&self) -> Result<String, ProximaError> {
        self.renew().await?;
        let jwt = self.jwt().await?;
        let token = match jwt["access_token"].as_str() {
            Some(v) => v.to_string(),
            None => return Err(ProximaError::JwtDecode)
        };
        Ok(token)
    }

    pub async fn jwt(&self) -> Result<Value, ProximaError> {
        let jwt = self.jwt.lock().unwrap();
        Ok(jwt.clone())
    }

    pub async fn expiration(&self) -> i64 {
        let expires = self.expires.lock().unwrap();
        *expires
    }

    pub async fn get_jwt(&self) -> Result<(), ProximaError> {

        let jwt_queries = JwtQueries {
            audience: self.audience.clone(),
            scopes: self.scopes.join(","),
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            grant_type: self.grant_type.clone()
        };

        let query = serde_urlencoded::to_string(jwt_queries).map_err(|_| ProximaError::JwtDecode)?;
        let url = format!("{}?{}", self.url, query);
        let uri = Uri::try_from(url)?;

        log::debug!("Get'ing JWT from {}", &uri);

        let req = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/x-www-form-urlencoded")
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

        // Get expiration time
        let expiration = match body["expires_in"].as_i64() {
            Some(t) => {
                log::debug!("Token expires in {}", &t);
                t
            },
            None => {
                log::debug!("Could not detect token expiration, defaulting to 360");
                360i64
            }
        };

        // Save jwt
        let mut jwt = self.jwt.lock().unwrap();
        *jwt = body;

        // Set last_read field
        let now = Utc::now().timestamp() + expiration;
        let mut last_read = self.expires.lock().unwrap();
        *last_read = now;

        Ok(())
    }

    pub async fn renew(&self) -> Result<(), ProximaError> {
        let expiration = self.expiration().await;
        let now = Utc::now().timestamp();
        if expiration - now <= 360 {
            log::debug!("jwt is expiring, kicking off job to get new token");
            metrics::increment_counter!("proxima_jwt_renew_attempts_total");
            drop(expiration);

            self.get_jwt().await?;

        } else {
            log::debug!("\"Reusing JWT, as it has not expired\"");
        }
        Ok(())
    }
}
