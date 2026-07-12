use reqwest::Client;

use crate::{db::Database, error::Result};

#[derive(Clone)]
pub struct State {
    pub http: Client,
    pub db: Database,
}

impl State {
    pub async fn init() -> Result<Self> {
        let db = Database::init().await?;
        let http = Client::new();

        Ok(Self { http, db })
    }
}
