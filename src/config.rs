use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;

pub type ConfigHash = HashMap<String, ConfigEntry>;

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct ConfigEntry {
    pub url: Url,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub username: String,

    #[serde(default)]
    #[serde(skip_serializing)]
    pub password: String,
}

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct Url(String);

impl Default for Url {
    fn default() -> Self {
        Url("string".to_string())
    }
}

// Add ability to use to_string() with Url
impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

//pub fn new(file: &str) -> Result<ConfigHash, serde_yaml::Error> {
//    let config = parse(file)?;
//    Ok(Arc::new(RwLock::new(config)))
//}

pub fn parse(file: &str) -> Result<HashMap<String, ConfigEntry>, serde_yaml::Error> {
    let mut file = File::open(file).expect("Unable to open config");
    let mut contents = String::new();

    file.read_to_string(&mut contents)
        .expect("Unable to read config");

    let deck: HashMap<String, ConfigEntry> = serde_yaml::from_str(&contents)?;
    Ok(deck)
}
