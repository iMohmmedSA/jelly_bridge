use std::time::Duration;

use axum::{extract::FromRequestParts, http::request::Parts};
use reqwest::StatusCode;
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::{cache::PlexUser, state::State};

#[derive(Deserialize)]
struct UserResponse {
    id: i64,
}

impl FromRequestParts<State> for PlexUser {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        let token = extract_token(parts);

        if token.is_empty() {
            warn!("Rejected request: No X-Plex-Token found in headers or URL");
            return Err(StatusCode::UNAUTHORIZED);
        }

        if let Some(user) = state.cache.auth.get(&token).await {
            debug!("Cache Hit: Instantly verified Plex User {}", user.plex_id);
            return Ok(user);
        }

        verify(token, state).await
    }
}

fn extract_token(parts: &mut Parts) -> String {
    if let Some(header) = parts.headers.get("x-plex-token") {
        return header.to_str().unwrap_or("").to_string();
    }

    if let Some(query) = parts.uri.query() {
        for pair in query.split('&') {
            let mut key_val = pair.splitn(2, '=');
            let key = key_val.next().unwrap_or("");

            if key.eq_ignore_ascii_case("x-plex-token") {
                return key_val.next().unwrap_or("").to_string();
            }
        }
    }

    String::new()
}

async fn verify(token: String, state: &State) -> Result<PlexUser, StatusCode> {
    if state.plex_limiter.check().is_err() {
        warn!("Rate limited");
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    info!("Cache Miss: Reaching out to Plex.tv to verify unknown token...");

    let res = state
        .http
        .get("https://plex.tv/api/v2/user")
        .timeout(Duration::from_secs(15))
        .header("X-Plex-Token", &token)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !res.status().is_success() {
        warn!("Plex.tv rejected the token! Unknown or expired session.");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let plex_data: UserResponse = res
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    info!(
        "Plex.tv confirmed this token belongs to User ID {}. Checking local database...",
        plex_data.id
    );

    let jellyfin_key = state
        .db
        .get_jellyfin_key(plex_data.id as i64)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(key) = jellyfin_key else {
        warn!(
            "User ID {} is a valid Plex account, but they NEVER registered! Rejecting.",
            plex_data.id
        );
        return Err(StatusCode::UNAUTHORIZED);
    };

    info!("Success! User ID {} is registered.", plex_data.id);

    let valid_user = PlexUser {
        plex_id: plex_data.id,
        jellyfin_api_key: key,
    };

    state.cache.auth.insert(token, valid_user.clone()).await;

    Ok(valid_user)
}
