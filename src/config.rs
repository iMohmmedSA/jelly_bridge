use config::Environment;
use serde::Deserialize;

use crate::error::Result;

#[derive(Debug, Deserialize)]
pub struct Config {
    jellyfin_url: String,
    plex_claim: Option<String>,
    pub log_filter: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(config::Config::builder()
            .set_default("log_filter", "info")?
            .add_source(Environment::default())
            .build()?
            .try_deserialize()?)
    }
}
