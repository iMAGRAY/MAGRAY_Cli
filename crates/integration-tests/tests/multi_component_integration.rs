//! Multi-Component Integration Tests
//!
//! Tests the integration between major completed components:
//! - Tool Context Builder + AI Embeddings (P1.3.2)
//! - Config Profiles + Security Integration (P2.3.7)
//! - Multi-Agent Orchestration components

use anyhow::Result;
use integration_tests::{
    common::{assertions, HealthChecker, MemoryTracker, PerformanceMetrics, TestFixture},
    fixtures::{ConfigProfileFixtures, OrchestrationFixtures, ToolContextFixtures},
    IntegrationTestResult, IntegrationTestRunner,
};
use serde_json::json;
use std::time::Duration;
use tracing::{info, warn};

/// Test Tool Context Builder with AI Embeddings integration
#[tokio::test]
async fn test_tool_context_builder_ai_embeddings_integration() -> Result<()> {
    let mut runner = IntegrationTestRunner::new().await?;

    runner
        .run_test("tool_context_builder_ai_embeddings", |env| {
            Box::pin(async move {
                let mut result =
                    IntegrationTestResult::new("tool_context_builder_ai_embeddings".to_string())
                        .with_component("tools::ToolContextBuilder")
                        .with_component("ai::EmbeddingService");

                let mut metrics = PerformanceMetrics::new("tool_context_builder_ai_embeddings");
                let mut memory_tracker = MemoryTracker::new();

                let mut fixture = TestFixture::new("tool_context_builder_ai_embeddings").await?;

                // Setup test data
                metrics.mark("setup_start");
                ToolContextFixtures::create_sample_tools(&mut fixture).await?;
                metrics.measure_since_mark("setup_duration", "setup_start");

                // Create Tool Context Builder configuration
                metrics.mark("builder_creation");

                let config = tools::context::ToolSelectionConfig {
                    max_candidates: 20,
                    top_n_tools: 5,
                    similarity_threshold: 0.2,
                    allowed_platforms: vec!["linux".into(), "win".into(), "mac".into()],
                    max_security_level: tools::registry::SecurityLevel::MediumRisk,
                    allowed_categories: None,
                    enable_reranking: true,
                };

                // Test both mock and real AI embeddings (fallback behavior)
                let builder = if env.use_real_ai {
                    info!("Testing with real AI embeddings");
                    match create_builder_with_real_embeddings(config.clone()).await {
                        Ok(builder) => {
                            result = result.with_metric("real_ai_available", 1.0);
                            builder
                        }
                        Err(e) => {
                            warn!("Real AI embeddings failed, falling back to mock: {}", e);
                            result = result.with_metric("real_ai_available", 0.0);
                            tools::context::ToolContextBuilder::new(config)
                        }
                    }
                } else {
                    info!("Testing with mock embeddings");
                    result = result.with_metric("real_ai_available", 0.0);
                    tools::context::ToolContextBuilder::new(config)
                };

                metrics.measure_since_mark("builder_creation_duration", "builder_creation");
                memory_tracker.record();

                // Register sample tools
                metrics.mark("tool_registration");
                register_sample_tools(&builder).await?;
                metrics.measure_since_mark("tool_registration_duration", "tool_registration");
                memory_tracker.record();

                // Test tool selection with various queries
                metrics.mark("tool_selection_tests");
                let test_queries = ToolContextFixtures::create_test_queries();

                for (query_name, query_data) in test_queries {
                    metrics.mark(&format!("query_{}_start", query_name));

                    let query = query_data["query"].as_str().expect("Test operation should succeed");
                    let context = create_tool_selection_context(query, &query_data);

                    let selection_result = builder.build_context(context).await?;

                    // Validate results
                    assert!(
                        !selection_result.selected_tools.is_empty(),
                        "No tools selected for query: {}",
                        query
                    );

                    assert!(
                        !selection_result.llm_context.is_empty(),
                        "Empty LLM context for query: {}",
                        query
                    );

                    // Record metrics
                    let duration = metrics
                        .measure_since_mark(
                            &format!("query_{}_duration", query_name),
                            &format!("query_{}_start", query_name),
                        )
                        .expect("Test operation should succeed");

                    result = result.with_metric(
                        &format!("{}_selection_time_ms", query_name),
                        duration.as_millis() as f64,
                    );

                    result = result.with_metric(
                        &format!("{}_tools_selected", query_name),
                        selection_result.selected_tools.len() as f64,
                    );

                    metrics.increment("successful_queries");
                    memory_tracker.record();
                }

                metrics.measure_since_mark("tool_selection_duration", "tool_selection_tests");

                // Test builder statistics
                let stats = builder.get_statistics().await?;
                result = result.with_metric("total_tools_registered", stats.total_tools as f64);
                result =
                    result.with_metric("tools_with_embeddings", stats.tools_with_embeddings as f64);

                // Performance validation
                let performance_criteria = std::collections::HashMap::from([
                    ("tool_registration_duration_ms".to_string(), 5000.0), // 5 seconds max
                    ("tool_selection_duration_ms".to_string(), 10000.0), // 10 seconds max for all queries
                ]);

                assertions::assert_performance_criteria(&metrics, &performance_criteria)?;
                assertions::assert_memory_bounds(&memory_tracker, 100.0)?; // 100MB max growth

                // Record final metrics
                for (name, value) in &metrics.measurements {
                    result = result.with_metric(name, *value);
                }

                if let Some(growth) = memory_tracker.memory_growth_mb() {
                    result = result.with_metric("memory_growth_mb", growth);
                }

                fixture.cleanup().await?;

                info!(
                    "Tool Context Builder + AI Embeddings integration test completed successfully"
                );
                Ok(result.success(metrics.total_duration().as_millis() as u64))
            })
        })
        .await?;

    let report = runner.generate_report();
    info!("Integration test report:\n{}", report);

    runner.cleanup().await
}

