use axum::{
    Router, routing::{post, delete},
    Json, extract::{Path, State},
    http::StatusCode, response::IntoResponse,
};
use socketioxide::SocketIo;
use std::sync::Arc;
use crate::store::{self, AppState};

/// POST /api/rooms -> { roomId }  (mirrors rooms.route.ts)
async fn create_room(
    State((state, _io)): State<(Arc<AppState>, SocketIo)>,
) -> Json<serde_json::Value> {
    let room = store::make_room(None);
    let room_id = room.id.clone();
    state.rooms.write().insert(room_id.clone(), room);
    Json(serde_json::json!({ "roomId": room_id }))
}

/// DELETE /api/rooms/:roomId  (mirrors rooms.route.ts)
async fn delete_room(
    State((state, io)): State<(Arc<AppState>, SocketIo)>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    let exists = {
        let rooms = state.rooms.read();
        rooms.contains_key(&room_id)
    };

    if !exists {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Room not found"}))).into_response();
    }

    // Emit room-deleted via Socket.IO
    io.to(room_id.clone())
        .emit("room-deleted", &room_id)
        .ok();

    state.rooms.write().remove(&room_id);
    StatusCode::NO_CONTENT.into_response()
}

pub fn router(state: Arc<AppState>, io: SocketIo) -> Router {
    Router::new()
        .route("/", post(create_room))
        .route("/{roomId}", delete(delete_room))
        .with_state((state, io))
}
