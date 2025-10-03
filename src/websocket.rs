use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
    routing::get,
    Router,
};
use serde::Deserialize;
use tracing::{error, info};
// IMPORTANTE: Importar StreamExt e SinkExt
use futures_util::{SinkExt, StreamExt};

use crate::state::AppState;

#[derive(Deserialize)]
struct WsQuery {
    match_id: String,
}

/// Cria rotas WebSocket
pub fn websocket_routes(state: AppState) -> Router {
    Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(state)
}

/// Handler para upgrade WebSocket
async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQuery>,
    State(state): State<AppState>,
) -> Response {
    info!("🔌 WebSocket connection request for match: {}", params.match_id);
    ws.on_upgrade(move |socket| handle_websocket(socket, params.match_id, state))
}

/// Gerencia conexão WebSocket
async fn handle_websocket(socket: WebSocket, match_id: String, state: AppState) {
    info!("✅ WebSocket connected for match: {}", match_id);
    
    // Split socket em sender e receiver
    let (mut sender, mut receiver) = socket.split();
    
    // Canal para receber broadcasts
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    
    // Registra observer
    state.add_observer(match_id.clone(), tx).await;
    
    // Envia estado inicial
    if let Some(match_data) = state.get_match(&match_id).await {
        let initial_state = serde_json::json!({
            "type": "initial_state",
            "match_id": match_id,
            "state": match_data.state,
        });
        
        // CORREÇÃO para Axum 0.8: Converter String para Utf8Bytes usando .into()
        if let Err(e) = sender
            .send(Message::Text(initial_state.to_string().into()))
            .await
        {
            error!("Erro ao enviar estado inicial: {}", e);
            return;
        }
    }
    
    // Task para enviar broadcasts
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            // CORREÇÃO para Axum 0.8: Converter String para Utf8Bytes
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });
    
    // Task para receber mensagens (ping/pong)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Ping(bytes) => {
                    // Para ping/pong, precisaríamos de uma referência mutável ao sender
                    // Por simplicidade, vamos apenas logar
                    info!("Recebido ping");
                }
                Message::Close(_) => {
                    info!("WebSocket fechado pelo cliente");
                    break;
                }
                _ => {}
            }
        }
    });
    
    // Aguarda alguma task terminar
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
    
    info!("🔌 WebSocket disconnected for match: {}", match_id);
}
