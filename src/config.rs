use config::Environment;
use serde::Deserialize;

use crate::error::Result;

#[derive(Debug, Deserialize)]
pub struct Config {
    jellyfin_url: String,

    pub server_name: String,
    pub plex_claim: Option<String>,
    pub advertise_ip: String,
    pub port: u16,

    pub log_filter: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(config::Config::builder()
            .set_default("server_name", "Jelly Bridge")?
            .set_default("port", 9096)?
            .set_default("log_filter", "info")?
            .add_source(Environment::default())
            .build()?
            .try_deserialize()?)
    }
}
