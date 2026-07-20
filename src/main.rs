use crate::{config::Config, error::Result, plex::claim, state::State};

mod cache;
mod config;
mod db;
mod error;
mod http;
mod logger;
mod plex;
mod state;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;

    logger::init(&config.log_filter);

    let state = State::init().await?;

    claim(&state, &config).await?;

    http::serve(state, config.enable_ssl).await?;

    Ok(())
}
