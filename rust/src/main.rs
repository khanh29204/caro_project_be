mod config;
mod db;
mod routes;
mod sockets;
mod store;
mod types;

use axum::Router;
use socketioxide::SocketIo;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    // Init tracing
    tracing_subscriber::fmt::init();

    // Load .env
    dotenvy::dotenv().ok();

    let cfg = config::Config::from_env();
    let port = cfg.port;

    // Init DB
    let database = db::Database::new();
    database.init();
    tracing::info!("Database initialized");

    // Shared state
    let app_state = Arc::new(store::AppState::new(database));

    // Setup Socket.IO with path /api/socket.io (matching JS backend)
    let (layer, io) = SocketIo::builder()
        .with_state(app_state.clone())
        .build_layer();

    // Register socket handlers
    sockets::setup_socket(&io, app_state.clone());

    // CORS - allow all origins (matching JS backend: origin: true)
    let cors = CorsLayer::very_permissive();

    // Build axum router
    let app = Router::new()
        .merge(routes::health::router())
        .nest("/api/users", routes::users::router())
        .nest("/api/rooms", routes::rooms::router(app_state.clone(), io.clone()))
        .nest("/api/history", routes::history::router(app_state.clone()))
        .layer(layer)     // Socket.IO layer (handles upgrade to WebSocket)
        .layer(cors);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Server listening on http://{addr} with Rust/Axum runtime");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
