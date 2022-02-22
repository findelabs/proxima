use crate::error::Error as RestError;
use crate::https::HttpsClient;
use axum::{
    http::{Request},
};
use hyper::header::{HeaderValue, AUTHORIZATION};
use hyper::Body;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use url::Url;

type BoxResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub type ConfigMap = HashMap<String, Entry>;

pub struct ProxyPath(String);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
	pub static_config: ConfigMap
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BasicAuth {
    pub username: String,

    #[serde(deserialize_with = "hide_string")]
    pub password: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BearerAuth {
    #[serde(deserialize_with = "hide_string")]
    pub token: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[warn(non_camel_case_types)]
#[serde(untagged)]
pub enum Entry {
    #[allow(non_camel_case_types)]
    ConfigMap(Box<ConfigMap>),
    #[allow(non_camel_case_types)]
    Endpoint(Endpoint)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[warn(non_camel_case_types)]
pub enum EndpointAuth {
    #[allow(non_camel_case_types)]
    basic(BasicAuth),
    #[allow(non_camel_case_types)]
    bearer(BearerAuth)
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
        basic_auth
    }

}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Endpoint {
    pub url: Url,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<EndpointAuth>
}

fn hide_string<'de, D>(d: D) -> Result<String, D::Error>
	where D: Deserializer<'de>,
{
   	let s = String::deserialize(d)?;
	let hidden: String = s.chars().enumerate()
		.filter(|(i, _)| i < &16)
		.map(|_| '*')
		.collect();
	Ok(hidden)
}

pub async fn parse(
    client: HttpsClient,
    location: &str,
    config_auth: Option<String>,
) -> BoxResult<Config> {
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
                    return Err(Box::new(e));
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
                }
                _ => Err(Box::new(RestError::UnkError)),
            }
        }
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
