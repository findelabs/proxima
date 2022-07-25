use serde::{Deserialize, Serialize};
//use serde_json::{json, Map, Value};

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfig {
    pub network: GlobalConfigNetwork,
    pub security: GlobalConfigSecurity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrations: Option<GlobalConfigIntegrations>
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfigNetwork {
    pub timeout: Timeout,
    pub nodelay: bool,
    pub reuse_address: bool,
    pub enforce_http: bool
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfigSecurity {
    pub tls: GlobalConfigSecurityTls,
    pub config: GlobalConfigSecurityConfig
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfigSecurityTls {
    pub accept_invalid_hostnames: bool,
    pub insecure: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_cert: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfigSecurityConfig {
    pub hide_folders: bool
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfigIntegrations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<GlobalConfigIntegrationsVault>
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfigIntegrationsVault {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kubernetes_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    login_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    jwt_path: Option<String>
}

impl Timeout {
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Timeout in seconds.
#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct Timeout(u64);
impl Default for Timeout {
    fn default() -> Self {
        Timeout(5000)
    }
}
