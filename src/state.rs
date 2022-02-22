use crate::config;
use crate::config::{Entry, Config};
use crate::https::HttpsClient;
use axum::{
    extract::BodyStream,
    http::uri::Uri,
    http::{StatusCode, Request, Response},
};
use chrono::offset::Utc;
use clap::ArgMatches;
use hyper::{
    header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Body, HeaderMap, Method,
};
use serde_json::Value;
use std::convert::TryFrom;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_recursion::async_recursion;

use crate::config::{EndpointAuth, ConfigMap};
use crate::create_https_client;
use crate::error::Error as RestError;

type BoxResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Debug)]
pub struct State {
    pub config_location: String,
    pub config_auth: Option<String>,
    pub config_read: Arc<RwLock<i64>>,
    pub config: Arc<RwLock<Config>>,
    pub client: HttpsClient,
}

impl State {
    pub async fn new(opts: ArgMatches<'_>) -> BoxResult<Self> {
        // Set timeout
        let timeout: u64 = opts
            .value_of("timeout")
            .unwrap()
            .parse()
            .unwrap_or_else(|_| {
                eprintln!("Supplied timeout not in range, defaulting to 60");
                60
            });

        let config_auth = match opts.value_of("config_username") {
            Some(config_username) => {
                log::debug!("Generating Basic auth for config endpoint");
                let config_password = opts.value_of("config_password").unwrap();
                let user_pass = format!("{}:{}", config_username, config_password);
                let encoded = base64::encode(user_pass);
                let basic_auth = format!("Basic {}", encoded);
                Some(basic_auth)
            }
            None => None,
        };

        let client = create_https_client(timeout)?;
        let config_location = opts.value_of("config").unwrap().to_owned();
        let config = config::parse(client.clone(), &config_location, config_auth.clone()).await?;
        let config_read = Utc::now().timestamp();

        Ok(State {
            config_location,
            config: Arc::new(RwLock::new(config)),
            client,
            config_auth,
            config_read: Arc::new(RwLock::new(config_read)),
        })
    }

	#[async_recursion]
	pub async fn get_sub_entry(&self, map: ConfigMap, endpoint: &str, remainder: &str) -> Option<(Entry, Option<String>)> {

		// Let's remove any / prefix, to make sure all remainders are the same,
		// but only if the remainder is more than just /
//		let mut remainder = match remainder.len() {
//			x if x > 1 => {
//				match remainder.chars().nth(0).unwrap_or('e') {
//					'/' => {
//						log::info!("Removing / prefix");
//						let mut rem = remainder.to_string();
//						rem.remove(0);
//						rem
//					},
//					_ => remainder.to_string()
//				}
//			},
//			_ => {
//				log::info!("remainder is one, adding slash");
//				"/".to_string()
//			}
//		};
		let mut remainder = if remainder != "" {
			match remainder.chars().nth(0).unwrap_or('e') {
				'/' => {
					log::info!("Removing / prefix");
					let mut rem = remainder.to_string();
					rem.remove(0);
					rem
				},
				_ => remainder.to_string()
			}
		} else {
			remainder.to_string()
		};

		log::info!("Searching for endpoint: {}, with remainder of {}", endpoint, remainder);

		match map.get(endpoint) {
			Some(entry) => match entry {
				Entry::ConfigMap(map) => {
					log::info!("Found config fork");
					
					// Split up remainder, getting next endpoint and remainder. Leave out the first item.
					let vec: Vec<&str> = remainder.splitn(2, "/").collect();

					log::info!("vec: {:?}, length: {}", vec, vec.len());

					match vec.len() {
						0 => {
							log::info!("Nothing passed after fork, returning forked endpaths");
							Some((config::Entry::ConfigMap(map.clone()), None))
						},
						1 => {
							match vec[0] {
								"" => {
									log::info!("Nothing passed after fork, returning forked endpaths");
									Some((config::Entry::ConfigMap(map.clone()), None))
								},
								_ => {
									log::info!("Found item path in fork, searching for endpoint");
									self.get_sub_entry(*map.clone(), vec[0], "").await
								}
							}
						},
						_ => {
							log::info!("Found item path in fork, searching for endpoint");
							self.get_sub_entry(*map.clone(), vec[0], vec[1]).await
						}
					}
				},
				Entry::Endpoint(e) => {
					// Add in forward slash prefix, if not empty
					if remainder != "" {
						log::info!("Adding in forward slash as prefix");
						remainder.insert(0, '/');
					};
					log::info!("Returning endpoint: {}, with remainder of {}", &e.url, remainder);
					Some((config::Entry::Endpoint(e.clone()), Some(remainder.to_string())))
				}
			},
			None => None
		}
	}

    pub async fn get_entry(&mut self, endpoint: &str, remainder: &str) -> Option<(Entry, Option<String>)> {
        self.renew().await;
        log::info!("Searching for {} from Config", &endpoint);
        let config = self.config.read().await;
		self.get_sub_entry(config.static_config.clone(), endpoint, remainder).await
    }

    pub async fn config(&mut self) -> Value {
        self.renew().await;
        let config = self.config.read().await;
        serde_json::to_value(&*config).expect("Cannot convert to JSON")
    }

