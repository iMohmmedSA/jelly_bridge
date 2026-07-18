use axum::{Router, extract::Request, http::StatusCode};
use tracing::{debug, warn};

use crate::state::State;

pub fn router(state: State) -> Router {
    Router::new().fallback(fallback_handler).with_state(state)
}

async fn fallback_handler(req: Request) -> StatusCode {
    warn!("Unmatched Route: {} {}", req.method(), req.uri().path());

    if let Some(query) = req.uri().query() {
        debug!("  Query Parameters:");
        for pair in query.split('&') {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next().unwrap_or("");
            let value = parts.next().unwrap_or("");
            debug!("    {}: {}", key, value);
        }
    }

    debug!("  Headers:");
    for (name, value) in req.headers() {
        debug!(
            "    {}: {}",
            name,
            String::from_utf8_lossy(value.as_bytes())
        );
    }

    StatusCode::NOT_FOUND
}
