use anyhow::{anyhow, Context, Result};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::time::timeout;
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIProvider {
    OpenAI,
    Anthropic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user", "assistant", "system"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIRequest {
    pub messages: Vec<ChatMessage>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
    pub contract_context: Option<ContractContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractContext {
    pub contract_id: String,
    pub contract_name: String,
    pub contract_code: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResponse {
    pub content: String,
    pub model_used: String,
    pub token_count: Option<u32>,
    pub response_time_ms: u64,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub provider: AIProvider,
    pub api_key: String,
    pub base_url: Option<String>,
    pub default_model: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

impl AIConfig {
    pub fn from_env() -> Result<Self> {
        let provider_str = std::env::var("AI_PROVIDER").unwrap_or_else(|_| "openai".to_string());

        let provider = match provider_str.to_lowercase().as_str() {
            "anthropic" => AIProvider::Anthropic,
            _ => AIProvider::OpenAI,
        };

        let api_key = match provider {
            AIProvider::OpenAI => std::env::var("OPENAI_API_KEY")
                .context("OPENAI_API_KEY environment variable is required")?,
            AIProvider::Anthropic => std::env::var("ANTHROPIC_API_KEY")
                .context("ANTHROPIC_API_KEY environment variable is required")?,
        };

        let default_model = match provider {
            AIProvider::OpenAI => {
                std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4-turbo-preview".to_string())
            }
            AIProvider::Anthropic => std::env::var("ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "claude-3-opus-20240229".to_string()),
        };

        let base_url = std::env::var("AI_BASE_URL").ok();

        let timeout_seconds = std::env::var("AI_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);

        let max_retries = std::env::var("AI_MAX_RETRIES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3);

        Ok(Self {
            provider,
            api_key,
            base_url,
            default_model,
            timeout_seconds,
            max_retries,
        })
    }
}

pub struct AIService {
    config: AIConfig,
    http_client: reqwest::Client,
}

impl AIService {
    pub fn new(config: AIConfig) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            config,
            http_client,
        })
    }

    pub fn from_env() -> Result<Self> {
        let config = AIConfig::from_env()?;
        Self::new(config)
    }

    pub async fn chat(&self, request: AIRequest) -> Result<AIResponse> {
        let start_time = std::time::Instant::now();

        let model = request
            .model
            .clone()
            .unwrap_or(self.config.default_model.clone());
        let messages = self.build_messages(&request)?;

        let response = match self.config.provider {
            AIProvider::OpenAI => {
                self.call_openai(&messages, &model, request.max_tokens, request.stream)
                    .await
            }
            AIProvider::Anthropic => {
                self.call_anthropic(&messages, &model, request.max_tokens, request.stream)
                    .await
            }
        }?;

        let response_time = start_time.elapsed().as_millis() as u64;

        Ok(AIResponse {
            content: response.content,
            model_used: model,
            token_count: response.token_count,
            response_time_ms: response_time,
            metadata: response.metadata,
        })
    }

    fn build_messages(&self, request: &AIRequest) -> Result<Vec<ChatMessage>> {
        let mut messages = Vec::new();

        // Add system prompt if there's contract context
        if let Some(ctx) = &request.contract_context {
            let system_msg = format!(
                "You are an expert in Soroban smart contracts (Stellar's blockchain platform). \
                 You are analyzing the following contract:\n\n\
                 Name: {}\n\
                 Description: {}\n\
                 Category: {}\n\
                 Tags: {}\n\n\
                 Contract Code:\n```rust\n{}\n```\n\n\
                 Provide accurate, concise, and helpful responses. Use markdown formatting. \
                 When suggesting code changes, show the full corrected code snippet.",
                ctx.contract_name,
                ctx.description.as_deref().unwrap_or("No description"),
                ctx.category.as_deref().unwrap_or("Uncategorized"),
                ctx.tags.join(", "),
                ctx.contract_code
            );
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: system_msg,
            });
        }

        // Add user messages
        messages.extend(request.messages.clone());

        Ok(messages)
    }

    async fn call_openai(
        &self,
        messages: &[ChatMessage],
        model: &str,
        max_tokens: Option<u32>,
        stream: Option<bool>,
    ) -> Result<OpenAIResponse> {
        let request_body = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": max_tokens.unwrap_or(4096),
            "stream": stream.unwrap_or(false),
        });

        let url = self
            .config
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");

        let response = self
            .http_client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("OpenAI API error: {} - {}", status, error_text);
            anyhow::bail!("OpenAI API error: {} - {}", status, error_text);
        }

        let openai_resp: OpenAIResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        Ok(openai_resp)
    }

    async fn call_anthropic(
        &self,
        messages: &[ChatMessage],
        model: &str,
        max_tokens: Option<u32>,
        stream: Option<bool>,
    ) -> Result<OpenAIResponse> {
        // Convert messages to Anthropic format
        let anthropic_messages: Vec<Value> = messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect();

        let system_prompt = messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let request_body = serde_json::json!({
            "model": model,
            "messages": anthropic_messages,
            "system": system_prompt,
            "max_tokens": max_tokens.unwrap_or(4096),
            "stream": stream.unwrap_or(false),
        });

        let url = self
            .config
            .base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com/v1/messages");

        let response = self
            .http_client
            .post(url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Anthropic API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Anthropic API error: {} - {}", status, error_text);
            anyhow::bail!("Anthropic API error: {} - {}", status, error_text);
        }

        let anthropic_resp: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        // Convert Anthropic response to our format
        let content = anthropic_resp["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|item| item["text"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(OpenAIResponse {
            content,
            model: model.to_string(),
            token_count: anthropic_resp["usage"]["output_tokens"]
                .as_u64()
                .map(|v| v as u32),
            metadata: anthropic_resp,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIResponse {
    content: String,
    model: String,
    token_count: Option<u32>,
    metadata: Value,
}
