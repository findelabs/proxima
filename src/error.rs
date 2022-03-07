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
    Unknown,
    BadToken,
    UnknownEndpoint,
    BadUserPasswd,
    Connection,
    UnparseableUrl,
    UnauthorizedUser,
    Hyper(hyper::Error),
    SerdeJson(serde_json::Error),
    SerdeYaml(serde_yaml::Error),
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
            Error::Hyper(ref err) => write!(f, "{{\"error\": \"{}\"}}", err),
            Error::SerdeJson(ref err) => write!(f, "{{\"error\": \"{}\"}}", err),
            Error::SerdeYaml(ref err) => write!(f, "{{\"error\": \"{}\"}}", err),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let payload = self.to_string();
        let body = body::boxed(body::Full::from(payload));

        let status_code = match self {
            Error::UnknownEndpoint => StatusCode::NOT_FOUND,
            Error::Forbidden => StatusCode::FORBIDDEN,
            Error::Unauthorized | Error::UnauthorizedUser => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Response::builder().status(status_code).body(body).unwrap()
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
