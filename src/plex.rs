use std::time::Duration;

use axum::http::{HeaderMap, HeaderValue};
use serde::Deserialize;
use tokio::{fs, time::sleep};
use tracing::{info, warn};

use crate::{
    config::Config,
    error::{Error, Result},
    state::State,
};

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
        info!("Server identity verified from database.");
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
    info!("Server successfully claimed!");

    publish_server(state, config, &id).await?;
    Ok(())
}

async fn publish_server(state: &State, config: &Config, machine_id: &str) -> Result<()> {
    let token = state
        .db
        .get_server_token()
        .await?
        .expect("Token must exist");

    publish_to_servers_xml(state, config, machine_id, &token).await?;

    let mut device_uri = format!("http://{}:{}", config.advertise_ip, config.port);
    let mut https_enabled = "0";

    if !config.enable_ssl {
        info!("SSL is disabled via config. Using standard HTTP.");
    } else {
        match setup_ssl(state, config, machine_id, &token).await {
            Ok(secure_uri) => {
                device_uri = secure_uri;
                https_enabled = "1";
            }
            Err(e) => warn!("Failed to setup SSL, falling back to HTTP: {}", e),
        }
    }

    register_device(
        state,
        config,
        machine_id,
        &token,
        &device_uri,
        https_enabled,
    )
    .await?;

    Ok(())
}

async fn publish_to_servers_xml(
    state: &State,
    config: &Config,
    machine_id: &str,
    token: &str,
) -> Result<()> {
    info!("Publishing server metadata to servers.xml...");
    let mut xml_headers = build_headers(machine_id, &config.server_name);
    xml_headers.insert("Content-Type", HeaderValue::from_static("application/xml"));

    let escaped_server_name = quick_xml::escape::escape(&config.server_name);
    let xml_payload = format!(
        r#"<MediaContainer size="1"><Server name="{}" host="" address="{}" localAddresses="{}" machineIdentifier="{}" version="1.43.2.10687" /></MediaContainer>"#,
        escaped_server_name, config.advertise_ip, config.advertise_ip, machine_id
    );

    let servers_res = state
        .http
        .post("https://plex.tv/servers.xml")
        .headers(xml_headers)
        .query(&[("auth_token", token)])
        .body(xml_payload)
        .send()
        .await?;

    let status = servers_res.status().as_u16();
    if matches!(status, 200 | 201 | 204 | 422) {
        info!("Server metadata published successfully.");
    } else {
        warn!("Failed to publish server metadata. Status: {}", status);
    }

    Ok(())
}

#[derive(Deserialize, Debug)]
struct CertificateSubject {
    #[serde(rename = "@commonName")]
    common_name: String,
}

