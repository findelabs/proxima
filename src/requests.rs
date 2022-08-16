use axum::{
    http::uri::Uri,
    http::{Request, Response},
};
use hyper::header::HeaderValue;
use hyper::{Body, HeaderMap, Method};
use std::time::Duration;
use url::Url;
use axum::body::HttpBody;

use crate::config::Proxy;
use crate::error::Error as ProximaError;
use crate::https::HttpsClient;
use crate::path::ProxyPath;
use crate::urls::Urls;

pub struct ProxyRequest {
    pub client: HttpsClient,
    pub endpoint: Proxy,
    pub method: Method,
    pub path: ProxyPath,
    pub body: Body,
    pub request_headers: HeaderMap,
    pub query: Option<String>,
}

// Default endpoint connection timeout of 60 seconds
const TIMEOUT_DEFAULT: u64 = 60000;

impl ProxyRequest {
    pub async fn single(
        mut self,
        url: &Url,
        queries: Option<String>,
    ) -> Result<Response<Body>, ProximaError> {
        // This needs to be done as urls with paths do not end with a forward slash,
        // but urls with no paths do
        let seperator = match (url.path(), self.path.suffix().as_str()) {
            ("/", "") => "",
            ("/", _) => "",
            (_, "") => "",
            (_,_) => "/",
        };

        let host_and_path = format!(
            "{}{}{}{}",
            url,
            seperator,
            self.path.suffix(),
            queries.unwrap_or_else(|| "".to_string())
        );

        let uri = match Uri::try_from(host_and_path) {
            Ok(u) => u,
            Err(e) => {
                log::error!("{{\"error\": \"{}\"}}", e);
                return Err(ProximaError::UnparseableUrl);
            }
        };

        log::debug!("full uri: {}", uri);

        let mut req = Request::builder()
            .method(&self.method)
            .uri(&uri)
            .body(self.body)
            .expect("request builder");

        let response_transmit = req.body().size_hint().upper().unwrap_or(0) as f64;

        // Apply changes to headers based on config
        if let Some(config) = self.endpoint.config {
            if !config.preserve_host_header {
                log::debug!("\"Removing client HOST/USER_AGENT headers\"");
                // Remove HOST and USER_AGENT headers
                self.request_headers.remove(hyper::header::HOST);
                self.request_headers.remove(hyper::header::USER_AGENT);
            };
        } else {
            // Apply all default changes to headers
            // Remove HOST and USER_AGENT headers
            log::debug!("\"Removing client HOST/USER_AGENT headers\"");
            self.request_headers.remove(hyper::header::HOST);
            self.request_headers.remove(hyper::header::USER_AGENT);
        }

        // Add x-forwarded-prefix
        let header = HeaderValue::from_str(self.path.path()).unwrap();
        self.request_headers.insert("x-forwarded-prefix", header);

        // Append to request the headers passed by client
        let headers = req.headers_mut();
        headers.extend(self.request_headers.clone());

        // Added Basic Auth if username/password exist
        if let Some(authentication) = self.endpoint.authentication {
            authentication.headers(headers, &uri).await?;
        }

        let timeout = match self.endpoint.timeout {
            Some(duration) => duration,
            None => TIMEOUT_DEFAULT,
        };

        match tokio::time::timeout(
            Duration::from_millis(timeout),
            self.client.request(req),
        )
        .await
        {
            Ok(result) => match result {
                Ok(response) => {
                    let labels = [
                        ("method", self.method.to_string()),
                        ("path", self.path.suffix()),
                        ("status", response.status().as_u16().to_string())
                    ];
                    let response_receive = response.body().size_hint().upper().unwrap_or(0) as f64;
                    metrics::increment_gauge!("proxima_response_receive_bytes", response_receive, &labels);
                    metrics::increment_gauge!("proxima_response_transmit_bytes", response_transmit, &labels);
                    Ok(response)
                },
                Err(e) => {
                    log::error!("{{\"error\":\"{}\"", e);
                    Err(ProximaError::Connection)
                }
            },
            Err(_) => Err(ProximaError::ConnectionTimeout),
        }
    }

    pub async fn go(self) -> Result<Response<Body>, ProximaError> {
        // Prepare queries for appending
        let queries = self.query.as_ref().map(|q| format!("?{}", q));

        // Tried improving this, may take some more work
        match self.endpoint.url.clone() {
            Urls::Url(u) => {
                log::debug!("Got a single url");
                self.single(&u, queries).await
            }
            Urls::UrlFailover(urlfailover) => {
                log::debug!("Got a failover url");
                let url = urlfailover.current();
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
