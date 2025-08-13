#![allow(unused_variables)]
//! Multi-Agent Orchestration Demo
//!
//! This example demonstrates the Actor Model implementation for multi-agent communication.
//! It shows how to create agents, send typed messages, and coordinate between different agent types.

use async_trait::async_trait;
use orchestrator::{
    actors::ActorContext, ActorError, ActorId, ActorMessage as BaseActorMessage,
    ActorSystemManager, AgentCommunicationConfig, AgentMessage, AgentType, BaseActor, SystemConfig,
};
use serde_json::json;
use tokio::time::Duration;
use tracing::{info, warn};

/// Simple demo agent that can handle agent messages
#[derive(Debug)]
struct DemoAgent {
    id: ActorId,
    agent_type: &'static str,
}

impl DemoAgent {
    fn new(agent_type: &'static str) -> Self {
        Self {
            id: ActorId::new(),
            agent_type,
        }
    }
}

#[async_trait]
impl BaseActor for DemoAgent {
    fn id(&self) -> ActorId {
        self.id
    }

    fn actor_type(&self) -> &'static str {
        self.agent_type
    }

    async fn handle_message(
        &mut self,
        message: BaseActorMessage,
        _context: &ActorContext,
    ) -> Result<(), ActorError> {
        match message {
            BaseActorMessage::Agent(agent_msg) => self.handle_agent_message(agent_msg).await,
            BaseActorMessage::Start => {
                info!(agent_type = %self.agent_type, actor_id = %self.id, "Agent started");
                Ok(())
            }
            BaseActorMessage::Stop => {
                info!(agent_type = %self.agent_type, actor_id = %self.id, "Agent stopping");
                Ok(())
            }
            BaseActorMessage::Ping => {
                info!(agent_type = %self.agent_type, actor_id = %self.id, "Pong!");
                Ok(())
            }
            _ => {
                warn!(agent_type = %self.agent_type, actor_id = %self.id, "Unhandled message: {:?}", message);
                Ok(())
            }
        }
    }
}

