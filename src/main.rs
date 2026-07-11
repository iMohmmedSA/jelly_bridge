use crate::{config::Config, error::Result, state::State};

mod config;
mod db;
mod error;
mod logger;
mod state;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;

    logger::init(&config.log_filter);

    State::init().await?;

    Ok(())
}
