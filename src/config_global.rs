use crate::security::Security;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfig {
    #[serde(default)]
    pub network: GlobalConfigNetwork,
    #[serde(default)]
    pub security: GlobalConfigSecurity,
    //    #[serde(skip_serializing_if = "Option::is_none")]
    //    pub integrations: Option<GlobalConfigIntegrations>
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfigNetwork {
    #[serde(default)]
    pub timeout: Timeout,
    #[serde(default)]
    pub nodelay: bool,
    #[serde(default)]
    pub reuse_address: bool,
    #[serde(default)]
    pub enforce_http: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfigSecurity {
    #[serde(default)]
    pub tls: GlobalConfigSecurityTls,
    #[serde(default)]
    pub config: GlobalConfigSecurityConfig,
    #[serde(default)]
    pub auth: Option<Security>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfigSecurityTls {
    #[serde(default)]
    pub accept_invalid_hostnames: bool,
    #[serde(default)]
    pub insecure: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_cert: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfigSecurityConfig {
    #[serde(default)]
    pub hide_folders: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfigIntegrations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<GlobalConfigIntegrationsVault>,
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
    jwt_path: Option<String>,
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
