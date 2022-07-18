use crate::security::Whitelist;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::auth::traits::{AuthList, Authorize, AuthorizeList};

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct BearerAuth {
    #[serde(skip_serializing)]
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Whitelist>,
}

// Pull in trait
impl AuthorizeList for AuthList<BearerAuth> {}

#[async_trait]
impl Authorize for BearerAuth {

    const AUTHORIZATION_TYPE: Option<&'static str> = Some("bearer");

    fn correct_header(&self) -> String {
        self.token()
    }

    fn header_name(&self) -> &str {
        "AUTHORIZATION"
    }

    fn whitelist(&self) -> Option<&Whitelist> {
        self.whitelist.as_ref()
    }
}


impl BearerAuth {
    pub fn token(&self) -> String {
        self.token.clone()
    }
}
