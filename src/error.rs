//use serde_json::error::Error as SerdeError;
use axum::{
    body::{self},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rand::{distributions::Alphanumeric, Rng};
use hyper::header::HeaderValue;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Forbidden,
    Unauthorized,
    NotFound,
    Unknown,
    BadToken,
    UnknownEndpoint,
    BadUserPasswd,
    Connection,
    UnparseableUrl,
    UnauthorizedUser,
    UnauthorizedDigestUser,
    ConnectionTimeout,
    Hyper(hyper::Error),
    SerdeJson(serde_json::Error),
    SerdeYaml(serde_yaml::Error),
    File(std::io::Error),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Forbidden => f.write_str("{\"error\": \"Cannot get config: Forbidden\"}"),
            Error::Unauthorized => f.write_str("{\"error\": \"Cannot get config: Unauthorized\"}"),
            Error::NotFound => f.write_str("{\"error\": \"Cannot get config: Not found\"}"),
            Error::Unknown => {
                f.write_str("{\"error\": \"Cannot get config: Returned bad status code\"}")
            }
            Error::BadToken => f.write_str("{\"error\": \"Unparsable token provided\"}"),
            Error::BadUserPasswd => {
                f.write_str("{\"error\": \"Unparsable username and password provided\"}")
            }
            Error::UnknownEndpoint => f.write_str("{\"error\": \"unknown endpoint\"}"),
            Error::Connection => f.write_str("{\"error\": \"Error connecting to rest endpoint\"}"),
            Error::UnparseableUrl => f.write_str("{\"error\": \"Error parsing uri\"}"),
            Error::UnauthorizedUser => f.write_str("{\"error\": \"Unauthorized\"}"),
            Error::UnauthorizedDigestUser => f.write_str("{\"error\": \"Unauthorized\"}"),
            Error::ConnectionTimeout=> f.write_str("{\"error\": \"Connection timeout\"}"),
            Error::Hyper(ref err) => write!(f, "{{\"error\": \"{}\"}}", err),
            Error::SerdeJson(ref err) => write!(f, "{{\"error\": \"{}\"}}", err),
            Error::SerdeYaml(ref err) => write!(f, "{{\"error\": \"{}\"}}", err),
            Error::File(ref err) => write!(f, "{{\"error\": \"{}\"}}", err),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let payload = self.to_string();
        let body = body::boxed(body::Full::from(payload));
        let mut res = Response::builder();
        let headers = res.headers_mut().expect("Failed to get headers from response");

        let status_code = match self {
            Error::UnknownEndpoint => StatusCode::NOT_FOUND,
            Error::Forbidden => StatusCode::FORBIDDEN,
            Error::Unauthorized | Error::UnauthorizedUser => StatusCode::UNAUTHORIZED,
            Error::UnauthorizedDigestUser => {
                let nonce: String = rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(32)
                    .map(char::from)
                    .collect();

                let header_value = format!("Digest realm=\"Proxima API\", domain=\"\", nonce=\"{}\", algorithm=MD5, qop=\"auth\", stale=false", nonce);
                let header = HeaderValue::from_str(&header_value).unwrap();
                headers.insert("www-authenticate", header);
                StatusCode::UNAUTHORIZED
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let response = res.status(status_code)
            .body(body)
            .unwrap();

        response
    }
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Error {
        Error::Hyper(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::SerdeJson(err)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(err: serde_yaml::Error) -> Error {
        Error::SerdeYaml(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::File(err)
    }
}
