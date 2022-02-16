//use serde_json::error::Error as SerdeError;
use axum::{
    body::{self},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Forbidden,
    Unauthorized,
    NotFound,
    UnkError,
    BadToken,
    UnknownEndpoint,
    BadUserPasswd,
    ConnectionError,
    UnparseableUrl,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Forbidden => f.write_str("{\"error\": \"Cannot get config: Forbidden\"}"),
            Error::Unauthorized => f.write_str("{\"error\": \"Cannot get config: Unauthorized\"}"),
            Error::NotFound => f.write_str("{\"error\": \"Cannot get config: Not found\"}"),
            Error::UnkError => {
                f.write_str("{\"error\": \"Cannot get config: Returned bad status code\"}")
            }
            Error::BadToken => f.write_str("{\"error\": \"Unparsable token provided\"}"),
            Error::BadUserPasswd => {
                f.write_str("{\"error\": \"Unparsable username and password provided\"}")
            }
            Error::UnknownEndpoint => f.write_str("{\"error\": \"please specify known endpoint\"}"),
            Error::ConnectionError => {
                f.write_str("{\"error\": \"Error connecting to rest endpoint\"}")
            }
            Error::UnparseableUrl => f.write_str("{\"error\": \"Error parsing uri\"}"),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let payload = self.to_string();
        let body = body::boxed(body::Full::from(payload));

        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(body)
            .unwrap()
    }
}
