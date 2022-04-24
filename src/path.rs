use axum::{
    async_trait,
    extract::{FromRequest, RequestParts},
};
use serde::Serialize;
use std::convert::Infallible;

#[derive(Debug, Clone, Serialize, Default)]
pub struct ProxyPath {
    pub path: String,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}

impl ProxyPath {
    pub fn new(path: &str) -> ProxyPath {

        // Create String so that we can manipulate it
        let mut path = path.to_string();

        // Remove prefix of /
        #[allow(clippy::iter_nth_zero)]
        if let Some('/') = path.chars().nth(0) {
            log::debug!("Removing / prefix from path");
            path.remove(0);
        };

        // Remove suffix of /
        if let Some('/') = path.chars().last() {
            log::debug!("Removing / prefix from path");
            path.remove(0);
        };

        let vec: Vec<&str> = path.splitn(2, '/').collect();

        match vec.len() {
            0 => {
                log::debug!("Weird, we have an empty vec...");
                ProxyPath {
                    path: path.clone(),
                    prefix: None,
                    suffix: None,
                }
            }
            1 => {
                log::debug!("Found one item, setting prefix to {}", &vec[0]);
                ProxyPath {
                    path: path.clone(),
                    prefix: Some(vec[0].to_owned()),
                    suffix: None,
                }
            }
            _ => {
                log::debug!(
                    "Found two items, setting prefix to {}, suffix to {}, and path to {}",
                    vec[0],
                    vec[1],
                    &path
                );
                ProxyPath {
                    path: path.clone(),
                    prefix: Some(vec[0].to_owned()),
                    suffix: Some(vec[1].to_owned()),
                }
            }
        }
    }

    pub fn next(&self) -> Option<ProxyPath> {
        self.suffix.as_ref().map(|s| ProxyPath::new(s))
    }

    pub fn path(&self) -> Option<&str> {
        match self.path.as_str() {
            "" => None,
            _ => Some(&self.path),
        }
    }

    pub fn prefix(&self) -> Option<String> {
        self.prefix.clone()
    }

    pub fn suffix(&self) -> Option<String> {
        self.suffix.clone()
    }
}

#[async_trait]
impl<B> FromRequest<B> for ProxyPath
where
    B: Send,
{
    type Rejection = Infallible;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let uri = req.uri().to_owned();
        Ok(ProxyPath::new(uri.path()))
    }
}
