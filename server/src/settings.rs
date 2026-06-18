use anyhow::Result;
use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}


#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Settings {
    pub server: ServerSettings
}

impl Settings {
    pub fn new(environment: &str) -> Result<Self> {
        let conf = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{environment}")))
            .add_source(Environment::with_prefix("APP").separator("_"))
            .build()?;
        Ok(conf.try_deserialize()?)
    }
}