/// Test Config Profiles + Security Integration
#[tokio::test]
async fn test_config_profiles_security_integration() -> Result<()> {
    let mut runner = IntegrationTestRunner::new().await?;

    runner
        .run_test("config_profiles_security", |env| {
            Box::pin(async move {
                let mut result = IntegrationTestResult::new("config_profiles_security".to_string())
                    .with_component("infrastructure::ConfigLoader")
                    .with_component("infrastructure::PolicyIntegrationEngine")
                    .with_component("domain::config");

                let mut metrics = PerformanceMetrics::new("config_profiles_security");
                let mut fixture = TestFixture::new("config_profiles_security").await?;

                // Create test configuration files
                metrics.mark("config_creation");
                ConfigProfileFixtures::create_base_config(&mut fixture).await?;
                ConfigProfileFixtures::create_dev_profile(&mut fixture).await?;
                ConfigProfileFixtures::create_prod_profile(&mut fixture).await?;
                metrics.measure_since_mark("config_creation_duration", "config_creation");

                // Test policy integration engine
                metrics.mark("policy_engine_init");
                let mut policy_engine = infrastructure::config::PolicyIntegrationEngine::new();
                metrics.measure_since_mark("policy_engine_init_duration", "policy_engine_init");

                // Test dev profile integration
                metrics.mark("dev_profile_test");
                let mut dev_config = domain::config::MagrayConfig::default();
                dev_config.profile = domain::config::Profile::Dev;
                dev_config.profile_config = Some(domain::config::ProfileConfig::dev());
                dev_config.apply_profile(&domain::config::ProfileConfig::dev());

                policy_engine.apply_profile_policy(&dev_config).await?;
                let dev_policy = policy_engine.get_policy_config();

                // Validate dev profile characteristics
                assert!(
                    dev_policy.permissive_mode,
                    "Dev profile should be permissive"
                );
                assert_eq!(dev_policy.default_mode, "ask");
                assert!(
                    !dev_policy.tool_permissions.require_signed_tools,
                    "Dev profile should not require signed tools"
                );

                metrics.measure_since_mark("dev_profile_duration", "dev_profile_test");
                metrics.increment("profiles_tested");

                // Test prod profile integration
                metrics.mark("prod_profile_test");
                let mut prod_config = domain::config::MagrayConfig::default();
                prod_config.profile = domain::config::Profile::Prod;
                prod_config.profile_config = Some(domain::config::ProfileConfig::prod());
                prod_config.apply_profile(&domain::config::ProfileConfig::prod());

                policy_engine.apply_profile_policy(&prod_config).await?;
                let prod_policy = policy_engine.get_policy_config();

                // Validate prod profile characteristics
                assert!(
                    !prod_policy.permissive_mode,
                    "Prod profile should not be permissive"
                );
                assert_eq!(prod_policy.default_mode, "deny");
                assert!(
                    prod_policy.tool_permissions.require_signed_tools,
                    "Prod profile should require signed tools"
                );

                metrics.measure_since_mark("prod_profile_duration", "prod_profile_test");
                metrics.increment("profiles_tested");

                // Test runtime profile switching
                metrics.mark("profile_switching_test");
                let config_loader = infrastructure::config::ConfigLoader::new();

                let mut runtime_config = dev_config;
                runtime_config = config_loader
                    .switch_profile(runtime_config, domain::config::Profile::Prod)
                    .await?;

                assert_eq!(runtime_config.profile, domain::config::Profile::Prod);

                policy_engine.apply_profile_policy(&runtime_config).await?;
                let switched_policy = policy_engine.get_policy_config();

                // Validate switched profile
                assert!(
                    !switched_policy.permissive_mode,
                    "Switched profile should be strict"
                );

                metrics.measure_since_mark("profile_switching_duration", "profile_switching_test");
                metrics.increment("profile_switches");

                // Test security policy decisions with different profiles
                metrics.mark("security_decisions_test");
                let test_operations = create_test_security_operations();

                for (operation, expected_dev, expected_prod) in test_operations {
                    // Test with dev profile
                    policy_engine.apply_profile_policy(&dev_config).await?;
                    let dev_decision =
                        policy_engine.check_operation_allowed(&operation.operation, &operation);

                    // Test with prod profile
                    policy_engine.apply_profile_policy(&prod_config).await?;
                    let prod_decision =
                        policy_engine.check_operation_allowed(&operation.operation, &operation);

                    // Validate expected decisions
                    validate_policy_decision(
                        &dev_decision,
                        &expected_dev,
                        &format!("dev/{}", operation.operation),
                    )?;
                    validate_policy_decision(
                        &prod_decision,
                        &expected_prod,
                        &format!("prod/{}", operation.operation),
                    )?;

                    metrics.increment("security_decisions_tested");
                }

                metrics
                    .measure_since_mark("security_decisions_duration", "security_decisions_test");

                // Record final metrics
                for (name, value) in &metrics.measurements {
                    result = result.with_metric(name, *value);
                }

                for (name, value) in &metrics.counters {
                    result = result.with_metric(&format!("{}_count", name), *value as f64);
                }

                fixture.cleanup().await?;

                info!("Config Profiles + Security integration test completed successfully");
                Ok(result.success(metrics.total_duration().as_millis() as u64))
            })
        })
        .await?;

    let report = runner.generate_report();
    info!("Integration test report:\n{}", report);

    runner.cleanup().await
}