impl DemoAgent {
    async fn handle_agent_message(&mut self, message: AgentMessage) -> Result<(), ActorError> {
        match message {
            AgentMessage::AnalyzeIntent {
                user_input,
                context,
            } => {
                info!(
                    agent_type = %self.agent_type,
                    actor_id = %self.id,
                    "Analyzing intent for: '{}'",
                    user_input
                );

                // Simulate intent analysis
                let intent = json!({
                    "action": "create_user",
                    "confidence": 0.95,
                    "parameters": {
                        "entity_type": "user_account"
                    }
                });

                info!(
                    agent_type = %self.agent_type,
                    actor_id = %self.id,
                    "Intent analysis complete: {:?}",
                    intent
                );
                Ok(())
            }

            AgentMessage::CreatePlan {
                intent,
                constraints,
            } => {
                info!(
                    agent_type = %self.agent_type,
                    actor_id = %self.id,
                    "Creating plan for intent: {:?}",
                    intent
                );

                // Simulate plan creation
                let plan = json!({
                    "steps": [
                        {"step": 1, "action": "validate_input", "description": "Validate user input"},
                        {"step": 2, "action": "create_user_record", "description": "Create user in database"},
                        {"step": 3, "action": "send_confirmation", "description": "Send confirmation email"}
                    ],
                    "estimated_time": 30,
                    "resource_requirements": {
                        "database": true,
                        "email_service": true
                    }
                });

                info!(
                    agent_type = %self.agent_type,
                    actor_id = %self.id,
                    "Plan created: {:?}",
                    plan
                );
                Ok(())
            }

            AgentMessage::ExecutePlan { plan, dry_run } => {
                info!(
                    agent_type = %self.agent_type,
                    actor_id = %self.id,
                    "Executing plan (dry_run: {}): {:?}",
                    dry_run,
                    plan
                );

                // Simulate plan execution
                if !dry_run {
                    info!(
                        agent_type = %self.agent_type,
                        actor_id = %self.id,
                        "Plan execution completed successfully"
                    );
                } else {
                    info!(
                        agent_type = %self.agent_type,
                        actor_id = %self.id,
                        "Dry run completed - no actual changes made"
                    );
                }
                Ok(())
            }

            AgentMessage::CritiqueResult { result, context } => {
                info!(
                    agent_type = %self.agent_type,
                    actor_id = %self.id,
                    "Critiquing result: {:?}",
                    result
                );

                // Simulate critique
                info!(
                    agent_type = %self.agent_type,
                    actor_id = %self.id,
                    "Critique: Result looks good, quality score: 0.92"
                );
                Ok(())
            }

            AgentMessage::Request {
                request_id,
                request_type,
                payload,
            } => {
                info!(
                    agent_type = %self.agent_type,
                    actor_id = %self.id,
                    "Received request {}: {} with payload: {:?}",
                    request_id,
                    request_type,
                    payload
                );
                Ok(())
            }

            _ => {
                info!(
                    agent_type = %self.agent_type,
                    actor_id = %self.id,
                    "Received agent message: {:?}",
                    message
                );
                Ok(())
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Multi-Agent Orchestration Demo");

    // Create system and communication configuration
    let system_config = SystemConfig::default();
    let comm_config = AgentCommunicationConfig::default();

    // Create actor system manager
    let manager = ActorSystemManager::new(system_config, comm_config).await?;

    info!("Actor System Manager initialized");

    // Spawn different types of agents
    let intent_analyzer = DemoAgent::new("intent_analyzer");
    let planner = DemoAgent::new("planner");
    let executor = DemoAgent::new("executor");
    let critic = DemoAgent::new("critic");

    let intent_analyzer_id = manager
        .spawn_agent(AgentType::IntentAnalyzer, Box::new(intent_analyzer))
        .await?;

    let planner_id = manager
        .spawn_agent(AgentType::Planner, Box::new(planner))
        .await?;

    let executor_id = manager
        .spawn_agent(AgentType::Executor, Box::new(executor))
        .await?;

    let critic_id = manager
        .spawn_agent(AgentType::Critic, Box::new(critic))
        .await?;

    info!("All agents spawned successfully");

    // Give agents a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Demonstrate multi-agent workflow: Intent → Plan → Execute → Critique
    info!("=== Starting Multi-Agent Workflow Demo ===");

    // Step 1: Send intent analysis request
    info!("Step 1: Intent Analysis");
    let intent_message = AgentMessage::AnalyzeIntent {
        user_input: "Create a new user account with email john@example.com".to_string(),
        context: Some(json!({"user_role": "admin", "session_id": "abc123"})),
    };

    manager
        .send_to_agent_type(AgentType::IntentAnalyzer, intent_message)
        .await?;

    // Step 2: Send plan creation request
    tokio::time::sleep(Duration::from_millis(50)).await;
    info!("Step 2: Plan Creation");
    let plan_message = AgentMessage::CreatePlan {
        intent: json!({
            "action": "create_user",
            "email": "john@example.com",
            "confidence": 0.95
        }),
        constraints: Some(json!({
            "max_time": 60,
            "required_approvals": ["security", "compliance"]
        })),
    };

    manager
        .send_to_agent_type(AgentType::Planner, plan_message)
        .await?;

    // Step 3: Send execution request
    tokio::time::sleep(Duration::from_millis(50)).await;
    info!("Step 3: Plan Execution");
    let execute_message = AgentMessage::ExecutePlan {
        plan: json!({
            "plan_id": "plan_123",
            "steps": [
                {"action": "validate_email", "params": {"email": "john@example.com"}},
                {"action": "create_user_record", "params": {"email": "john@example.com"}},
                {"action": "send_welcome_email", "params": {"email": "john@example.com"}}
            ]
        }),
        dry_run: true, // Safe demo mode
    };

    manager
        .send_to_agent_type(AgentType::Executor, execute_message)
        .await?;

    // Step 4: Send critique request
    tokio::time::sleep(Duration::from_millis(50)).await;
    info!("Step 4: Result Critique");
    let critique_message = AgentMessage::CritiqueResult {
        result: json!({
            "success": true,
            "user_created": true,
            "user_id": "user_456",
            "execution_time": 1.2
        }),
        context: Some(json!({"execution_mode": "dry_run"})),
    };

    manager
        .send_to_agent_type(AgentType::Critic, critique_message)
        .await?;

    // Wait for message processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Demonstrate request/response pattern
    info!("=== Demonstrating Request/Response Pattern ===");
    let request_id = manager
        .send_request(
            intent_analyzer_id,
            AgentType::Planner,
            AgentMessage::Request {
                request_id: "req_789".to_string(),
                request_type: "status_check".to_string(),
                payload: json!({"check_type": "health"}),
            },
            Some(5000), // 5 second timeout
        )
        .await?;

    info!("Sent request with ID: {}", request_id);

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Get system statistics
    let stats = manager.get_agent_stats().await;
    info!("=== System Statistics ===");
    info!("Total agents: {}", stats.get_total_agents());
    info!("Pending requests: {}", stats.pending_requests);

    for agent_type in [
        AgentType::IntentAnalyzer,
        AgentType::Planner,
        AgentType::Executor,
        AgentType::Critic,
    ] {
        let count = stats.get_agent_count(&agent_type);
        info!("{}: {} agents", agent_type, count);
    }

    // Demonstrate broadcasting
    info!("=== Broadcasting Demo ===");
    let ping_message = AgentMessage::Request {
        request_id: "broadcast_ping".to_string(),
        request_type: "ping_all".to_string(),
        payload: json!({"message": "Hello from broadcast!"}),
    };

    let sent_count = manager
        .broadcast_to_agent_type(AgentType::IntentAnalyzer, ping_message.clone())
        .await?;

    info!("Broadcast sent to {} IntentAnalyzer agents", sent_count);

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    info!("=== Demo Complete ===");
    info!("Shutting down Actor System Manager...");

    // Shutdown the system
    manager.shutdown().await?;

    info!("Multi-Agent Orchestration Demo completed successfully!");

    Ok(())
}
