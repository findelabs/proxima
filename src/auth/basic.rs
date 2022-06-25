use serde::{Deserialize, Serialize};
use crate::security::Whitelist;
use hyper::header::HeaderValue;
use crate::error::Error as ProximaError;
use hyper::Method;


#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct BasicAuth {
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct BasicAuthList(Vec<BasicAuth>);

impl BasicAuthList {
    pub async fn authorize(
        &self,
        header: &HeaderValue,
        method: &Method
    ) -> Result<(), ProximaError> {
        log::debug!("Looping over basic users");
        let Self(internal) = self;

        for user in internal.iter() {
            log::debug!("\"Checking if connecting client matches {:?}\"", user);
            match user.authorize(header, method).await {
                Ok(_) => return Ok(()),
                Err(_) => {
                    continue;
                }
            }
        }
        log::debug!("\"Client could not be authenticated\"");
        Err(ProximaError::UnauthorizedClientBasic)
    }
}


impl BasicAuth {
    #[allow(dead_code)]
    pub fn username(&self) -> String {
        self.username.clone()
    }

    #[allow(dead_code)]
    pub fn password(&self) -> String {
        self.password.clone()
    }

    pub fn basic(&self) -> String {
        log::debug!("Generating Basic auth");
        let user_pass = format!("{}:{}", self.username, self.password);
        let encoded = base64::encode(user_pass);
        let basic_auth = format!("Basic {}", encoded);
        log::debug!("Using {}", &basic_auth);
        basic_auth
    }

    pub async fn authorize(
        &self,
        header: &HeaderValue,
        method: &Method
    ) -> Result<(), ProximaError> {
        if let Some(ref whitelist) = self.whitelist {
            log::debug!("Found whitelist");
            whitelist.authorize(method)?
        }
        if HeaderValue::from_str(&self.basic()).unwrap() != header {
            metrics::increment_counter!(
                "proxima_security_client_authentication_failed_count",
                "type" => "basic"
            );
            return Err(ProximaError::UnauthorizedClientBasic);
        }
        Ok(())
    }
}
