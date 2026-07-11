use crate::{config::Config, db::Database, error::Result};

mod config;
mod db;
mod error;
mod logger;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;

    logger::init(&config.log_filter);

    let db = Database::init().await?;

    Ok(())
}
