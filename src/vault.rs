use clap::ArgMatches;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::offset::Utc;
use vault_client_rs::client::{Client, ClientBuilder};
use serde_json::{Map, Value};
use serde_json::json;
use handlebars::Handlebars;
use crate::error::Error as ProximaError;
use serde::{Deserialize, Serialize};
use base64;

use crate::https::HttpsClient;

type BoxResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultConfig {
    pub template: String,
    pub secret: String,
    #[serde(skip_serializing)]
    pub last: Arc<RwLock<i64>>,
    #[serde(skip_serializing)]
    pub template_parsed: Option<Handlebars>,
    pub insecure: bool,
    pub recursive: bool
}

impl VaultConfig {
    pub async fn template(&self, secret: Map<String, Value>) -> Result<Value, ProximaError> {
        let output = t.render("secret", &secret)?;
        log::debug!("Rendered string: {}", &output);
        Ok(serde_json::from_str(&output)?)
    }

    pub async fn new(opts: ArgMatches<'_>) -> BoxResult<State> {
        // Set timeout
        let timeout: u64 = opts
            .value_of("timeout")
            .unwrap()
            .parse()
            .unwrap_or_else(|_| {
                eprintln!("Supplied timeout not in range, defaulting to 60");
                60
            });

        let client = HttpsClient::default();
        let vault_secret = opts.value_of("vault_secret").unwrap().to_owned();
        let recursive = opts.is_present("recursive");
        let last = Utc::now().timestamp();

        let template = match opts.is_present("template") {
            true => {
                let template_base64 = opts.value_of("template").unwrap();
                let bytes = match base64::decode(&template_base64) {
                    Ok(b) => b,
                    Err(e) => panic!("Error decoding template from base64: {}", e)
                };

                let template_decoded = match std::str::from_utf8(&bytes) {
                    Ok(t) => t,
                    Err(e) => panic!("Error converting template to string: {}", e)
                };

                let mut handlebars = Handlebars::new();
                handlebars.register_template_string("secret", template_decoded)?;
                handlebars.set_strict_mode(true);
                Some(handlebars)
            },
            false => None
        };

        let mut vault = ClientBuilder::new()
            .with_mount(opts.value_of("vault_mount").unwrap())
            .with_url(opts.value_of("vault_url").unwrap())
            .with_login_path(opts.value_of("vault_login_path").unwrap())
            .with_kubernetes_role(opts.value_of("vault_kubernetes_role"))
            .with_role_id(opts.value_of("vault_role_id"))
            .with_secret_id(opts.value_of("vault_secret_id"))
            .with_jwt_path(opts.value_of("jwt_path"))
            .insecure(opts.is_present("insecure"))
            .build().unwrap();

        vault.login().await.unwrap();

        Ok(State {
            client,
            vault_secret,
            template,
            last: Arc::new(RwLock::new(last)),
            vault,
            recursive
        })
    }
}
