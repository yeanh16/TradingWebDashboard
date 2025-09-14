use axum::{
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};

/// GET /api/health - Health check endpoint
pub async fn health() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({
        "status": "ok",
        "service": "crypto-dash-api",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// GET /api/ready - Readiness check endpoint
pub async fn ready() -> Result<Json<Value>, StatusCode> {
    // In a real implementation, check if services are ready
    Ok(Json(json!({
        "status": "ready",
        "service": "crypto-dash-api",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "dependencies": {
            "stream_hub": "ok",
            "cache": "ok",
            "exchanges": "ok"
        }
    })))
}