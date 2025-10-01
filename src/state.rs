use tatic_lib::{GameState, PlayerId};
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::RwLock;
use uuid::Uuid;

/// ID de uma partida
pub type MatchId = String;

/// Estado de uma partida
#[derive(Clone)]
pub struct Match {
    pub id: MatchId,
    pub state: GameState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Match {
    /// Cria nova partida
    pub fn new(player1: PlayerId, player2: PlayerId) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: format!("match-{}", Uuid::new_v4()),
            state: GameState::new(player1, player2),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Estado compartilhado da aplicação
#[derive(Clone)]
pub struct AppState {
    /// Partidas ativas
    pub matches: Arc<RwLock<HashMap<MatchId, Match>>>,
    /// Observers conectados via WebSocket
    pub observers: Arc<RwLock<HashMap<MatchId, Vec<tokio::sync::mpsc::Sender<String>>>>>,
}

impl AppState {
    /// Cria novo estado da aplicação
    pub fn new() -> Self {
        let state = Self {
            matches: Arc::new(RwLock::new(HashMap::new())),
            observers: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Inicializa com partidas de exemplo
        state.init_example_matches();
        
        state
    }
    
    /// Inicializa partidas de exemplo (hardcoded)
    fn init_example_matches(&self) {
        let matches = vec![
            Match::new("alice".to_string(), "bob".to_string()),
            Match::new("player1".to_string(), "player2".to_string()),
            Match::new("human".to_string(), "ai".to_string()),
        ];
        
        // Clona para evitar bloqueio durante o loop
        let matches_lock = self.matches.clone();
        
        tokio::spawn(async move {
            let mut matches_map = matches_lock.write().await;
            for match_data in matches {
                tracing::info!("📋 Criando partida exemplo: {}", match_data.id);
                let id = match_data.id.clone();
                matches_map.insert(id, match_data);
            }
            tracing::info!("✅ {} partidas exemplo criadas", matches_map.len());
        });
    }
    
    /// Obtém uma partida
    pub async fn get_match(&self, match_id: &str) -> Option<Match> {
        self.matches.read().await.get(match_id).cloned()
    }
    
    /// Atualiza uma partida
    pub async fn update_match(&self, match_id: &str, new_state: GameState) {
        let mut matches = self.matches.write().await;
        if let Some(match_data) = matches.get_mut(match_id) {
            match_data.state = new_state;
            match_data.updated_at = chrono::Utc::now();
        }
    }
    
    /// Cria nova partida
    pub async fn create_match(&self, player1: PlayerId, player2: PlayerId) -> MatchId {
        let match_data = Match::new(player1, player2);
        let match_id = match_data.id.clone();
        
        self.matches.write().await.insert(match_id.clone(), match_data);
        
        match_id
    }
    
    /// Lista todas as partidas
    pub async fn list_matches(&self) -> Vec<MatchId> {
        self.matches.read().await.keys().cloned().collect()
    }
    
    /// Notifica observers via WebSocket
    pub async fn notify_observers(&self, match_id: &str, message: String) {
        let observers = self.observers.read().await;
        
        if let Some(senders) = observers.get(match_id) {
            // Envia para todos os observers
            for sender in senders {
                let _ = sender.send(message.clone()).await;
            }
        }
    }
    
    /// Adiciona observer
    pub async fn add_observer(
        &self,
        match_id: String,
        sender: tokio::sync::mpsc::Sender<String>,
    ) {
        let mut observers = self.observers.write().await;
        observers.entry(match_id).or_insert_with(Vec::new).push(sender);
    }
}
