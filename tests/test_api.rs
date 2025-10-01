#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use tatic_lib::{Action, Coord};
    
    #[tokio::test]
    async fn test_root_endpoint() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        let response = server.get("/").await;
        
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let json: serde_json::Value = response.json();
        assert_eq!(json["name"], "RPG ASCII Tático - Servidor");
    }
    
    #[tokio::test]
    async fn test_list_matches() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        let response = server.get("/matches").await;
        
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let json: serde_json::Value = response.json();
        assert!(json["success"].as_bool().unwrap());
        assert!(json["data"].is_array());
    }
    
    #[tokio::test]
    async fn test_create_match() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        let response = server
            .post("/match/create")
            .json(&serde_json::json!({
                "player1": "test1",
                "player2": "test2"
            }))
            .await;
        
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let json: serde_json::Value = response.json();
        assert!(json["success"].as_bool().unwrap());
        assert!(json["data"].as_str().unwrap().starts_with("match-"));
    }
    
    #[tokio::test]
    async fn test_get_state() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        // Primeiro cria uma partida
        let create_response = server
            .post("/match/create")
            .json(&serde_json::json!({
                "player1": "test1",
                "player2": "test2"
            }))
            .await;
        
        let match_id = create_response.json()["data"].as_str().unwrap();
        
        // Então obtém o estado
        let response = server
            .get(&format!("/state?match_id={}", match_id))
            .await;
        
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let json: serde_json::Value = response.json();
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["turn"], "test1");
    }
    
    #[tokio::test]
    async fn test_post_action() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        // Cria partida
        let create_response = server
            .post("/match/create")
            .json(&serde_json::json!({
                "player1": "test1",
                "player2": "test2"
            }))
            .await;
        
        let match_id = create_response.json()["data"].as_str().unwrap();
        
        // Envia ação
        let response = server
            .post("/action")
            .json(&serde_json::json!({
                "match_id": match_id,
                "player_id": "test1",
                "action": {
                    "type": "EndTurn"
                }
            }))
            .await;
        
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let json: serde_json::Value = response.json();
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["turn"], "test2");
    }
    
    async fn create_test_app() -> Router {
        let state = crate::state::AppState::new();
        crate::routes::create_routes(state)
    }
}
