use anyhow::{Context, Result};
use axum::{routing::get, Json, Router};
use serde_json::json;

pub async fn serve() -> Result<()> {
    let app = Router::new().route("/health", get(health_check));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .context("failed to bind health server to port 3000")?;

    axum::serve(listener, app)
        .await
        .context("health server error")?;

    Ok(())
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({"status": "ok"}))
}
