use axum::{
    Router, routing::get,
    Json, extract::{Query, State},
    http::StatusCode, response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;
use crate::store::AppState;

/// GET /api/history?userId=&opponentId=  (mirrors history.route.ts)
#[derive(Deserialize)]
struct HistoryQuery {
    #[serde(rename = "userId")]
    user_id: Option<String>,
    #[serde(rename = "opponentId")]
    opponent_id: Option<String>,
}

async fn get_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    let user_id = match &query.user_id {
        Some(id) if !id.is_empty() => id.clone(),
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "userId & opponentId required"}))).into_response(),
    };

    let opponent_id = match &query.opponent_id {
        Some(id) if !id.is_empty() => id.clone(),
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "userId & opponentId required"}))).into_response(),
    };

    let data = state.db.perspective_history(&user_id, &opponent_id);
    Json(data).into_response()
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(get_history))
        .with_state(state)
}
