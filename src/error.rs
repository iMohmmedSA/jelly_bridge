use thiserror::Error;
use tokio::io;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("I/O error: {0}")]
    IO(#[from] io::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("XML Parse error: {0}")]
    XMLDeserialize(#[from] quick_xml::de::DeError),

    #[error("RSA Crypto error: {0}")]
    Rsa(#[from] rsa::errors::Error),

    #[error("PKCS8 error: {0}")]
    Pkcs8(#[from] rsa::pkcs8::Error),

    #[error("Rcgen error: {0}")]
    Rcgen(#[from] rcgen::Error),

    #[error("Generic error: {0}")]
    Generic(String),
}

pub type Result<T> = std::result::Result<T, Error>;
