/// Configuration management for the dbt PR Review Agent
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub agent: AgentSettings,
    pub quality_gates: QualityGates,
    pub notifications: NotificationSettings,
    pub integrations: IntegrationSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSettings {
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub parallel_execution: bool,
    pub fail_fast: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGates {
    pub critical_issues: CriticalIssuesGate,
    pub performance_threshold: PerformanceThreshold,
    pub test_coverage: TestCoverageGate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalIssuesGate {
    pub enabled: bool,
    pub block_merge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThreshold {
    pub cost_increase_percent: f64,
    pub execution_time_increase_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCoverageGate {
    pub minimum_coverage_percent: f64,
    pub require_tests_for_new_models: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub slack: SlackSettings,
    pub email: EmailSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackSettings {
    pub enabled: bool,
    pub webhook_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSettings {
    pub enabled: bool,
    pub recipients: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationSettings {
    pub github: GitHubIntegration,
    pub dbt_cloud: DbtCloudIntegration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIntegration {
    pub auto_comment: bool,
    pub update_pr_status: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbtCloudIntegration {
    pub enabled: bool,
    pub api_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent: AgentSettings {
                timeout_seconds: 300,
                max_retries: 3,
                parallel_execution: true,
                fail_fast: false,
            },
            quality_gates: QualityGates {
                critical_issues: CriticalIssuesGate {
                    enabled: true,
                    block_merge: true,
                },
                performance_threshold: PerformanceThreshold {
                    cost_increase_percent: 25.0,
                    execution_time_increase_percent: 50.0,
                },
                test_coverage: TestCoverageGate {
                    minimum_coverage_percent: 80.0,
                    require_tests_for_new_models: true,
                },
            },
            notifications: NotificationSettings {
                slack: SlackSettings {
                    enabled: false,
                    webhook_url: String::new(),
                },
                email: EmailSettings {
                    enabled: false,
                    recipients: Vec::new(),
                },
            },
            integrations: IntegrationSettings {
                github: GitHubIntegration {
                    auto_comment: true,
                    update_pr_status: true,
                },
                dbt_cloud: DbtCloudIntegration {
                    enabled: false,
                    api_key: String::new(),
                },
            },
        }
    }
}

impl Config {
    /// Load configuration from file
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Load configuration from environment variables
    pub fn load_from_env() -> Result<Self> {
        let mut config = Config::default();

        // Override with environment variables if present
        if let Ok(timeout) = std::env::var("DBT_PR_AGENT_TIMEOUT_SECONDS") {
            config.agent.timeout_seconds = timeout.parse()?;
        }

        if let Ok(retries) = std::env::var("DBT_PR_AGENT_MAX_RETRIES") {
            config.agent.max_retries = retries.parse()?;
        }

        if let Ok(parallel) = std::env::var("DBT_PR_AGENT_PARALLEL_EXECUTION") {
            config.agent.parallel_execution = parallel.parse()?;
        }

        if let Ok(webhook_url) = std::env::var("SLACK_WEBHOOK_URL") {
            config.notifications.slack.webhook_url = webhook_url;
            config.notifications.slack.enabled = true;
        }

        Ok(config)
    }

    /// Merge with another configuration (other takes precedence)
    pub fn merge_with(&mut self, other: Config) {
        // Merge agent settings
        if other.agent.timeout_seconds != 300 {
            self.agent.timeout_seconds = other.agent.timeout_seconds;
        }
        if other.agent.max_retries != 3 {
            self.agent.max_retries = other.agent.max_retries;
        }
        self.agent.parallel_execution = other.agent.parallel_execution;
        self.agent.fail_fast = other.agent.fail_fast;

        // Merge quality gates
        self.quality_gates.critical_issues.enabled = other.quality_gates.critical_issues.enabled;
        self.quality_gates.critical_issues.block_merge = other.quality_gates.critical_issues.block_merge;
        
        if other.quality_gates.performance_threshold.cost_increase_percent != 25.0 {
            self.quality_gates.performance_threshold.cost_increase_percent = 
                other.quality_gates.performance_threshold.cost_increase_percent;
        }
        
        if other.quality_gates.performance_threshold.execution_time_increase_percent != 50.0 {
            self.quality_gates.performance_threshold.execution_time_increase_percent = 
                other.quality_gates.performance_threshold.execution_time_increase_percent;
        }

        // Merge notifications
        if other.notifications.slack.enabled {
            self.notifications.slack = other.notifications.slack;
        }
        if other.notifications.email.enabled {
            self.notifications.email = other.notifications.email;
        }

        // Merge integrations
        self.integrations.github = other.integrations.github;
        if other.integrations.dbt_cloud.enabled {
            self.integrations.dbt_cloud = other.integrations.dbt_cloud;
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.agent.timeout_seconds == 0 {
            return Err(anyhow::anyhow!("Agent timeout must be greater than 0"));
        }

        if self.quality_gates.performance_threshold.cost_increase_percent < 0.0 {
            return Err(anyhow::anyhow!("Cost increase threshold must be non-negative"));
        }

        if self.quality_gates.test_coverage.minimum_coverage_percent < 0.0 
            || self.quality_gates.test_coverage.minimum_coverage_percent > 100.0 {
            return Err(anyhow::anyhow!("Test coverage percentage must be between 0 and 100"));
        }

        if self.notifications.slack.enabled && self.notifications.slack.webhook_url.is_empty() {
            return Err(anyhow::anyhow!("Slack webhook URL is required when Slack notifications are enabled"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_config_save_and_load() {
        let config = Config::default();
        let temp_file = NamedTempFile::new().unwrap();
        
        // Save config
        config.save_to_file(temp_file.path()).await.unwrap();
        
        // Load config
        let loaded_config = Config::load_from_file(temp_file.path()).await.unwrap();
        
        assert_eq!(config.agent.timeout_seconds, loaded_config.agent.timeout_seconds);
        assert_eq!(config.agent.max_retries, loaded_config.agent.max_retries);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());
        
        // Test invalid timeout
        config.agent.timeout_seconds = 0;
        assert!(config.validate().is_err());
        
        // Reset and test invalid cost threshold
        config = Config::default();
        config.quality_gates.performance_threshold.cost_increase_percent = -1.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_merge() {
        let mut base_config = Config::default();
        let mut override_config = Config::default();
        
        override_config.agent.timeout_seconds = 600;
        override_config.notifications.slack.enabled = true;
        override_config.notifications.slack.webhook_url = "https://hooks.slack.com/test".to_string();
        
        base_config.merge_with(override_config);
        
        assert_eq!(base_config.agent.timeout_seconds, 600);
        assert!(base_config.notifications.slack.enabled);
        assert_eq!(base_config.notifications.slack.webhook_url, "https://hooks.slack.com/test");
    }
}