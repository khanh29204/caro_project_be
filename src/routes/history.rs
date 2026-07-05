use axum::{
    Router, routing::get,
    Json, extract::{Query, State, Path},
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

#[derive(Deserialize)]
struct ConflictQuery {
    #[serde(rename = "oldId")]
    old_id: String,
    #[serde(rename = "newId")]
    new_id: String,
}

async fn check_conflict(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ConflictQuery>,
) -> impl IntoResponse {
    let old_has = state.db.check_history(&query.old_id);
    let new_has = state.db.check_history(&query.new_id);

    Json(serde_json::json!({
        "oldHasHistory": old_has,
        "newHasHistory": new_has,
        "hasConflict": old_has && new_has
    })).into_response()
}

#[derive(Deserialize)]
struct MergeBody {
    #[serde(rename = "oldId")]
    old_id: String,
    #[serde(rename = "newId")]
    new_id: String,
    resolution: String, // "keep_old", "keep_new", "sum"
}

async fn merge_history(
    State(state): State<Arc<AppState>>,
    Json(body): Json<MergeBody>,
) -> impl IntoResponse {
    state.db.merge_history(&body.old_id, &body.new_id, &body.resolution);
    Json(serde_json::json!({"success": true})).into_response()
}

async fn get_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let data = state.db.get_profile(&id);
    Json(data).into_response()
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(get_history))
        .route("/check-conflict", get(check_conflict))
        .route("/merge", axum::routing::post(merge_history))
        .route("/profile/{id}", get(get_profile))
        .with_state(state)
}
