use crate::https::HttpsClient;
use axum::http::Request;
use chrono::Utc;
use hyper::header::HeaderValue;
use hyper::HeaderMap;
use hyper::Method;
use hyper::{Body, Uri};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
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
    last_read: Arc<Mutex<i64>>,
    client_id: String,
    #[serde(skip_serializing)]
    client_secret: String,
    #[serde(skip_serializing)]
    grant_type: String,
    #[serde(default)]
    #[serde(skip_serializing)]
    jwt: Arc<Mutex<Value>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[warn(non_camel_case_types)]
pub struct JwtQueries {
    audience: String,
    scopes: Vec<String>,
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
    pub async fn token(&self) -> Result<Option<String>, ProximaError> {
        self.renew().await?;
        let jwt = self.jwt().await?;
        let token = match jwt["token"].as_str() {
            Some(v) => Some(v.to_string()),
            None => None
        };
        Ok(token)
    }

    pub async fn jwt(&self) -> Result<Value, ProximaError> {
        let jwt = self.jwt.lock().unwrap();
        Ok(jwt.clone())
    }

    pub async fn get_jwt(&self) -> Result<(), ProximaError> {

        let jwt_queries = JwtQueries {
            audience: self.audience.clone(),
            scopes: self.scopes.clone(),
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            grant_type: self.grant_type.clone()
        };

        let query = serde_urlencoded::to_string(jwt_queries).map_err(|_| ProximaError::JwtDecode)?;
        let url = format!("{}?{}", self.url, query);
        let uri = Uri::try_from(url)?;

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

        // Save jwt
        let mut jwt = self.jwt.lock().unwrap();
        *jwt = body;

        // Set last_read field
        let now = Utc::now().timestamp();
        let mut last_read = self.last_read.lock().unwrap();
        *last_read = now;

        Ok(())
    }

    pub async fn renew(&self) -> Result<(), ProximaError> {
        let last_read = self.last_read.lock().expect("Error getting last_read");
        let diff = Utc::now().timestamp() - *last_read;
        if diff >= 360 {
            log::debug!("jwt has expired, kicking off job to get keys");
            metrics::increment_counter!("proxima_jwt_renew_attempts_total");
            drop(last_read);

            self.get_jwt().await?;

        } else {
            log::debug!("\"jwt has not expired, current age is {} seconds\"", diff);
        }
        Ok(())
    }
}
