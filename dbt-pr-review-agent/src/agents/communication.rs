use crate::types::AgentEvent;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info};

/// Communication bus for inter-agent coordination and event handling
pub struct AgentCommunicationBus {
    /// Broadcast channel for real-time events
    event_sender: broadcast::Sender<AgentEvent>,
    
    /// Event history storage
    event_history: Arc<RwLock<Vec<AgentEvent>>>,
    
    /// Event subscribers by event type
    subscribers: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl AgentCommunicationBus {
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        
        Self {
            event_sender,
            event_history: Arc::new(RwLock::new(Vec::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Publish an event to all subscribers
    pub async fn publish(&self, event: AgentEvent) -> Result<()> {
        debug!(
            "Publishing event: {} from agent: {}",
            event.event_type, event.agent_name
        );

        // Store in history
        {
            let mut history = self.event_history.write().await;
            history.push(event.clone());
            
            // Keep only last 10000 events to prevent memory growth
            if history.len() > 10000 {
                history.drain(0..1000);
            }
        }

        // Broadcast to subscribers
        match self.event_sender.send(event) {
            Ok(subscriber_count) => {
                debug!("Event broadcast to {} subscribers", subscriber_count);
            }
            Err(_) => {
                debug!("No active subscribers for event");
            }
        }

        Ok(())
    }

    /// Subscribe to events and get a receiver
    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.event_sender.subscribe()
    }

    /// Register a subscriber for specific event types
    pub async fn register_subscriber(&self, agent_name: String, event_types: Vec<String>) {
        let mut subscribers = self.subscribers.write().await;
        
        for event_type in event_types {
            subscribers
                .entry(event_type.clone())
                .or_insert_with(Vec::new)
                .push(agent_name.clone());
        }

        info!("Registered subscriber: {}", agent_name);
    }

    /// Get event history filtered by criteria
    pub async fn get_event_history(
        &self, 
        agent_name: Option<&str>,
        event_type: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<AgentEvent> {
        let history = self.event_history.read().await;
        
        let filtered: Vec<AgentEvent> = history
            .iter()
            .filter(|event| {
                if let Some(name) = agent_name {
                    if event.agent_name != name {
                        return false;
                    }
                }
                
                if let Some(event_t) = event_type {
                    if event.event_type != event_t {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();

        if let Some(limit) = limit {
            filtered.into_iter().rev().take(limit).collect()
        } else {
            filtered
        }
    }

    /// Get communication statistics
    pub async fn get_statistics(&self) -> CommunicationStatistics {
        let history = self.event_history.read().await;
        let subscribers = self.subscribers.read().await;

        let total_events = history.len();
        let total_subscribers = subscribers.values().map(|v| v.len()).sum::<usize>();

        // Count events by agent
        let mut events_by_agent: HashMap<String, usize> = HashMap::new();
        for event in history.iter() {
            *events_by_agent.entry(event.agent_name.clone()).or_insert(0) += 1;
        }

        // Count events by type
        let mut events_by_type: HashMap<String, usize> = HashMap::new();
        for event in history.iter() {
            *events_by_type.entry(event.event_type.clone()).or_insert(0) += 1;
        }

        CommunicationStatistics {
            total_events,
            total_subscribers,
            events_by_agent,
            events_by_type,
            active_subscribers: subscribers.len(),
        }
    }

    /// Clear event history (useful for testing or memory management)
    pub async fn clear_history(&self) {
        let mut history = self.event_history.write().await;
        history.clear();
        info!("Event history cleared");
    }

    /// Check if there are any active subscribers
    pub fn has_subscribers(&self) -> bool {
        self.event_sender.receiver_count() > 0
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.event_sender.receiver_count()
    }
}

#[derive(Debug, Clone)]
pub struct CommunicationStatistics {
    pub total_events: usize,
    pub total_subscribers: usize,
    pub events_by_agent: HashMap<String, usize>,
    pub events_by_type: HashMap<String, usize>,
    pub active_subscribers: usize,
}

/// Helper trait for agents to interact with the communication bus
#[async_trait::async_trait]
pub trait AgentCommunication {
    /// Get agent name
    fn agent_name(&self) -> &str;

    /// Publish an event
    async fn publish_event(
        &self,
        communication_bus: &AgentCommunicationBus,
        event_type: String,
        data: serde_json::Value,
    ) -> Result<()> {
        let event = AgentEvent {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            agent_name: self.agent_name().to_string(),
            event_type,
            data,
        };

        communication_bus.publish(event).await
    }

    /// Handle incoming events (to be implemented by each agent)
    async fn handle_event(&self, event: &AgentEvent) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_communication_bus_publish_subscribe() {
        let bus = AgentCommunicationBus::new();
        let mut receiver = bus.subscribe();

        let event = AgentEvent {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            agent_name: "test_agent".to_string(),
            event_type: "test_event".to_string(),
            data: json!({"message": "test"}),
        };

        // Publish event
        bus.publish(event.clone()).await.unwrap();

        // Receive event
        let received_event = receiver.recv().await.unwrap();
        assert_eq!(received_event.agent_name, "test_agent");
        assert_eq!(received_event.event_type, "test_event");
    }

    #[tokio::test]
    async fn test_event_history() {
        let bus = AgentCommunicationBus::new();

        let event1 = AgentEvent {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            agent_name: "agent1".to_string(),
            event_type: "type1".to_string(),
            data: json!({}),
        };

        let event2 = AgentEvent {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            agent_name: "agent2".to_string(),
            event_type: "type2".to_string(),
            data: json!({}),
        };

        bus.publish(event1).await.unwrap();
        bus.publish(event2).await.unwrap();

        let history = bus.get_event_history(None, None, None).await;
        assert_eq!(history.len(), 2);

        let agent1_history = bus.get_event_history(Some("agent1"), None, None).await;
        assert_eq!(agent1_history.len(), 1);
        assert_eq!(agent1_history[0].agent_name, "agent1");

        let type1_history = bus.get_event_history(None, Some("type1"), None).await;
        assert_eq!(type1_history.len(), 1);
        assert_eq!(type1_history[0].event_type, "type1");
    }

    #[tokio::test]
    async fn test_subscriber_registration() {
        let bus = AgentCommunicationBus::new();

        bus.register_subscriber(
            "test_agent".to_string(),
            vec!["event1".to_string(), "event2".to_string()],
        ).await;

        let stats = bus.get_statistics().await;
        assert_eq!(stats.active_subscribers, 2); // Two event types registered
    }

    #[tokio::test]
    async fn test_communication_statistics() {
        let bus = AgentCommunicationBus::new();

        let event1 = AgentEvent {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            agent_name: "agent1".to_string(),
            event_type: "type1".to_string(),
            data: json!({}),
        };

        let event2 = AgentEvent {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            agent_name: "agent1".to_string(),
            event_type: "type2".to_string(),
            data: json!({}),
        };

        bus.publish(event1).await.unwrap();
        bus.publish(event2).await.unwrap();

        let stats = bus.get_statistics().await;
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.events_by_agent.get("agent1"), Some(&2));
        assert_eq!(stats.events_by_type.get("type1"), Some(&1));
        assert_eq!(stats.events_by_type.get("type2"), Some(&1));
    }

    #[tokio::test]
    async fn test_history_limit() {
        let bus = AgentCommunicationBus::new();

        // Publish multiple events
        for i in 0..5 {
            let event = AgentEvent {
                id: uuid::Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
                agent_name: format!("agent{}", i),
                event_type: "test".to_string(),
                data: json!({"index": i}),
            };
            bus.publish(event).await.unwrap();
        }

        let limited_history = bus.get_event_history(None, None, Some(3)).await;
        assert_eq!(limited_history.len(), 3);

        let full_history = bus.get_event_history(None, None, None).await;
        assert_eq!(full_history.len(), 5);
    }
}