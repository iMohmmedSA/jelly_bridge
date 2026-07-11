use axum::{Router, extract::Request, http::StatusCode};
use tracing::warn;

use crate::state::State;

pub fn router(state: State) -> Router {
    Router::new().fallback(fallback_handler).with_state(state)
}

async fn fallback_handler(req: Request) -> StatusCode {
    warn!("Unmatched Route: {} {}", req.method(), req.uri());

    StatusCode::NOT_FOUND
}
