use reqwest::Client;

use crate::{cache::Cache, db::Database, error::Result};

#[derive(Clone)]
pub struct State {
    pub http: Client,
    pub db: Database,
    pub cache: Cache,
}

impl State {
    pub async fn init() -> Result<Self> {
        let db = Database::init().await?;
        let http = Client::new();
        let cache = Cache::new();

        Ok(Self { http, db, cache })
    }
}
