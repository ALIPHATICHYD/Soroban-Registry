use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChatSession {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub contract_id: Option<Uuid>,
    pub session_title: Option<String>,
    pub context_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChatMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
    #[sqlx(rename = "contract_code_snippet")]
    pub contract_code_snippet: Option<String>,
    pub token_count: Option<i32>,
    pub model_used: Option<String>,
    #[sqlx(rename = "response_time_ms")]
    pub response_time_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractContext {
    pub contract_id: String,
    pub contract_name: String,
    pub contract_code: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub network: Option<shared::models::Network>,
}

impl From<shared::models::Contract> for ContractContext {
    fn from(contract: shared::models::Contract) -> Self {
        Self {
            contract_id: contract.id.to_string(),
            contract_name: contract.name,
            contract_code: String::new(), // Would need to fetch from source storage
            description: contract.description,
            category: contract.category,
            tags: contract.tags.iter().map(|t| t.name.clone()).collect(),
            network: Some(contract.network),
        }
    }
}
