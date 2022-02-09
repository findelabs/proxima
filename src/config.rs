use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use axum::http::Request;
use hyper::Body;
use crate::https::HttpsClient;
use hyper::header::{AUTHORIZATION, HeaderValue};
use crate::error::Error as RestError;

pub type ConfigHash = HashMap<String, ConfigEntry>;
type BoxResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

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

pub async fn parse(client: HttpsClient, location: &str, config_auth: Option<String>) -> BoxResult<HashMap<String, ConfigEntry>> {

    // Test if config flag is url
    match url::Url::parse(location) {
        Ok(url) => {
            log::debug!("config location is url: {}", &location);

            // Create new get request
            let mut req = Request::builder()
                .method("GET")
                .uri(url.to_string())
                .body(Body::empty())
                .expect("request builder");

			// Add in basic auth if required
			let headers = req.headers_mut();
			if config_auth.is_some() {
                log::debug!("Inserting basic auth for config endpoint");
                let header_basic_auth = HeaderValue::from_str(&config_auth.unwrap())?;
				headers.insert(AUTHORIZATION, header_basic_auth);
			};

            // Send request
            let response = match client.request(req).await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("{{\"error\":\"{}\"", e);
                    return Err(Box::new(e))
                }
            };

			// Error if status code is not 200
			match response.status().as_u16() {
            	404 => Err(Box::new(RestError::NotFound)),
            	403 => Err(Box::new(RestError::Forbidden)),
            	401 => Err(Box::new(RestError::Unauthorized)),
            	200 => {
		            let contents = hyper::body::to_bytes(response.into_body()).await?;
		            Ok(serde_json::from_slice(&contents)?)
            	},
            	_ => Err(Box::new(RestError::UnkError))
	        }

        },
        Err(e) => {
            log::debug!("\"config location {} is not Url: {}\"", &location, e);
            let mut file = File::open(location).expect("Unable to open config");
            let mut contents = String::new();

            file.read_to_string(&mut contents)
                .expect("Unable to read config");

            Ok(serde_yaml::from_str(&contents)?)
        }
    }
}
