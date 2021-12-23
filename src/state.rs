use axum::{
    http::{StatusCode, uri::Uri},
    http::{Request, Response},
    Json,
};
use hyper::{
    header::{HeaderValue, CONTENT_TYPE},
    client::HttpConnector, 
    Body, 
    Method
};
use hyper_tls::HttpsConnector;
use serde_json::{Value};
use std::{convert::TryFrom};
//use std::error::Error;
use crate::config::{ConfigHash, ConfigEntry};

type HttpsClient = hyper::client::Client<HttpsConnector<HttpConnector>, Body>;
//type BoxResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub struct State {
    pub config: ConfigHash,
    pub client: HttpsClient
}

impl State {
    async fn get_entry(&self, item: &str) -> Option<&ConfigEntry> {
        log::debug!("Getting {} from ConfigHash", &item);
        self.config.get(item)
    }

    pub async fn response(&self,
        method: Method,
        path_and_query: &str,
        payload: Option<Json<Value>>
    ) -> Response<Body> {

        // Convert path to string, so we can remove the first char
        let mut path_and_query = path_and_query.to_string();
        path_and_query.remove(0);

        let (first,rest) = match path_and_query.split_once("/") {
            Some((f,r)) => (f,r),
            None => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("{\"error\": \"please specify endpoint\"}"))
                    .unwrap()
            }
        };

        let config_entry = match self.get_entry(first).await {
            Some(e) => e,
            None => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from(serde_json::to_string(&*self.config).unwrap()))
                    .unwrap()
            }
        };

        let path_and_query = rest.replace(" ", "%20");
        let host_and_path = format!("{}/{}", config_entry.url, path_and_query);
        log::debug!("full uri: {}", host_and_path);

        match Uri::try_from(host_and_path) {
            Ok(u) => {
                let body = match payload {
                    Some(p) => {
                        log::debug!("Received body: {:?}", &p);
                        Body::from(p.to_string())
                    },
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
    
                req.headers_mut()
                    .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    
                match self.client.request(req).await {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("{}", e);
                        Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from("{\"error\": \"Error connecting to rest endpoint\"}"))
                            .unwrap()
                    }
                }
            },
            Err(_) => {
                Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("{\"error\": \"Error parsing uri\"}"))
                    .unwrap()
            }
        }
    }
}
