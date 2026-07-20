mod providers;

use axum::{Router, routing::get};

use crate::state::State;

pub fn router() -> Router<State> {
    Router::new().route("/providers", get(providers::handler))
}
