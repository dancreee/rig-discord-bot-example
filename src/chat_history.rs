use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::debug;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UserHistory {
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Default)]
pub struct ChatHistoryManager {
    histories: RwLock<HashMap<String, UserHistory>>,
    storage_path: String,
}

impl ChatHistoryManager {
    pub fn new(storage_path: &str) -> Self {
        Self {
            histories: RwLock::new(HashMap::new()),
            storage_path: storage_path.to_string(),
        }
    }

    pub async fn load_histories(&self) -> Result<()> {
        let path = Path::new(&self.storage_path);
        if !path.exists() {
            fs::create_dir_all(path)?;
            return Ok(());
        }

        let mut histories = self.histories.write().await;
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(user_id) = path.file_stem().and_then(|s| s.to_str()) {
                    let content = fs::read_to_string(&path)?;
                    let history: UserHistory = serde_json::from_str(&content)?;
                    histories.insert(user_id.to_string(), history);
                }
            }
        }
        Ok(())
    }

    pub async fn add_message(&self, user_id: &str, message: ChatMessage) -> Result<()> {
        let mut histories = self.histories.write().await;
        let history = histories.entry(user_id.to_string()).or_default();
        history.messages.push(message.clone());
        debug!("Added message for user {}: {:?}", user_id, message);
        
        // Save to file
        let path = Path::new(&self.storage_path).join(format!("{}.json", user_id));
        let content = serde_json::to_string_pretty(&history)?;
        fs::write(&path, &content)?;
        debug!("Saved history to file: {:?}", path);
        
        Ok(())
    }

    pub async fn get_history(&self, user_id: &str) -> Vec<ChatMessage> {
        let histories = self.histories.read().await;
        let history = histories
            .get(user_id)
            .map(|h| h.messages.clone())
            .unwrap_or_default();
        debug!("Retrieved {} messages for user {}", history.len(), user_id);
        history
    }
} 