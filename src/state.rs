use crate::{db::Database, error::Result};

#[derive(Clone)]
pub struct State {
    db: Database,
}

impl State {
    pub async fn init() -> Result<Self> {
        let db = Database::init().await?;

        Ok(Self { db })
    }
}