    pub async fn renew(&mut self) {
        let config_read = self.config_read.read().await;
        let diff = Utc::now().timestamp() - *config_read;
        if diff >= 30 {
            log::debug!("cache has expired, kicking off config reload");
            drop(config_read);

            // Kick off background thread to update config
            let mut me = self.clone();
            tokio::spawn(async move {
                log::debug!("Kicking off background thread to reload config");
                me.reload().await;
            });
        } else {
            log::debug!("\"cache has not expired, current age is {} seconds\"", diff);
        }
    }

    pub async fn reload(&mut self) {
        let config = self.config.read().await;
        let new_config = match config::parse(
            self.client.clone(),
            &self.config_location,
            self.config_auth.clone(),
        )
        .await
        {
            Ok(e) => e,
            Err(e) => {
                log::error!("{{\"error\": \"Could not parse config: {}\"}}", e);
                config.clone()
            }
        };

        let config_read_time = Utc::now().timestamp();
        
        // Get mutable handle on config and config_read
        drop(config);
        let mut config = self.config.write().await;
        let mut config_read = self.config_read.write().await;

        *config = new_config;
        *config_read = config_read_time;
    }

    pub async fn response(
        &mut self,
        method: Method,
        endpoint: &str,
        path: &str,
        query: Option<String>,
        mut all_headers: HeaderMap,
        payload: Option<BodyStream>,
    ) -> Result<Response<Body>, RestError> {
        let (config_entry, path) = match self.get_entry(endpoint, path).await {
            Some((e, path)) => match e {
                Entry::ConfigMap(f) => {
					let config = serde_json::to_string(&f).expect("Cannot convert to JSON");
			        let body = Body::from(config);
			
			        return Ok(Response::builder()
			            .status(StatusCode::OK)
			            .body(body)
						.unwrap())
                },
                Entry::Endpoint(end) => {
					log::info!("Response fn found endpoint");
					(end, path)
				}
            }
            None => return Err(RestError::UnknownEndpoint),
        };

		// If the path for an endpoint is empty, return the endpoint config instead of hitting the endpoint,
		// so that fn proxy mirrors the behavior of fn endpoint
		if let Some(path) = &path {
			if path == "" {
				let config = serde_json::to_string(&config_entry).expect("Cannot convert to JSON");
				let body = Body::from(config);
				return Ok(Response::builder()
					.status(StatusCode::OK)
					.body(body)
					.unwrap())
			}
		};

		// We need to replace any spaces in the url with %20's
		let path = match path {
			Some(p) => {
				p.replace(" ", "%20")
			},
			None => "".to_string()
		};
		
		// Clean up url, so that there are no trailing forward slashes
        let url = match config_entry.url.to_string().chars().last() {
            Some(c) => match c {
                '/' => {
                    let mut url = config_entry.url.to_string();
                    url.pop();
                    url
                },
                _ => config_entry.url.to_string()
            },
            None => config_entry.url.to_string()
        };

        let host_and_path = match query {
            Some(q) => format!("{}{}?{}", url, path, q),
            None => format!("{}{}", url, path),
        };
		// Switch this back to debug
        log::info!("full uri: {}", host_and_path);

        match Uri::try_from(host_and_path) {
            Ok(u) => {
                let body = match payload {
                    Some(p) => {
                        log::debug!("Received body: {:#?}", &p);
                        Body::wrap_stream(p)
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
                    log::debug!("\"Adding content-type: application/json\"");
                    all_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                };
                let headers = req.headers_mut();
                headers.extend(all_headers.clone());

                // Added Basic Auth if username/password exist
                match config_entry.authentication {
                    Some(authentication) => match authentication {
                        EndpointAuth::basic(auth) => {
                            let basic_auth = auth.basic();
                            let header_basic_auth = match HeaderValue::from_str(&basic_auth) {
                                Ok(a) => a,
                                Err(e) => {
                                    log::error!("{{\"error\":\"{}\"", e);
                                    return Err(RestError::BadUserPasswd);
                                }
                            };
                            headers.insert(AUTHORIZATION, header_basic_auth);
                        },
                        EndpointAuth::bearer(auth) => {
                            log::debug!("Generating Bearer auth");
                            let basic_auth = format!("Bearer {}", auth.token());
                            let header_bearer_auth = match HeaderValue::from_str(&basic_auth) {
                                Ok(a) => a,
                                Err(e) => {
                                    log::error!("{{\"error\":\"{}\"", e);
                                    return Err(RestError::BadToken);
                                }
                            };
                            headers.insert(AUTHORIZATION, header_bearer_auth);
                        }
                    },
                    None => log::debug!("No authentication specified for endpoint")
                };

                match self.client.clone().request(req).await {
                    Ok(s) => Ok(s),
                    Err(e) => {
                        log::error!("{{\"error\":\"{}\"", e);
                        Err(RestError::ConnectionError)
                    }
                }
            }
            Err(e) => {
                log::error!("{{\"error\": \"{}\"}}", e);
                Err(RestError::UnparseableUrl)
            }
        }
    }
}
