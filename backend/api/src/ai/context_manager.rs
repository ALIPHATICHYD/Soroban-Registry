use crate::ai::models::{ChatMessage, ChatSession};
use crate::ai::service::ContractContext;
use serde_json::Value;
use sqlx::{postgres::PgPool, types::Uuid};

/// Manages conversation context and chat history for AI sessions
#[derive(Clone)]
pub struct ContextManager {
    db: PgPool,
}

impl ContextManager {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Create a new chat session
    pub async fn create_session(
        &self,
        user_id: Option<Uuid>,
        contract_id: Option<Uuid>,
        context_type: &str,
    ) -> sqlx::Result<ChatSession> {
        sqlx::query_as::<_, ChatSession>(
            r#"
            INSERT INTO ai_chat_sessions (user_id, contract_id, context_type)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, contract_id, session_title, context_type,
                      created_at, updated_at, message_count, is_active
            "#,
        )
        .bind(user_id)
        .bind(contract_id)
        .bind(context_type)
        .fetch_one(&self.db)
        .await
    }

    /// Get session with messages
    pub async fn get_session_with_messages(
        &self,
        session_id: Uuid,
    ) -> sqlx::Result<(ChatSession, Vec<ChatMessage>)> {
        let session = sqlx::query_as::<_, ChatSession>(
            "SELECT id, user_id, contract_id, session_title, context_type, \
             created_at, updated_at, message_count, is_active \
             FROM ai_chat_sessions WHERE id = $1",
        )
        .bind(session_id)
        .fetch_one(&self.db)
        .await?;

        let messages = sqlx::query_as::<_, ChatMessage>(
            r#"
            SELECT id, session_id, role, content, contract_code_snippet,
                   token_count, model_used, response_time_ms, created_at, metadata
            FROM ai_chat_messages
            WHERE session_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.db)
        .await?;

        Ok((session, messages))
    }

    /// Add a message to session
    pub async fn add_message(
        &self,
        session_id: Uuid,
        role: &str,
        content: &str,
        contract_code_snippet: Option<&str>,
        token_count: Option<i32>,
        model_used: Option<&str>,
        response_time_ms: Option<i32>,
        metadata: Option<Value>,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO ai_chat_messages (
                session_id, role, content, contract_code_snippet,
                token_count, model_used, response_time_ms, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(session_id)
        .bind(role)
        .bind(content)
        .bind(contract_code_snippet)
        .bind(token_count)
        .bind(model_used)
        .bind(response_time_ms)
        .bind(metadata)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get recent sessions for a user
    pub async fn get_user_sessions(
        &self,
        user_id: Uuid,
        limit: i32,
    ) -> sqlx::Result<Vec<ChatSession>> {
        sqlx::query_as::<_, ChatSession>(
            r#"
            SELECT id, user_id, contract_id, session_title, context_type,
                   created_at, updated_at, message_count, is_active
            FROM ai_chat_sessions
            WHERE user_id = $1
            ORDER BY updated_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit as i64)
        .fetch_all(&self.db)
        .await
    }

    /// Get contract context from database
    pub async fn get_contract_context(
        &self,
        contract_id: Uuid,
    ) -> sqlx::Result<Option<ContractContext>> {
        // ContractContext doesn't derive FromRow (and has a `network`
        // field this query doesn't select), so we fetch into a
        // private row struct and convert.
        #[derive(sqlx::FromRow)]
        struct Row {
            contract_id: String,
            contract_name: String,
            description: Option<String>,
            category: Option<String>,
            tags: Vec<String>,
        }
        let row = sqlx::query_as::<_, Row>(
            r#"
            SELECT
                c.id::text AS contract_id,
                c.name AS contract_name,
                c.description,
                c.category,
                COALESCE(
                    array_agg(t.name) FILTER (WHERE t.id IS NOT NULL),
                    ARRAY[]::text[]
                ) AS tags
            FROM contracts c
            LEFT JOIN contract_tags ct ON c.id = ct.contract_id
            LEFT JOIN tags t ON ct.tag_id = t.id
            WHERE c.id = $1
            GROUP BY c.id, c.name, c.description, c.category
            "#,
        )
        .bind(contract_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|r| ContractContext {
            contract_id: r.contract_id,
            contract_name: r.contract_name,
            contract_code: String::new(),
            description: r.description,
            category: r.category,
            tags: r.tags,
        }))
    }

    /// Update session title based on first message
    pub async fn update_session_title(
        &self,
        session_id: Uuid,
        first_message: &str,
    ) -> sqlx::Result<()> {
        let title = if first_message.len() > 50 {
            format!("{}...", &first_message[..47])
        } else {
            first_message.to_string()
        };

        sqlx::query("UPDATE ai_chat_sessions SET session_title = $1 WHERE id = $2")
            .bind(title)
            .bind(session_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }
}
