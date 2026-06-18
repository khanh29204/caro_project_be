use axum::{Router, routing::get, Json};

/// GET /health -> { ok: true }
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true}))
}

pub fn router() -> Router {
    Router::new().route("/health", get(health_check))
}
