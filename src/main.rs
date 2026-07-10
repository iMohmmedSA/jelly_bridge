use crate::{config::Config, error::Result};

mod config;
mod error;
mod logger;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;

    logger::init(&config.log_filter);

    Ok(())
}
