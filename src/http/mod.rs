use std::{net::SocketAddr, path::Path};

use axum::{Router, routing::IntoMakeService};
use axum_server::tls_rustls::RustlsConfig;
use tracing::info;

use crate::{error::Result, http::router::router, state::State};

mod router;

async fn run_server(addr: SocketAddr, service: IntoMakeService<Router>, enable_ssl: bool) -> Result<()> {
    let cert = Path::new("plex_cert.pem");
    let key = Path::new("plex_key.pem");

    if enable_ssl && cert.exists() && key.exists() {
        info!("Jelly Bridge listening securely on https://{}", addr);
        let config = RustlsConfig::from_pem_file(cert, key).await?;
        axum_server::bind_rustls(addr, config)
            .serve(service)
            .await?;
    } else {
        info!("Jelly Bridge listening on http://{}", addr);
        axum_server::bind(addr).serve(service).await?;
    }

    Ok(())
}

pub async fn serve(state: State, enable_ssl: bool) -> Result<()> {
    let router = router(state);
    let addr = "0.0.0.0:9096".parse::<SocketAddr>().unwrap();

    run_server(addr, router.into_make_service(), enable_ssl).await?;

    Ok(())
}
