use tokio::net::TcpListener;
use tracing::info;

use crate::{error::Result, http::router::router, state::State};

mod router;

pub async fn serve(state: State) -> Result<()> {
    let router = router(state);
    let addr = "0.0.0.0:9096";

    let listener = TcpListener::bind(addr).await?;

    info!("Jelly Bridge listening on http://{}", addr);

    axum::serve(listener, router).await?;
    Ok(())
}
