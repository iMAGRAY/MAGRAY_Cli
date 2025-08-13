#![allow(clippy::uninlined_format_args)]
#![allow(clippy::useless_vec)]
#![allow(clippy::new_without_default)]
//! Simple Agent Example
//!
//! This example shows how to implement a simple agent using the Actor System.
//! It demonstrates the basic actor lifecycle, message handling, and resource management.

use async_trait::async_trait;
use orchestrator::actors::ActorContext;
use orchestrator::{
    ActorId, ActorMessage, ActorResult, ActorSystem, BaseActor, ResourceBudget, SystemConfig,
};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

/// Example IntentAnalyzer Agent
pub struct IntentAnalyzer {
    id: ActorId,
    processed_intents: u32,
}

impl IntentAnalyzer {
    pub fn new() -> Self {
        Self {
            id: ActorId::new(),
            processed_intents: 0,
        }
    }
}

#[async_trait]
impl BaseActor for IntentAnalyzer {
    fn id(&self) -> ActorId {
        self.id
    }

    fn actor_type(&self) -> &'static str {
        "intent_analyzer"
    }

    async fn initialize(&mut self, context: &ActorContext) -> ActorResult<()> {
        println!("🤖 IntentAnalyzer {} initializing...", self.id);

        // Subscribe to intent analysis events
        let topics = &["user.intent", "system.analyze"];
        context.subscribe_to_events(topics).await?;

        println!("✅ IntentAnalyzer {} initialized successfully", self.id);
        Ok(())
    }

    async fn handle_message(
        &mut self,
        message: ActorMessage,
        context: &ActorContext,
    ) -> ActorResult<()> {
        match message {
            ActorMessage::Custom {
                message_type,
                payload,
            } if message_type == "analyze_intent" => {
                self.processed_intents += 1;

                let user_input = payload
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                println!("🔍 Analyzing intent for: '{}'", user_input);

                // Simulate some processing time
                sleep(Duration::from_millis(100)).await;

                // Classify intent (simplified)
                let intent = match user_input.to_lowercase() {
                    s if s.contains("create") || s.contains("generate") => "create",
                    s if s.contains("read") || s.contains("show") || s.contains("list") => "read",
                    s if s.contains("update") || s.contains("modify") || s.contains("change") => {
                        "update"
                    }
                    s if s.contains("delete") || s.contains("remove") => "delete",
                    s if s.contains("help") || s.contains("?") => "help",
                    _ => "unknown",
                };

                let confidence = if intent == "unknown" { 0.3 } else { 0.85 };

                // Publish analysis result
                let result = json!({
                    "intent": intent,
                    "confidence": confidence,
                    "original_text": user_input,
                    "processed_by": self.id.to_string(),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                context.publish_event("intent.analyzed", result).await?;

                println!(
                    "✨ Intent classified as '{}' with confidence {:.1}%",
                    intent,
                    confidence * 100.0
                );
            }

            ActorMessage::Ping => {
                println!(
                    "🏓 IntentAnalyzer {} responding to ping (processed {} intents)",
                    self.id, self.processed_intents
                );
            }

            ActorMessage::System { command } => {
                println!(
                    "⚙️ IntentAnalyzer {} received system command: {:?}",
                    self.id, command
                );
            }

            _ => {
                println!(
                    "❓ IntentAnalyzer {} received unknown message: {:?}",
                    self.id, message
                );
            }
        }

        Ok(())
    }

    async fn shutdown(&mut self, _context: &ActorContext) -> ActorResult<()> {
        println!(
            "🛑 IntentAnalyzer {} shutting down. Processed {} intents total.",
            self.id, self.processed_intents
        );
        Ok(())
    }

    fn resource_budget(&self) -> ResourceBudget {
        ResourceBudget::new()
            .cpu_time_limit(Duration::from_secs(60))
            .memory_limit(128 * 1024 * 1024) // 128MB
            .queue_limit(1000)
            .timeout_limit(Duration::from_secs(10))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 Starting Actor System Example");

    // Create system configuration
    let config = SystemConfig::default();

    // Create and start actor system
    let system = ActorSystem::new(config).await?;
    println!("✅ Actor System initialized");

    // Create and spawn IntentAnalyzer agent
    let intent_analyzer = IntentAnalyzer::new();
    let _agent_id = intent_analyzer.id();

    let actor_id = system
        .spawn_actor(
            Box::new(intent_analyzer),
            None, // Use default resource budget
            None, // Use default metadata
        )
        .await?;

    println!("🤖 Spawned IntentAnalyzer agent with ID: {}", actor_id);

    // Wait for system to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Get actor handle and send some test messages
    if let Some(handle) = system.get_actor(actor_id) {
        println!("📤 Sending test messages...");

        // Test intent analysis messages
        let test_intents = vec![
            "create a new project",
            "show me the current status",
            "update the configuration",
            "delete old files",
            "help me with this task",
            "something completely random",
        ];

        for (i, intent_text) in test_intents.iter().enumerate() {
            let message = ActorMessage::Custom {
                message_type: "analyze_intent".to_string(),
                payload: json!({
                    "text": intent_text,
                    "request_id": i
                }),
            };

            handle.send(message).await?;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Send a ping
        handle.send(ActorMessage::Ping).await?;
    }

    // Let the system run for a bit
    println!("⏳ Running system for 3 seconds...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check system health
    let health = system.get_system_health().await;
    println!("📊 System Health:");
    println!("  - Total actors: {}", health.total_actors);
    println!("  - Healthy actors: {}", health.healthy_actors);
    println!(
        "  - System memory: {:.1} MB used / {:.1} MB total",
        health.system_status.used_memory as f64 / (1024.0 * 1024.0),
        health.system_status.total_memory as f64 / (1024.0 * 1024.0)
    );
    println!(
        "  - Memory pressure: {:.1}%",
        health.system_status.memory_pressure * 100.0
    );

    // Graceful shutdown
    println!("🛑 Shutting down Actor System...");
    system.shutdown().await?;

    println!("✅ Example completed successfully!");
    Ok(())
}