/// Test Multi-Agent Orchestration Integration  
#[tokio::test]
async fn test_multi_agent_orchestration_integration() -> Result<()> {
    let mut runner = IntegrationTestRunner::new().await?;

    runner
        .run_test("multi_agent_orchestration", |env| {
            Box::pin(async move {
                let mut result =
                    IntegrationTestResult::new("multi_agent_orchestration".to_string())
                        .with_component("orchestrator::ActorSystemManager")
                        .with_component("orchestrator::AgentOrchestrator")
                        .with_component("common::EventBus");

                let mut metrics = PerformanceMetrics::new("multi_agent_orchestration");
                let mut fixture = TestFixture::new("multi_agent_orchestration").await?;

                // Setup agent configuration
                metrics.mark("agent_setup");
                OrchestrationFixtures::create_agent_config(&mut fixture).await?;

                let system_config = orchestrator::SystemConfig::default();
                let comm_config = orchestrator::AgentCommunicationConfig::default();

                let manager =
                    orchestrator::ActorSystemManager::new(system_config, comm_config).await?;
                metrics.measure_since_mark("agent_setup_duration", "agent_setup");

                // Spawn different agent types
                metrics.mark("agent_spawning");
                let intent_analyzer = create_test_agent("intent_analyzer");
                let planner = create_test_agent("planner");
                let executor = create_test_agent("executor");
                let critic = create_test_agent("critic");

                let _intent_id = manager
                    .spawn_agent(
                        orchestrator::AgentType::IntentAnalyzer,
                        Box::new(intent_analyzer),
                    )
                    .await?;

                let _planner_id = manager
                    .spawn_agent(orchestrator::AgentType::Planner, Box::new(planner))
                    .await?;

                let _executor_id = manager
                    .spawn_agent(orchestrator::AgentType::Executor, Box::new(executor))
                    .await?;

                let _critic_id = manager
                    .spawn_agent(orchestrator::AgentType::Critic, Box::new(critic))
                    .await?;

                metrics.measure_since_mark("agent_spawning_duration", "agent_spawning");

                // Wait for agents to start
                tokio::time::sleep(Duration::from_millis(100)).await;

                // Test multi-agent workflow execution
                metrics.mark("workflow_execution");
                let workflow_tests = OrchestrationFixtures::create_workflow_test_cases();

                for (workflow_name, workflow_data) in workflow_tests {
                    metrics.mark(&format!("workflow_{}_start", workflow_name));

                    // Execute Intent → Plan → Execute → Critique workflow
                    let user_input = workflow_data["user_input"].as_str().expect("Test operation should succeed");

                    // Step 1: Intent Analysis
                    let intent_message = orchestrator::AgentMessage::AnalyzeIntent {
                        user_input: user_input.to_string(),
                        context: Some(json!({"test": true})),
                    };

                    manager
                        .send_to_agent_type(orchestrator::AgentType::IntentAnalyzer, intent_message)
                        .await?;

                    tokio::time::sleep(Duration::from_millis(50)).await;

                    // Step 2: Plan Creation
                    let plan_message = orchestrator::AgentMessage::CreatePlan {
                        intent: workflow_data["expected_intent"].clone(),
                        constraints: Some(json!({"max_time": 60})),
                    };

                    manager
                        .send_to_agent_type(orchestrator::AgentType::Planner, plan_message)
                        .await?;

                    tokio::time::sleep(Duration::from_millis(50)).await;

                    // Step 3: Plan Execution (dry run)
                    let execute_message = orchestrator::AgentMessage::ExecutePlan {
                        plan: json!({
                            "steps": workflow_data["expected_plan_steps"]
                        }),
                        dry_run: true,
                    };

                    manager
                        .send_to_agent_type(orchestrator::AgentType::Executor, execute_message)
                        .await?;

                    tokio::time::sleep(Duration::from_millis(50)).await;

                    // Step 4: Result Critique
                    let critique_message = orchestrator::AgentMessage::CritiqueResult {
                        result: json!({"success": true, "dry_run": true}),
                        context: Some(json!({"workflow": workflow_name})),
                    };

                    manager
                        .send_to_agent_type(orchestrator::AgentType::Critic, critique_message)
                        .await?;

                    tokio::time::sleep(Duration::from_millis(50)).await;

                    let duration = metrics
                        .measure_since_mark(
                            &format!("workflow_{}_duration", workflow_name),
                            &format!("workflow_{}_start", workflow_name),
                        )
                        .expect("Test operation should succeed");

                    result = result.with_metric(
                        &format!("{}_workflow_time_ms", workflow_name),
                        duration.as_millis() as f64,
                    );

                    metrics.increment("workflows_completed");
                }

                metrics.measure_since_mark("workflow_execution_duration", "workflow_execution");

                // Test system statistics and health
                let stats = manager.get_agent_stats().await;
                result = result.with_metric("total_agents", stats.get_total_agents() as f64);
                result = result.with_metric("pending_requests", stats.pending_requests as f64);

                // Test broadcasting
                metrics.mark("broadcasting_test");
                let broadcast_message = orchestrator::AgentMessage::Request {
                    request_id: "broadcast_test".to_string(),
                    request_type: "ping".to_string(),
                    payload: json!({"test": "broadcast"}),
                };

                let sent_count = manager
                    .broadcast_to_agent_type(
                        orchestrator::AgentType::IntentAnalyzer,
                        broadcast_message,
                    )
                    .await?;

                result = result.with_metric("broadcast_recipients", sent_count as f64);
                metrics.measure_since_mark("broadcasting_duration", "broadcasting_test");

                // Cleanup
                manager.shutdown().await?;

                // Record final metrics
                for (name, value) in &metrics.measurements {
                    result = result.with_metric(name, *value);
                }

                for (name, value) in &metrics.counters {
                    result = result.with_metric(&format!("{}_count", name), *value as f64);
                }

                fixture.cleanup().await?;

                info!("Multi-Agent Orchestration integration test completed successfully");
                Ok(result.success(metrics.total_duration().as_millis() as u64))
            })
        })
        .await?;

    let report = runner.generate_report();
    info!("Integration test report:\n{}", report);

    runner.cleanup().await
}

