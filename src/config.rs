use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use axum::http::Request;
use hyper::Body;
use std::error::Error;
use crate::https::HttpsClient;

pub type ConfigHash = HashMap<String, ConfigEntry>;
type BoxResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct ConfigEntry {
    pub url: Url,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub username: String,

    #[serde(default)]
    #[serde(skip_serializing)]
    pub password: String,

    #[serde(default)]
    #[serde(skip_serializing)]
    pub token: String,
}

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct Url(String);

impl Default for Url {
    fn default() -> Self {
        Url("string".to_string())
    }
}

// Add ability to use to_string() with Url
impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub async fn parse(client: HttpsClient, location: &str) -> BoxResult<HashMap<String, ConfigEntry>> {

    let deck: HashMap<String, ConfigEntry> = match url::Url::parse(location) {
        Ok(url) => {
            log::debug!("config location is url: {}", &location);

            // Create new get request
            let req = Request::builder()
                .method("GET")
                .uri(url.to_string())
                .body(Body::empty())
                .expect("request builder");

            // Send request
            let response = match client.request(req).await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("{{\"error\":\"{}\"", e);
                    return Err(Box::new(e))
                }
            };

            let contents = hyper::body::to_bytes(response.into_body()).await?;

            serde_json::from_slice(&contents)?
        },
        Err(e) => {
            log::debug!("\"config location {} is not Url: {}\"", &location, e);
            let mut file = File::open(location).expect("Unable to open config");
            let mut contents = String::new();

            file.read_to_string(&mut contents)
                .expect("Unable to read config");

            serde_yaml::from_str(&contents)?
        }
    };

    Ok(deck)
}
