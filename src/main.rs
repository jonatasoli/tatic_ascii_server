//! Servidor autoritativo do jogo tático ASCII
//!
//! Este servidor implementa:
//! - REST API para ações do jogo
//! - WebSocket para broadcast de estado
//! - Logging detalhado com tracing
//! - Gerenciamento de múltiplas partidas

use axum::{
    Router,
    http::{header, Method},
};
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{info, Level};

mod routes;
mod state;
mod websocket;
mod logging;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Inicializa logging
    logging::init_tracing();
    
    info!("🚀 Iniciando servidor do RPG ASCII Tático");
    
    // Cria estado compartilhado
    let app_state = state::AppState::new();
    
    // Configura CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE]);
    
    // Configura trace layer para logging de requests
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));
    
    // Monta rotas
    let app = Router::new()
        .merge(routes::create_routes(app_state.clone()))
        .merge(websocket::websocket_routes(app_state))
        .layer(cors)
        .layer(trace_layer);
    
    // Bind e serve
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("🎮 Servidor rodando em http://{}", addr);
    info!("📡 WebSocket disponível em ws://{}/ws", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Erro no servidor: {}", e))?;
    
    Ok(())
}
