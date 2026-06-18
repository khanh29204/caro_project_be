use axum::{Router, routing::post, Json, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

/// POST /api/users -> { id, name }  (mirrors users.route.ts)
#[derive(Deserialize)]
struct CreateUserBody {
    name: Option<String>,
}

async fn create_user(Json(body): Json<CreateUserBody>) -> impl IntoResponse {
    let name = match &body.name {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Name required"}))).into_response(),
    };

    let id = nanoid::nanoid!(12);
    Json(serde_json::json!({ "id": id, "name": name })).into_response()
}

pub fn router() -> Router {
    Router::new().route("/", post(create_user))
}
