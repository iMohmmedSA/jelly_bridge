use std::str::FromStr;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tracing::info;
use uuid::Uuid;

use crate::error::Result;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn init() -> Result<Self> {
        let url = "sqlite://jelly_bridge.db";
        info!("Connecting to SQLite database at {}...", url);

        let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        info!("Running database migrations...");
        sqlx::migrate!("./migrations").run(&pool).await?;

        info!("Database connected successfully!");

        Ok(Self { pool })
    }

    pub async fn is_claimed(&self) -> Result<bool> {
        let record = sqlx::query!(
            "SELECT plex_auth_token FROM server_identity WHERE id = 1 AND plex_auth_token IS NOT NULL"
        ).fetch_optional(&self.pool).await?;
        Ok(record.is_some())
    }

    pub async fn get_or_create_machine_id(&self) -> Result<String> {
        let record = sqlx::query!("SELECT machine_identifier FROM server_identity WHERE id = 1")
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = record {
            return Ok(row.machine_identifier);
        }

        info!("Generating Server Identity...");

        let raw_string = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let machine_id = raw_string[0..40].to_string();

        sqlx::query!(
            "INSERT INTO server_identity (id, machine_identifier, server_name) VALUES (1, ?, 'JellyBridge')",
            machine_id
        )
        .execute(&self.pool)
        .await?;

        Ok(machine_id)
    }

    pub async fn get_server_token(&self) -> Result<Option<String>> {
        let record = sqlx::query!("SELECT plex_auth_token FROM server_identity WHERE id = 1")
            .fetch_optional(&self.pool)
            .await?;

        Ok(record.and_then(|r| r.plex_auth_token))
    }

    pub async fn save_server_token(&self, token: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE server_identity SET plex_auth_token = ? WHERE id = 1",
            token
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_jellyfin_key(&self, plex_user_id: i64) -> Result<Option<String>> {
        let record = sqlx::query!(
            "SELECT jellyfin_api_key FROM users WHERE plex_user_id = ?",
            plex_user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| r.jellyfin_api_key))
    }
}
