use super::interfaces::*;
use anyhow::{Result, Context};
use async_trait::async_trait;
use reqwest::{Client, header};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

/// Base trait for all LLM providers
#[async_trait]
pub trait LLMProvider: LLMInterface {
    fn name(&self) -> &str;
    fn supports_tools(&self) -> bool;
    fn supports_streaming(&self) -> bool;
}

/// OpenAI Provider Implementation
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    base_url: String,
    config: LLMConfig,
}

impl OpenAIProvider {
    pub fn new(config: LLMConfig) -> Result<Self> {
        let api_key = config.api_key.clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .context("OpenAI API key not found")?;
        
        let base_url = config.base_url.clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds.unwrap_or(60)))
            .build()?;
        
        Ok(Self {
            client,
            api_key,
            base_url,
            config,
        })
    }
    
    async fn make_request(&self, request: &LLMRequest) -> Result<LLMResponse> {
        let mut messages = vec![];
        
        if let Some(system) = &request.system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system
            }));
        }
        
        for msg in &request.messages {
            messages.push(json!({
                "role": match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                },
                "content": msg.content
            }));
        }
        
        let mut body = json!({
            "model": request.model.as_str(),
            "messages": messages,
            "temperature": request.temperature.unwrap_or(0.7),
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });
        
        if let Some(tools) = &request.tools {
            body["tools"] = json!(tools.iter().map(|t| json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters
                }
            })).collect::<Vec<_>>());
        }
        
        if let Some(format) = &request.response_format {
            match format {
                ResponseFormat::JsonObject => {
                    body["response_format"] = json!({"type": "json_object"});
                }
                _ => {}
            }
        }
        
        let response = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;
        
        let status = response.status();
        let text = response.text().await?;
        
        if !status.is_success() {
            return Err(anyhow::anyhow!("OpenAI API error: {} - {}", status, text));
        }
        
        let data: serde_json::Value = serde_json::from_str(&text)?;
        let choice = data["choices"][0].clone();
        let usage = data["usage"].clone();
        
        Ok(LLMResponse {
            content: choice["message"]["content"].as_str().unwrap_or("").to_string(),
            tool_calls: None, // TODO: Parse tool calls
            finish_reason: match choice["finish_reason"].as_str() {
                Some("stop") => FinishReason::Stop,
                Some("length") => FinishReason::Length,
                Some("tool_calls") => FinishReason::ToolCalls,
                _ => FinishReason::Stop,
            },
            usage: Usage {
                prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage["total_tokens"].as_u64().unwrap_or(0) as u32,
            },
        })
    }
}

#[async_trait]
impl LLMInterface for OpenAIProvider {
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse> {
        let max_retries = self.config.max_retries.unwrap_or(3);
        let mut last_error = None;
        
        for attempt in 0..max_retries {
            match self.make_request(&request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries - 1 {
                        sleep(Duration::from_secs(2_u64.pow(attempt))).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }
    
    async fn stream_complete(
        &self,
        request: LLMRequest,
        callback: Box<dyn Fn(String) + Send>,
    ) -> Result<LLMResponse> {
        // TODO: Implement streaming
        self.complete(request).await
    }
    
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let body = json!({
            "model": "text-embedding-3-small",
            "input": texts
        });
        
        let response = self.client
            .post(format!("{}/embeddings", self.base_url))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;
        
        let data: serde_json::Value = response.json().await?;
        let embeddings = data["data"]
            .as_array()
            .context("Invalid embeddings response")?
            .iter()
            .map(|item| {
                item["embedding"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap() as f32)
                    .collect()
            })
            .collect();
        
        Ok(embeddings)
    }
    
    async fn health_check(&self) -> Result<bool> {
        let response = self.client
            .get(format!("{}/models", self.base_url))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.api_key))
            .send()
            .await?;
        
        Ok(response.status().is_success())
    }
}

impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "OpenAI"
    }
    
    fn supports_tools(&self) -> bool {
        true
    }
    
    fn supports_streaming(&self) -> bool {
        true
    }
}

/// Anthropic Provider Implementation
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
    config: LLMConfig,
}

impl AnthropicProvider {
    pub fn new(config: LLMConfig) -> Result<Self> {
        let api_key = config.api_key.clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .context("Anthropic API key not found")?;
        
        let base_url = config.base_url.clone()
            .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());
        
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds.unwrap_or(60)))
            .build()?;
        
        Ok(Self {
            client,
            api_key,
            base_url,
            config,
        })
    }
}

#[async_trait]
impl LLMInterface for AnthropicProvider {
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse> {
        let mut messages = vec![];
        
        for msg in &request.messages {
            messages.push(json!({
                "role": match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "user", // Anthropic doesn't have tool role
                },
                "content": msg.content
            }));
        }
        
        let body = json!({
            "model": request.model.as_str(),
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "temperature": request.temperature.unwrap_or(0.7),
            "system": request.system_prompt.unwrap_or_default(),
        });
        
