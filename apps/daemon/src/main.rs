use axum::{
    extract::{ws::WebSocketUpgrade, Path, State},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tracing::info;

use sandbox_core::SandboxManager;
use session_manager::SessionManager;
use shared_types::{CreateSessionRequest, SessionStatus};

#[derive(Clone)]
struct AppState {
    session_manager: Arc<SessionManager>,
    #[allow(dead_code)]
    sandbox_manager: Arc<SandboxManager>,
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    std::fs::create_dir_all("data")?;
    let cwd = std::env::current_dir()?;
    let db_path = cwd.join("data/sessions.db");
    let db_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| format!("sqlite:{}", db_path.display()));
    let session_manager = Arc::new(SessionManager::new(&db_url).await?);
    let sandbox_manager = Arc::new(SandboxManager::new()?);

    let state = AppState {
        session_manager,
        sandbox_manager,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/sessions", post(create_session).get(list_sessions))
        .route("/sessions/:id", get(get_session))
        .route("/sessions/:id/status", post(update_session_status))
        .route("/ws", get(ws_handler))
        .with_state(state);

    // Bind to loopback by default — the daemon must not be reachable from the
    // network. Override with AGENTICBOX_BIND_ADDR for trusted deployments.
    let bind_addr =
        std::env::var("AGENTICBOX_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("Daemon listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<shared_types::Session>, axum::http::StatusCode> {
    let session = state
        .session_manager
        .create(req.name, req.model_config, req.permissions, req.identity_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create session: {:?}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(session))
}

async fn list_sessions(
    State(state): State<AppState>,
) -> Result<Json<Vec<shared_types::Session>>, axum::http::StatusCode> {
    let sessions = state
        .session_manager
        .list()
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(sessions))
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<shared_types::Session>, axum::http::StatusCode> {
    let session = state
        .session_manager
        .get(id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;
    Ok(Json(session))
}

async fn update_session_status(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(status): Json<SessionStatus>,
) -> Result<(), axum::http::StatusCode> {
    state
        .session_manager
        .update_status(id, status)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(_state): State<AppState>) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: axum::extract::ws::WebSocket) {
    use axum::extract::ws::Message;
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            let _ = socket.send(Message::Text(format!("echo: {}", text))).await;
        }
    }
}