async fn setup_ssl(
    state: &State,
    config: &Config,
    machine_id: &str,
    token: &str,
) -> Result<String> {
    info!("Requesting SSL Certificate from Plex...");
    let mut headers_token = build_headers(machine_id, &config.server_name);
    headers_token.insert("X-Plex-Token", HeaderValue::from_str(token).unwrap());

    let subject_url = format!(
        "https://plex.tv/api/v2/devices/{}/certificate/subject",
        machine_id
    );
    let subject_res = state
        .http
        .get(&subject_url)
        .headers(headers_token.clone())
        .send()
        .await?;

    if !subject_res.status().is_success() {
        return Err(Error::Generic(format!(
            "Failed to get certificate subject. Status: {}",
            subject_res.status()
        )));
    }

    let subject_xml = subject_res.text().await?;
    let subject: CertificateSubject = quick_xml::de::from_str(&subject_xml)?;
    let common_name = subject.common_name;
    info!("Received commonName: {}", common_name);

    let device_uri = build_secure_uri(&config.advertise_ip, &common_name, config.port);

    if is_cert_valid().await {
        info!("Valid SSL certificates already exist on disk. Skipping generation.");
        return Ok(device_uri);
    }

    use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
    let mut params = CertificateParams::new(Vec::<String>::new())?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, common_name.clone());
    params.distinguished_name = dn;
    params.key_usages = vec![];
    params.extended_key_usages = vec![];

    use rsa::{
        RsaPrivateKey,
        pkcs8::{EncodePrivateKey, LineEnding},
    };
    let mut rng = rsa::rand_core::OsRng;

    let private_key = RsaPrivateKey::new(&mut rng, 2048)?;
    let key_pem = private_key.to_pkcs8_pem(LineEnding::LF)?;
    let key_pair = KeyPair::from_pem(key_pem.as_str())?;
    let csr = params.serialize_request(&key_pair)?;
    let csr_pem = csr.pem()?;

    let csr_url = format!(
        "https://plex.tv/api/v2/devices/{}/certificate/csr?reason=new&invalidIn=0",
        machine_id
    );

    use reqwest::multipart::{Form, Part};
    let part = Part::text(csr_pem).file_name("csr.pem");
    let form = Form::new().part("file", part);

    let csr_res = state
        .http
        .put(&csr_url)
        .headers(headers_token.clone())
        .multipart(form)
        .send()
        .await?;
    let status = csr_res.status();

    if status != 204 {
        let body = csr_res.text().await.unwrap_or_default();
        return Err(Error::Generic(format!(
            "Failed to upload CSR. Status: {}. Body: {}",
            status, body
        )));
    }

    info!("CSR accepted. Polling for certificate download...");
    let download_url = format!(
        "https://plex.tv/api/v2/devices/{}/certificate/download",
        machine_id
    );

    let mut cert_pem = None;
    let mut wait_time = 2;
    for _ in 0..6 {
        let dl_res = state
            .http
            .get(&download_url)
            .headers(headers_token.clone())
            .send()
            .await?;

        if dl_res.status() == 200 {
            cert_pem = Some(dl_res.text().await?);
            break;
        }

        sleep(Duration::from_secs(wait_time)).await;
        wait_time *= 2; // Exponential backoff: 2s, 4s, 8s, 16s, 32s, 64s
    }

    let cert = cert_pem
        .ok_or_else(|| Error::Generic("Timed out waiting for certificate download".to_string()))?;

    info!("Certificate downloaded successfully.");
    fs::write("plex_cert.pem", cert).await?;
    fs::write("plex_key.pem", key_pem.as_str()).await?;
    info!("Saved cert to plex_cert.pem and key to plex_key.pem");

    Ok(device_uri)
}

async fn register_device(
    state: &State,
    config: &Config,
    machine_id: &str,
    token: &str,
    device_uri: &str,
    https_enabled: &str,
) -> Result<()> {
    info!("Registering device URI with Plex: {}", device_uri);

    let headers = build_headers(machine_id, &config.server_name);
    let device_url = format!("https://plex.tv/devices/{}", machine_id);

    let device_res = state
        .http
        .put(&device_url)
        .headers(headers)
        .query(&[
            ("Connection[][uri]", device_uri),
            ("httpsEnabled", https_enabled),
            ("httpsRequired", "0"),
            ("X-Plex-Token", token),
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

async fn is_cert_valid() -> bool {
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    if !Path::new("plex_cert.pem").exists() || !Path::new("plex_key.pem").exists() {
        return false;
    }

    async {
        let cert_data = fs::read("plex_cert.pem").await.ok()?;
        let (_, parsed_pem) = x509_parser::pem::parse_x509_pem(&cert_data).ok()?;
        let (_, cert) = x509_parser::parse_x509_certificate(&parsed_pem.contents).ok()?;

        let not_after = cert.validity().not_after;
        let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
        let expires_ts = not_after.timestamp();

        // Must have at least 30 days (2,592,000 seconds) left to be considered valid
        Some(expires_ts - now_ts >= 2592000)
    }
    .await
    .unwrap_or(false)
}

fn build_secure_uri(ip: &str, common_name: &str, port: u16) -> String {
    let ip_dashed = ip.replace('.', "-").replace(':', "-");
    let domain_hash = common_name.replace("*.", "");
    format!("https://{}.{}:{}", ip_dashed, domain_hash, port)
}
