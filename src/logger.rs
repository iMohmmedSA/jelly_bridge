use tracing::info;
use tracing_appender::rolling;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init(filter: &str) {
    let env_filter = EnvFilter::new(filter);

    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_ansi(false)
        .with_filter(env_filter.clone());

    let file_appender = rolling::daily("logs", ".log");

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_filter(env_filter);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    info!("Logger initialized.")
}
