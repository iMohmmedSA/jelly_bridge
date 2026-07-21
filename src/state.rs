use std::{num::NonZero, sync::Arc};

use governor::{
    Quota, RateLimiter,
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
};
use reqwest::Client;

use crate::{cache::Cache, db::Database, error::Result};

type GlobalLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

#[derive(Clone)]
pub struct State {
    pub http: Client,
    pub db: Database,
    pub cache: Cache,
    pub plex_limiter: GlobalLimiter,
}

impl State {
    pub async fn init() -> Result<Self> {
        let db = Database::init().await?;
        let http = Client::new();
        let cache = Cache::new();
        let plex_limiter = Arc::new(RateLimiter::direct(Quota::per_minute(
            NonZero::new(10).unwrap(),
        )));

        Ok(Self {
            http,
            db,
            cache,
            plex_limiter,
        })
    }
}
