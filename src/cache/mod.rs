use moka::future::Cache as MokaCache;
pub use plex_user::PlexUser;
use std::time::Duration;

pub mod plex_user;

#[derive(Clone)]
pub struct Cache {
    pub auth: MokaCache<String, PlexUser>,
}

impl Cache {
    pub fn new() -> Self {
        let auth = MokaCache::builder()
            .time_to_idle(Duration::from_mins(5))
            .time_to_live(Duration::from_hours(1))
            .build();

        Self { auth }
    }
}
