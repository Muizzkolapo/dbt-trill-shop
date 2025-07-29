use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported LLM models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Model {
    // OpenAI Models
    GPT4Turbo,
    GPT4,
    GPT35Turbo,
    
    // Anthropic Models
    Claude3Opus,
    Claude3Sonnet,
    Claude3Haiku,
    
    // Open Source Models (via Ollama)
    Llama3,
    Mixtral,
    CodeLlama,
    
    // Custom model string
    Custom(String),
}

impl Model {
    pub fn as_str(&self) -> &str {
        match self {
            Model::GPT4Turbo => "gpt-4-turbo-preview",
            Model::GPT4 => "gpt-4",
            Model::GPT35Turbo => "gpt-3.5-turbo",
            Model::Claude3Opus => "claude-3-opus-20240229",
            Model::Claude3Sonnet => "claude-3-sonnet-20240229",
            Model::Claude3Haiku => "claude-3-haiku-20240307",
            Model::Llama3 => "llama3",
            Model::Mixtral => "mixtral",
            Model::CodeLlama => "codellama",
            Model::Custom(s) => s,
        }
    }
}

/// LLM request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub messages: Vec<Message>,
    pub model: Model,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
    pub tools: Option<Vec<Tool>>,
    pub response_format: Option<ResponseFormat>,
}

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool call in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Response format specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseFormat {
    Text,
    JsonObject,
    JsonSchema(serde_json::Value),
}

/// LLM response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub finish_reason: FinishReason,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Main LLM interface trait
#[async_trait]
pub trait LLMInterface: Send + Sync {
    /// Send a completion request to the LLM
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse>;
    
    /// Stream a completion response
    async fn stream_complete(
        &self,
        request: LLMRequest,
        callback: Box<dyn Fn(String) + Send>,
    ) -> Result<LLMResponse>;
    
    /// Get embeddings for text
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
    
    /// Check if the provider is available
    async fn health_check(&self) -> Result<bool>;
}

/// Configuration for LLM providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub default_model: Model,
    pub timeout_seconds: Option<u64>,
    pub max_retries: Option<u32>,
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
}

/// Context passed to LLM for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisContext {
    pub pr_diff: String,
    pub model_definitions: HashMap<String, String>,
    pub lineage_graph: String,
    pub test_results: Option<HashMap<String, TestResult>>,
    pub historical_metrics: Option<HistoricalMetrics>,
    pub warehouse_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub status: String,
    pub message: Option<String>,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalMetrics {
    pub avg_query_time: f64,
    pub avg_bytes_scanned: u64,
    pub failure_rate: f64,
    pub last_30_days_cost: f64,
}

/// Agent-specific analysis request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAnalysisRequest {
    pub agent_type: AgentType,
    pub context: AnalysisContext,
    pub specific_focus: Vec<String>,
    pub output_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    Impact,
    Quality,
    Performance,
}

/// Structured analysis response from agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAnalysisResponse {
    pub findings: Vec<Finding>,
    pub recommendations: Vec<Recommendation>,
    pub metrics: HashMap<String, f64>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub severity: Severity,
    pub category: String,
    pub description: String,
    pub evidence: Vec<String>,
    pub affected_resources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub priority: Priority,
    pub action: String,
    pub rationale: String,
    pub implementation_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Urgent,
    High,
    Medium,
    Low,
}