        let response = self.client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?;
        
        let status = response.status();
        let text = response.text().await?;
        
        if !status.is_success() {
            return Err(anyhow::anyhow!("Anthropic API error: {} - {}", status, text));
        }
        
        let data: serde_json::Value = serde_json::from_str(&text)?;
        
        Ok(LLMResponse {
            content: data["content"][0]["text"].as_str().unwrap_or("").to_string(),
            tool_calls: None,
            finish_reason: match data["stop_reason"].as_str() {
                Some("end_turn") => FinishReason::Stop,
                Some("max_tokens") => FinishReason::Length,
                _ => FinishReason::Stop,
            },
            usage: Usage {
                prompt_tokens: data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: 0, // Calculate if needed
            },
        })
    }
    
    async fn stream_complete(
        &self,
        request: LLMRequest,
        callback: Box<dyn Fn(String) + Send>,
    ) -> Result<LLMResponse> {
        // TODO: Implement streaming
        self.complete(request).await
    }
    
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        Err(anyhow::anyhow!("Anthropic does not support embeddings"))
    }
    
    async fn health_check(&self) -> Result<bool> {
        // Anthropic doesn't have a dedicated health endpoint
        Ok(true)
    }
}

impl LLMProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "Anthropic"
    }
    
    fn supports_tools(&self) -> bool {
        false // As of now, Anthropic doesn't support function calling
    }
    
    fn supports_streaming(&self) -> bool {
        true
    }
}

/// Ollama Provider Implementation (for local models)
pub struct OllamaProvider {
    client: Client,
    base_url: String,
    config: LLMConfig,
}

impl OllamaProvider {
    pub fn new(config: LLMConfig) -> Result<Self> {
        let base_url = config.base_url.clone()
            .unwrap_or_else(|| "http://localhost:11434".to_string());
        
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds.unwrap_or(300))) // Longer timeout for local models
            .build()?;
        
        Ok(Self {
            client,
            base_url,
            config,
        })
    }
}

#[async_trait]
impl LLMInterface for OllamaProvider {
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse> {
        let mut prompt = String::new();
        
        if let Some(system) = &request.system_prompt {
            prompt.push_str(&format!("System: {}\n\n", system));
        }
        
        for msg in &request.messages {
            match msg.role {
                MessageRole::User => prompt.push_str(&format!("User: {}\n", msg.content)),
                MessageRole::Assistant => prompt.push_str(&format!("Assistant: {}\n", msg.content)),
                _ => {}
            }
        }
        
        let body = json!({
            "model": request.model.as_str(),
            "prompt": prompt,
            "temperature": request.temperature.unwrap_or(0.7),
            "stream": false
        });
        
        let response = self.client
            .post(format!("{}/api/generate", self.base_url))
            .json(&body)
            .send()
            .await?;
        
        let data: serde_json::Value = response.json().await?;
        
        Ok(LLMResponse {
            content: data["response"].as_str().unwrap_or("").to_string(),
            tool_calls: None,
            finish_reason: FinishReason::Stop,
            usage: Usage {
                prompt_tokens: 0, // Ollama doesn't provide token counts
                completion_tokens: 0,
                total_tokens: 0,
            },
        })
    }
    
    async fn stream_complete(
        &self,
        request: LLMRequest,
        callback: Box<dyn Fn(String) + Send>,
    ) -> Result<LLMResponse> {
        // TODO: Implement streaming
        self.complete(request).await
    }
    
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = vec![];
        
        for text in texts {
            let body = json!({
                "model": "all-minilm",
                "prompt": text
            });
            
            let response = self.client
                .post(format!("{}/api/embeddings", self.base_url))
                .json(&body)
                .send()
                .await?;
            
            let data: serde_json::Value = response.json().await?;
            let embedding = data["embedding"]
                .as_array()
                .context("Invalid embedding response")?
                .iter()
                .map(|v| v.as_f64().unwrap() as f32)
                .collect();
            
            embeddings.push(embedding);
        }
        
        Ok(embeddings)
    }
    
    async fn health_check(&self) -> Result<bool> {
        let response = self.client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;
        
        Ok(response.status().is_success())
    }
}

impl LLMProvider for OllamaProvider {
    fn name(&self) -> &str {
        "Ollama"
    }
    
    fn supports_tools(&self) -> bool {
        false
    }
    
    fn supports_streaming(&self) -> bool {
        true
    }
}

/// Factory for creating LLM providers
pub struct LLMProviderFactory;

impl LLMProviderFactory {
    pub fn create(config: LLMConfig) -> Result<Box<dyn LLMProvider>> {
        match config.provider.to_lowercase().as_str() {
            "openai" => Ok(Box::new(OpenAIProvider::new(config)?)),
            "anthropic" => Ok(Box::new(AnthropicProvider::new(config)?)),
            "ollama" => Ok(Box::new(OllamaProvider::new(config)?)),
            _ => Err(anyhow::anyhow!("Unknown LLM provider: {}", config.provider)),
        }
    }
}