use axum::{
    http::{uri::Uri, StatusCode},
    http::{Request, Response},
    Json,
};
use hyper::{
    header::{AUTHORIZATION, HeaderValue, CONTENT_TYPE},
    Body, HeaderMap, Method,
};
use serde_json::Value;
use std::convert::TryFrom;
use crate::config::{ConfigEntry, ConfigHash};
use crate::https::HttpsClient;
use crate::config;
use std::error::Error;
use tokio::sync::RwLock;
use clap::ArgMatches;
use std::sync::Arc;
use base64;

use crate::create_https_client;

type BoxResult<T> = Result<T,Box<dyn Error + Send + Sync>>;

#[derive(Clone, Debug)]
pub struct State {
    pub config_path: String,
    pub config: Arc<RwLock<ConfigHash>>,
    pub client: HttpsClient,
}

impl State {
    pub async fn new(opts: ArgMatches<'_>) -> BoxResult<Self> {

		// Set timeout
	    let timeout: u64 = opts.value_of("timeout").unwrap().parse().unwrap_or_else(|_| {
	        eprintln!("Supplied timeout not in range, defaulting to 60");
	        60
	    });

		let client = create_https_client(timeout)?;
	    let config_path = opts.value_of("config").unwrap().to_owned();
	    let config = config::parse(&config_path)?;
	
	    Ok(State {
	        config_path,
	        config: Arc::new(RwLock::new(config)),
	        client: client,
	    })
    }

    pub async fn get_entry(&self, item: &str) -> Option<ConfigEntry> {
        log::debug!("Getting {} from ConfigHash", &item);
        let config = self.config.read().await;
        let entry = config.get(item);
        entry.cloned()
    }

    pub async fn config(&self) -> Value {
        let config = self.config.read().await;
        serde_json::to_value(&*config).expect("Cannot convert to JSON")
    }

    pub async fn reload(&mut self) {
        let mut config = self.config.write().await;
        let new_config = match config::parse(&self.config_path) {
            Ok(e) => e,
            Err(e) => {
                log::error!("Could not parse config: {}", e);
                config.clone()
            }
        };
        *config = new_config;
    }

    pub async fn response(
        &self,
        method: Method,
        endpoint: &str,
        path: &str,
        query: Option<String>,
        mut all_headers: HeaderMap,
        payload: Option<Json<Value>>,
    ) -> Response<Body> {

        let config_entry = match self.get_entry(endpoint).await {
            Some(e) => e,
            None => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("{\"error\": \"please specify known endpoint\"}"))
                    .unwrap()
            }
        };

        let path = path.replace(" ", "%20");

        let host_and_path = match query {
            Some(q) => format!("{}{}?{}", config_entry.url, path, q),
            None => format!("{}{}", config_entry.url, path)
        };

        log::debug!("full uri: {}", host_and_path);

        match Uri::try_from(host_and_path) {
            Ok(u) => {
                let body = match payload {
                    Some(p) => {
                        log::info!("Received body: {}", &p.to_string());
                        Body::from(p.to_string())
                    }
                    None => {
                        log::debug!("Did not receive a body");
                        Body::empty()
                    }
                };

                let mut req = Request::builder()
                    .method(method)
                    .uri(u)
                    .body(body)
                    .expect("request builder");


                // Append to request the headers passed by client
                all_headers.remove(hyper::header::HOST);
                all_headers.remove(hyper::header::USER_AGENT);
                if !all_headers.contains_key(CONTENT_TYPE) {
                    all_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                };
                let headers = req.headers_mut();
                headers.extend(all_headers.clone());

                // Added Basic Auth if username/password exist
                if config_entry.username != "" {
                    log::debug!("Generating Basic auth");
                    let user_pass = format!("{}:{}", config_entry.username, config_entry.password);
                    let encoded = base64::encode(user_pass);
                    let basic_auth = format!("Basic {}", encoded);
                    let header_basic_auth = match HeaderValue::from_str(&basic_auth) {
                        Ok(a) => a,
                        Err(e) => {
                            log::error!("{}", e);
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(
                                    "{\"error\": \"Unparsable username and password provided\"}",
                                ))
                                .unwrap()
                        }
                    };
                    headers.insert(AUTHORIZATION, header_basic_auth);
                };

                match self.client.request(req).await {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("{}", e);
                        Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from(
                                "{\"error\": \"Error connecting to rest endpoint\"}",
                            ))
                            .unwrap()
                    }
                }
            }
            Err(_) => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("{\"error\": \"Error parsing uri\"}"))
                .unwrap(),
        }
    }
}
