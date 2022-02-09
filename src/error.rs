//use serde_json::error::Error as SerdeError;
use std::fmt;
use axum::{
    body::{self},
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum Error {
    Forbidden,
    Unauthorized,
    NotFound,
    UnkError
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Forbidden=> f.write_str("Forbidden"),
            Error::Unauthorized=> f.write_str("Unauthorized"),
            Error::NotFound=> f.write_str("Not found"),
            Error::UnkError=> f.write_str("Returned bad http status code")
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
