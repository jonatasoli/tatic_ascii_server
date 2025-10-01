use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use tatic_lib::{ai_choose_action, apply_action, Action, PlayerId};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::state::{AppState, MatchId};

/// Query params para GET /state
#[derive(Deserialize)]
pub struct StateQuery {
    match_id: String,
}

/// Request body para POST /action
#[derive(Deserialize)]
pub struct ActionRequest {
    match_id: MatchId,
    player_id: PlayerId,
    action: Action,
}

/// Response para requisições bem-sucedidas
#[derive(Serialize)]
pub struct SuccessResponse<T> {
    success: bool,
    data: T,
}

/// Response para erros
#[derive(Serialize)]
pub struct ErrorResponse {
    success: bool,
    error: String,
}

/// Cria as rotas REST
pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(root_handler))
        .route("/state", get(get_state_handler))
        .route("/action", post(post_action_handler))
        .route("/matches", get(list_matches_handler))
        .route("/match/create", post(create_match_handler))
        .route("/ai/action", post(ai_action_handler))
        .with_state(state)
}

/// Handler raiz - informações da API
async fn root_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "RPG ASCII Tático - Servidor",
        "version": "0.1.0",
        "endpoints": {
            "GET /": "Informações da API",
            "GET /state?match_id={id}": "Obtém estado do jogo",
            "POST /action": "Envia ação do jogador",
            "GET /matches": "Lista partidas disponíveis",
            "POST /match/create": "Cria nova partida",
            "POST /ai/action": "Solicita ação da IA",
            "WS /ws?match_id={id}": "WebSocket para observar partida"
        }
    }))
}

/// GET /state - Retorna estado atual da partida
async fn get_state_handler(
    Query(params): Query<StateQuery>,
    State(state): State<AppState>,
) -> Result<Json<SuccessResponse<tatic_lib::GameState>>, (StatusCode, Json<ErrorResponse>)> {
    info!("📥 GET /state - match_id: {}", params.match_id);
    
    match state.get_match(&params.match_id).await {
        Some(match_data) => {
            info!("✅ Estado retornado para partida {}", params.match_id);
            Ok(Json(SuccessResponse {
                success: true,
                data: match_data.state,
            }))
        }
        None => {
            warn!("❌ Partida não encontrada: {}", params.match_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    success: false,
                    error: format!("Partida {} não encontrada", params.match_id),
                }),
            ))
        }
    }
}

/// POST /action - Processa ação do jogador
async fn post_action_handler(
    State(state): State<AppState>,
    Json(request): Json<ActionRequest>,
) -> Result<Json<SuccessResponse<tatic_lib::GameState>>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "📥 POST /action - match: {}, player: {}, action: {:?}",
        request.match_id, request.player_id, request.action
    );
    
    // Obtém partida
    let match_data = state.get_match(&request.match_id).await.ok_or_else(|| {
        warn!("❌ Partida não encontrada: {}", request.match_id);
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("Partida {} não encontrada", request.match_id),
            }),
        )
    })?;
    
    // Log detalhado ANTES da ação
    info!(
        "📊 Estado ANTES - Turno: {}, Contador: {}, Fase: {:?}",
        match_data.state.turn, match_data.state.turn_count, match_data.state.phase
    );
    
    // Aplica ação
    match apply_action(&match_data.state, &request.player_id, request.action.clone()) {
        Ok(new_state) => {
            // Log detalhado DEPOIS da ação
            info!(
                "📊 Estado DEPOIS - Turno: {}, Contador: {}, Fase: {:?}",
                new_state.turn, new_state.turn_count, new_state.phase
            );
            info!("✅ Ação aplicada com sucesso");
            
            // Atualiza estado
            state.update_match(&request.match_id, new_state.clone()).await;
            
            // Notifica observers via WebSocket
            let notification = serde_json::json!({
                "type": "state_update",
                "match_id": request.match_id,
                "state": &new_state,
            });
            
            state
                .notify_observers(&request.match_id, notification.to_string())
                .await;
            
            Ok(Json(SuccessResponse {
                success: true,
                data: new_state,
            }))
        }
        Err(e) => {
            error!("❌ Erro ao aplicar ação: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error: e.to_string(),
                }),
            ))
        }
    }
}

/// GET /matches - Lista partidas disponíveis
async fn list_matches_handler(
    State(state): State<AppState>,
) -> Json<SuccessResponse<Vec<serde_json::Value>>> {
    info!("📥 GET /matches");
    
    let matches = state.matches.read().await;
    let match_list: Vec<_> = matches
        .values()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "players": m.state.players,
                "turn": m.state.turn,
                "turn_count": m.state.turn_count,
                "phase": m.state.phase,
                "created_at": m.created_at,
                "updated_at": m.updated_at,
            })
        })
        .collect();
    
    info!("✅ Retornando {} partidas", match_list.len());
    
    Json(SuccessResponse {
        success: true,
        data: match_list,
    })
}

/// Request para criar partida
#[derive(Deserialize)]
pub struct CreateMatchRequest {
    player1: PlayerId,
    player2: PlayerId,
}

/// POST /match/create - Cria nova partida
async fn create_match_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateMatchRequest>,
) -> Json<SuccessResponse<String>> {
    info!(
        "📥 POST /match/create - player1: {}, player2: {}",
        request.player1, request.player2
    );
    
    let match_id = state.create_match(request.player1, request.player2).await;
    
    info!("✅ Partida criada: {}", match_id);
    
    Json(SuccessResponse {
        success: true,
        data: match_id,
    })
}

/// Request para ação da IA
#[derive(Deserialize)]
pub struct AiActionRequest {
    match_id: MatchId,
    ai_player: PlayerId,
}

/// POST /ai/action - Solicita ação da IA
async fn ai_action_handler(
    State(state): State<AppState>,
    Json(request): Json<AiActionRequest>,
) -> Result<Json<SuccessResponse<Action>>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "🤖 POST /ai/action - match: {}, ai_player: {}",
        request.match_id, request.ai_player
    );
    
    let match_data = state.get_match(&request.match_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("Partida {} não encontrada", request.match_id),
            }),
        )
    })?;
    
    // IA escolhe ação
    match ai_choose_action(&match_data.state, &request.ai_player) {
        Some(action) => {
            info!("🎯 IA escolheu ação: {:?}", action);
            Ok(Json(SuccessResponse {
                success: true,
                data: action,
            }))
        }
        None => {
            warn!("❌ IA não conseguiu escolher ação");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error: "IA não conseguiu escolher ação".to_string(),
                }),
            ))
        }
    }
}
