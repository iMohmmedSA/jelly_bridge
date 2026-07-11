use crate::{config::Config, error::Result, state::State};

mod config;
mod db;
mod error;
mod http;
mod logger;
mod state;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;

    logger::init(&config.log_filter);

    let state = State::init().await?;

    http::serve(state).await?;

    Ok(())
}
