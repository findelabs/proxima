use axum::{
    http::uri::Uri,
    http::{Request, Response},
};
use hyper::{Body, HeaderMap, Method};
use std::time::Duration;

use crate::config::Endpoint;
use crate::error::Error as RestError;
use crate::https::HttpsClient;
use crate::path::ProxyPath;
use crate::urls::Urls;

pub struct ProxyRequest {
    pub client: HttpsClient,
    pub endpoint: Endpoint,
    pub method: Method,
    pub path: ProxyPath,
    pub body: Body,
    pub request_headers: HeaderMap,
    pub query: Option<String>,
}

// Default endpoint connection timeout of 5 seconds
const TIMEOUT_DEFAULT: u64 = 5000;

impl ProxyRequest {
    pub async fn single(
        self,
        url: String,
        queries: Option<String>,
    ) -> Result<Response<Body>, RestError> {
        let host_and_path = format!(
            "{}{}{}",
            url,
            self.path.path(),
            queries.unwrap_or_else(||"".to_string())
        );
        let uri = match Uri::try_from(host_and_path) {
            Ok(u) => u,
            Err(e) => {
                log::error!("{{\"error\": \"{}\"}}", e);
                return Err(RestError::UnparseableUrl);
            }
        };

        log::debug!("full uri: {}", uri);

        let mut req = Request::builder()
            .method(self.method)
            .uri(&uri)
            .body(self.body)
            .expect("request builder");

        // Append to request the headers passed by client
        let headers = req.headers_mut();
        headers.extend(self.request_headers.clone());

        // Added Basic Auth if username/password exist
        if let Some(authentication) = self.endpoint.authentication {
            authentication.headers(headers, &uri).await?;
        }

        let work = self.client.clone().request(req);
        let timeout = match self.endpoint.timeout {
            Some(duration) => duration,
            None => TIMEOUT_DEFAULT,
        };

        match tokio::time::timeout(Duration::from_millis(timeout), work).await {
            Ok(result) => match result {
                Ok(response) => Ok(response),
                Err(e) => {
                    log::error!("{{\"error\":\"{}\"", e);
                    Err(RestError::Connection)
                }
            },
            Err(_) => Err(RestError::ConnectionTimeout),
        }
    }

    pub async fn go(self) -> Result<Response<Body>, RestError> {
        // Prepare queries for appending
        let queries = self.query.as_ref().map(|q| format!("?{}", q));

        match self.endpoint.url.clone() {
            Urls::Url(_) => {
                log::debug!("Got a single url");
                let url = self.endpoint.url().await;
                self.single(url, queries).await
            }
            Urls::UrlFailover(urlfailover) => {
                log::debug!("Got a failover url");
                let url = urlfailover.url().to_string();
                match self.single(url, queries).await {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        log::error!("Error connecting to member, failing over member");
                        let _ = urlfailover.next();
                        Err(e)
                    }
                }
            }
        }
    }
}