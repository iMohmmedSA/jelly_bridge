use axum::http::{HeaderMap, HeaderValue};
use serde::Deserialize;
use tracing::{info, warn};

use crate::{config::Config, error::Result, state::State};

#[derive(Deserialize)]
struct PlexUser {
    #[serde(rename = "@authToken")]
    auth_token: String,
}

fn build_headers(machine_id: &str, server_name: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-Plex-Client-Identifier",
        HeaderValue::from_str(machine_id).expect("Invalid Machine ID"),
    );
    headers.insert("X-Plex-Provides", HeaderValue::from_static("server"));
    headers.insert("X-Plex-Version", HeaderValue::from_static("1.43.2.10687"));
    headers.insert(
        "X-Plex-Product",
        HeaderValue::from_static("Plex Media Server"),
    );
    headers.insert("X-Plex-Platform", HeaderValue::from_static("Linux"));
    headers.insert("X-Plex-Device", HeaderValue::from_static("PC"));
    headers.insert(
        "X-Plex-Device-Name",
        HeaderValue::from_str(server_name).expect("Invalid Server Name"),
    );
    headers
}

pub async fn claim(state: &State, config: &Config) -> Result<()> {
    let id = state.db.get_or_create_machine_id().await?;

    if state.db.is_claimed().await? {
        tracing::info!("Server identity verified from database.");
        publish_server(state, config, &id).await?;
        return Ok(());
    }

    let claim_token = config
        .plex_claim
        .as_ref()
        .expect("CRITICAL: Server is unclaimed. You must provide PLEX_CLAIM in your .env file.");

    info!("Unclaimed server. Attempting to claim with plex.tv...");

    let headers = build_headers(&id, &config.server_name);
    let res = state
        .http
        .post("https://plex.tv/api/claim/exchange")
        .query(&[("token", claim_token)])
        .headers(headers.clone())
        .send()
        .await?;

    if !res.status().is_success() {
        panic!("CRITICAL: PLEX_CLAIM token is invalid or expired. Get a new one at plex.tv/claim.");
    }

    let xml = res.text().await?;
    let data: PlexUser = quick_xml::de::from_str(&xml)?;

    state.db.save_server_token(&data.auth_token).await?;
    tracing::info!("Server successfully claimed!");

    publish_server(state, config, &id).await?;
    Ok(())
}

async fn publish_server(state: &State, config: &Config, machine_id: &str) -> Result<()> {
    let token = state
        .db
        .get_server_token()
        .await?
        .expect("Token must exists");

    info!("Publishing server metadata to servers.xml...");
    let mut xml_headers = build_headers(machine_id, &config.server_name);
    xml_headers.insert("Content-Type", HeaderValue::from_static("application/xml"));

    let escaped_server_name = quick_xml::escape::escape(&config.server_name);
    let xml_payload = format!(
        r#"<MediaContainer size="1"><Server name="{}" host="" localAddresses="{}" machineIdentifier="{}" version="1.43.2.10687" /></MediaContainer>"#,
        escaped_server_name, config.advertise_ip, machine_id
    );

    let servers_res = state
        .http
        .post("https://plex.tv/servers.xml")
        .headers(xml_headers)
        .query(&[("auth_token", token.as_str())])
        .body(xml_payload)
        .send()
        .await?;

    let status = servers_res.status().as_u16();
    if matches!(status, 200 | 201 | 204 | 422) {
        info!("Server metadata published successfully.");
    } else {
        warn!("Failed to publish server metadata. Status: {}", status);
    }

    let uri = format!("http://{}:{}", config.advertise_ip, config.port);
    info!("Registering device URI with Plex: {}", uri);

    let headers = build_headers(machine_id, &config.server_name);
    let device_url = format!("https://plex.tv/devices/{}", machine_id);

    let device_res = state
        .http
        .put(&device_url)
        .headers(headers)
        .query(&[
            ("Connection[][uri]", uri.as_str()),
            ("httpsEnabled", "0"),
            ("httpsRequired", "0"),
            ("X-Plex-Token", token.as_str()),
        ])
        .send()
        .await?;

    if device_res.status().is_success() {
        info!("Device URI registered successfully.");
    } else {
        warn!("Failed to register device URI. Clients may not be able to auto-discover it.");
    }

    Ok(())
}