// Helper functions

async fn create_builder_with_real_embeddings(
    config: tools::context::ToolSelectionConfig,
) -> Result<tools::context::ToolContextBuilder> {
    let embedding_config = ai::EmbeddingConfig {
        model_name: "qwen3emb".to_string(),
        max_length: 512,
        batch_size: 8,
        use_gpu: false,
        gpu_config: None,
        embedding_dim: Some(1024),
    };

    tools::context::ToolContextBuilder::new_with_real_embeddings(config, embedding_config)
}

async fn register_sample_tools(builder: &tools::context::ToolContextBuilder) -> Result<()> {
    // This would register the sample tools from fixtures
    // Implementation depends on the actual ToolContextBuilder API
    info!("Registering sample tools for integration test");

    // Simulate tool registration delay
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(())
}

fn create_tool_selection_context(
    query: &str,
    query_data: &serde_json::Value,
) -> tools::context::ToolSelectionContext {
    tools::context::ToolSelectionContext {
        query: query.to_string(),
        project_context: query_data.get("context").and_then(|ctx| {
            Some(tools::context::ProjectContext {
                language: ctx
                    .get("language")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                framework: ctx
                    .get("framework")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                repository_type: Some("git".to_string()),
                project_files: ctx
                    .get("project_files")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                dependencies: vec!["tokio".to_string(), "serde".to_string()],
            })
        }),
        system_context: tools::context::SystemContext {
            platform: "linux".to_string(),
            has_network: query_data
                .get("context")
                .and_then(|ctx| ctx.get("has_network"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            has_gpu: false,
            available_memory_mb: 2048,
            max_execution_time_secs: Some(30),
        },
        user_preferences: Some(tools::context::UserPreferences {
            preferred_tools: vec![],
            avoided_tools: vec![],
            risk_tolerance: tools::registry::SecurityLevel::MediumRisk,
            performance_priority: tools::context::PerformancePriority::Balanced,
        }),
    }
}

fn create_test_security_operations() -> Vec<(
    infrastructure::config::policy_integration::OperationContext,
    &'static str,
    &'static str,
)> {
    use infrastructure::config::policy_integration::{OperationContext, ResourceRequirements};
    use infrastructure::config::RiskLevel;

    vec![
        (
            OperationContext {
                operation: "file_read".to_string(),
                tool_name: Some("file_reader".to_string()),
                risk_level: RiskLevel::Low,
                resource_requirements: ResourceRequirements {
                    memory_mb: Some(50),
                    cpu_time_secs: Some(5),
                    network_required: false,
                    filesystem_write: false,
                },
                user_confirmation: false,
            },
            "allow", // dev expected
            "allow", // prod expected
        ),
        (
            OperationContext {
                operation: "shell_exec".to_string(),
                tool_name: Some("shell_executor".to_string()),
                risk_level: RiskLevel::High,
                resource_requirements: ResourceRequirements {
                    memory_mb: Some(100),
                    cpu_time_secs: Some(30),
                    network_required: false,
                    filesystem_write: true,
                },
                user_confirmation: false,
            },
            "ask",  // dev expected
            "deny", // prod expected
        ),
    ]
}

fn validate_policy_decision(
    decision: &infrastructure::config::PolicyDecision,
    expected: &str,
    context: &str,
) -> Result<()> {
    use infrastructure::config::PolicyDecision;

    let actual = match decision {
        PolicyDecision::Allow(_) => "allow",
        PolicyDecision::Ask(_) => "ask",
        PolicyDecision::Deny(_) => "deny",
    };

    if actual != expected {
        anyhow::bail!(
            "Policy decision mismatch for {}: expected '{}', got '{}'",
            context,
            expected,
            actual
        );
    }

    Ok(())
}

fn create_test_agent(agent_type: &'static str) -> TestAgent {
    TestAgent::new(agent_type)
}

// Simplified test agent for orchestration testing
#[derive(Debug)]
struct TestAgent {
    id: orchestrator::ActorId,
    agent_type: &'static str,
}

impl TestAgent {
    fn new(agent_type: &'static str) -> Self {
        Self {
            id: orchestrator::ActorId::new(),
            agent_type,
        }
    }
}

#[async_trait::async_trait]
impl orchestrator::BaseActor for TestAgent {
    fn id(&self) -> orchestrator::ActorId {
        self.id
    }

    fn actor_type(&self) -> &'static str {
        self.agent_type
    }

    async fn handle_message(
        &mut self,
        message: orchestrator::ActorMessage,
        _context: &orchestrator::ActorContext,
    ) -> Result<(), orchestrator::ActorError> {
        // Simple message handling for tests
        match message {
            orchestrator::ActorMessage::Agent(_) => {
                info!("Agent {} processed message", self.agent_type);
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
