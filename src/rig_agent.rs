// rig_agent.rs

use anyhow::{Context, Result};
use rig::providers::openai;
use rig::vector_store::in_memory_store::InMemoryVectorStore;
use rig::vector_store::VectorStore;
use rig::embeddings::EmbeddingsBuilder;
use rig::agent::Agent;
use rig::completion::Prompt;
use std::path::Path;
use std::fs;
use std::sync::Arc;
use crate::chat_history::{ChatHistoryManager, ChatMessage};
use chrono::Utc;
use tracing::debug;

pub struct RigAgent {
    agent: Arc<Agent<openai::CompletionModel>>,
    history_manager: Arc<ChatHistoryManager>,
}

impl RigAgent {
    pub async fn new() -> Result<Self> {
        let history_manager = Arc::new(ChatHistoryManager::new("chat_histories"));
        history_manager.load_histories().await?;
        
        // Initialize OpenAI client
        let openai_client = openai::Client::from_env();
        let embedding_model = openai_client.embedding_model(openai::TEXT_EMBEDDING_3_SMALL);

        // Create vector store
        let mut vector_store = InMemoryVectorStore::default();

        // Get the current directory and construct paths to markdown files
        let current_dir = std::env::current_dir()?;
        let documents_dir = current_dir.join("documents");

        let md1_path = documents_dir.join("Rig_guide.md");
        let md2_path = documents_dir.join("Rig_faq.md");
        let md3_path = documents_dir.join("Rig_examples.md");

        // Load markdown documents
        let md1_content = Self::load_md_content(&md1_path)?;
        let md2_content = Self::load_md_content(&md2_path)?;
        let md3_content = Self::load_md_content(&md3_path)?;

        // Create embeddings and add to vector store
        let embeddings = EmbeddingsBuilder::new(embedding_model.clone())
            .simple_document("Rig_guide", &md1_content)
            .simple_document("Rig_faq", &md2_content)
            .simple_document("Rig_examples", &md3_content)
            .build()
            .await?;

        vector_store.add_documents(embeddings).await?;

        // Create index
        let index = vector_store.index(embedding_model);

        // Create Agent
        let agent = Arc::new(openai_client.agent(openai::GPT_4O)
            .preamble("You are a knowledgeable but irreverent crypto expert, with a focus on infrastructure, L1/L2 dynamics, and DeFi. Your personality traits include:

            1. Direct Communication: You speak plainly and often use casual language. You're not afraid to be blunt when needed.
            
            2. Technical Knowledge:
            - Deep understanding of blockchain infrastructure (L1s, L2s, rollups)
            - Strong grasp of DeFi mechanics and tokenomics
            - Practical understanding of market dynamics
            
            3. Perspective:
            - Pragmatic rather than ideological
            - Focus on what actually works rather than theoretical perfection
            - Skeptical of hype but open to innovation
            
            4. Style:
            - Use casual language but maintain technical accuracy
            - Often employ dry humor or mild sarcasm
            - Keep responses concise and to the point
            - Occasionally use phrases like 'ser', 'anon', or other crypto slang
            - Don't shy away from calling out flaws or issues
            
            5. Key Beliefs:
            - Pragmatic view on centralization vs decentralization tradeoffs
            - Skeptical of 'one size fits all' solutions
            - Focus on actual user behavior over theoretical ideals
            - Understanding that markets and narratives drive much of crypto
            
            When responding:
            - Keep answers concise but informative
            - Use technical terms when appropriate but explain complex concepts simply
            - Be direct about both positives and negatives
            - Format code examples properly for Discord using triple backticks
            - Stay grounded in practical reality rather than theoretical ideals
            
            Remember: You're knowledgeable but not pretentious, technical but practical, and always focused on what actually works rather than what should work in theory.")
            .dynamic_context(2, index)
            .build());

        Ok(Self { agent, history_manager })
    }

    fn load_md_content<P: AsRef<Path>>(file_path: P) -> Result<String> {
        fs::read_to_string(file_path.as_ref())
            .with_context(|| format!("Failed to read markdown file: {:?}", file_path.as_ref()))
    }

    pub async fn process_message(&self, user_id: &str, content: &str) -> Result<String> {
        let history = self.history_manager.get_history(user_id).await;
        debug!("Retrieved history for user {}: {} messages", user_id, history.len());
        
        // Format history into a context string
        let context = history.iter().map(|msg| {
            format!("{}: {}", msg.role, msg.content)
        }).collect::<Vec<_>>().join("\n");
        
        debug!("Formatted context:\n{}", context);
        
        // Create prompt with history context
        let prompt = if context.is_empty() {
            content.to_string()
        } else {
            format!(
                "Previous conversation:\n{}\n\nCurrent message: {}",
                context, content
            )
        };
        
        debug!("Final prompt to agent:\n{}", prompt);

        // Get response from agent
        let response = self.agent.prompt(&prompt).await?;

        // Add messages to history
        self.history_manager.add_message(
            user_id,
            ChatMessage {
                role: "user".to_string(),
                content: content.to_string(),
                timestamp: Utc::now().timestamp(),
            },
        ).await?;

        self.history_manager.add_message(
            user_id,
            ChatMessage {
                role: "assistant".to_string(),
                content: response.clone(),
                timestamp: Utc::now().timestamp(),
            },
        ).await?;

        Ok(response)
    }
}