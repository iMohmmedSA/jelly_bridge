use axum::extract::State as AxumState;
use tracing::info;

use crate::{cache::PlexUser, state::State};

pub async fn handler(user: PlexUser, state: AxumState<State>) -> String {
    let f = format!("Welcome to the library, user {}!", user.plex_id);

    info!("{}", f);

    f
}
