//! Integration Benchmarks
//!
//! Performance benchmarks for integrated components to ensure
//! performance regression detection in CI/CD pipeline

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use integration_tests::{
    common::{PerformanceMetrics, TestFixture},
    fixtures::{PerformanceFixtures, ToolContextFixtures},
    TestEnvironment,
};
#[cfg(all(unix, feature = "profiling"))]
use pprof::criterion::{Output, PProfProfiler};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark tool selection pipeline performance
fn bench_tool_selection_pipeline(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create Tokio runtime for benchmark");

    // Setup benchmark environment
    let env = rt.block_on(async { TestEnvironment::setup().await.expect("Failed to setup test environment") });

    let mut fixture =
        rt.block_on(async { TestFixture::new("tool_selection_benchmark").await.expect("Failed to create test fixture") });

    // Setup tool context builder
    let builder = rt.block_on(async {
        ToolContextFixtures::create_sample_tools(&mut fixture)
            .await
            .expect("Failed to create actor system manager");

        let config = tools::context::ToolSelectionConfig {
            max_candidates: 20,
            top_n_tools: 5,
            similarity_threshold: 0.2,
            allowed_platforms: vec!["linux".into()],
            max_security_level: tools::registry::SecurityLevel::MediumRisk,
            allowed_categories: None,
            enable_reranking: true,
        };

        if env.use_real_ai {
            match create_builder_with_real_embeddings(config.clone()).await {
                Ok(b) => b,
                Err(_) => tools::context::ToolContextBuilder::new(config),
            }
        } else {
            tools::context::ToolContextBuilder::new(config)
        }
    });

    // Benchmark different query types
    let queries = vec![
        ("simple_file_read", "read a configuration file"),
        ("complex_web_api", "fetch data from REST API, parse JSON, validate schema, and update local database"),
        ("multi_tool_workflow", "analyze project dependencies, check for security vulnerabilities, generate report, and send notifications"),
    ];

    let mut group = c.benchmark_group("tool_selection_pipeline");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(10));

    for (query_name, query) in queries {
        group.bench_with_input(
            BenchmarkId::new("build_context", query_name),
            &(query_name, query),
            |b, (name, query_text)| {
                b.to_async(&rt).iter(|| async {
                    let context = create_benchmark_context(query_text);
                    let result = builder.build_context(black_box(context)).await.expect("Failed to build context in benchmark");
                    black_box(result)
                });
            },
        );
    }

    group.finish();

    // Cleanup
    rt.block_on(async {
        fixture.cleanup().await.expect("Failed to cleanup test fixture");
        env.cleanup().await.expect("Failed to cleanup test environment");
    });
}

/// Benchmark multi-agent coordination performance
fn bench_multi_agent_coordination(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create Tokio runtime for benchmark");

    // Setup agent system
    let (manager, env) = rt.block_on(async {
        let env = TestEnvironment::setup().await.expect("Failed to setup test environment for agents");

        let system_config = orchestrator::SystemConfig::default();
        let comm_config = orchestrator::AgentCommunicationConfig::default();
        let manager = orchestrator::ActorSystemManager::new(system_config, comm_config)
            .await
            .expect("Failed to create actor system manager");

        // Spawn agents
        for _ in 0..2 {
            let intent_analyzer = create_benchmark_agent("intent_analyzer");
            manager
                .spawn_agent(
                    orchestrator::AgentType::IntentAnalyzer,
                    Box::new(intent_analyzer),
                )
                .await
                .expect("Failed to create actor system manager");

            let planner = create_benchmark_agent("planner");
            manager
                .spawn_agent(orchestrator::AgentType::Planner, Box::new(planner))
                .await
                .expect("Failed to create actor system manager");

            let executor = create_benchmark_agent("executor");
            manager
                .spawn_agent(orchestrator::AgentType::Executor, Box::new(executor))
                .await
                .expect("Failed to create actor system manager");

            let critic = create_benchmark_agent("critic");
            manager
                .spawn_agent(orchestrator::AgentType::Critic, Box::new(critic))
                .await
                .expect("Failed to create actor system manager");
        }

        tokio::time::sleep(Duration::from_millis(100)).await; // Let agents start

        (manager, env)
    });

    let workflow_types = vec![
        ("simple_task", "Create a new user account"),
        (
            "analysis_task",
            "Analyze project structure and dependencies",
        ),
        (
            "complex_workflow",
            "Multi-step data processing with validation and reporting",
        ),
    ];

    let mut group = c.benchmark_group("multi_agent_coordination");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(8));

    for (workflow_name, task_description) in workflow_types {
        group.bench_with_input(
            BenchmarkId::new("execute_workflow", workflow_name),
            &(workflow_name, task_description),
            |b, (name, task)| {
                b.to_async(&rt).iter(|| async {
                    let workflow_result = execute_benchmark_workflow(&manager, task).await.expect("Test operation should succeed");
                    black_box(workflow_result)
                });
            },
        );
    }

    // Test concurrent workflows
    let concurrent_levels = vec![1, 5, 10, 20];

    for concurrent_count in concurrent_levels {
        group.bench_with_input(
            BenchmarkId::new("concurrent_workflows", concurrent_count),
            &concurrent_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let mut tasks = Vec::new();

                    for i in 0..count {
                        let manager_ref = &manager;
                        let task = async move {
                            execute_benchmark_workflow(
                                manager_ref,
                                &format!("Concurrent task {}", i),
                            )
                            .await
                        };
                        tasks.push(task);
                    }

                    let results = futures::future::join_all(tasks).await;
                    black_box(results)
                });
            },
        );
    }

    group.finish();

    // Cleanup
    rt.block_on(async {
        manager.shutdown().await.expect("Test operation should succeed");
        env.cleanup().await.expect("Failed to cleanup test environment");
    });
}

/// Benchmark configuration management performance
fn bench_configuration_management(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create Tokio runtime for benchmark");

    let (config_loader, configs, env) = rt.block_on(async {
        let env = TestEnvironment::setup().await.expect("Failed to setup test environment for agents");
        let mut fixture = TestFixture::new("config_benchmark").await.expect("Test operation should succeed");

        // Create configuration files
        let base_config = create_benchmark_base_config();
        let dev_profile = create_benchmark_dev_profile();
        let prod_profile = create_benchmark_prod_profile();

        fixture
            .create_test_data(
                "base_config",
                serde_json::to_value(base_config.clone()).expect("Test operation should succeed"),
            )
            .await
            .expect("Failed to create actor system manager");
        fixture
            .create_test_data(
                "dev_profile",
                serde_json::to_value(dev_profile.clone()).expect("Test operation should succeed"),
            )
            .await
            .expect("Failed to create actor system manager");
        fixture
            .create_test_data(
                "prod_profile",
                serde_json::to_value(prod_profile.clone()).expect("Test operation should succeed"),
            )
            .await
            .expect("Failed to create actor system manager");

        let config_loader = infrastructure::config::ConfigLoader::new();

        let configs = std::collections::HashMap::from([
            ("base".to_string(), base_config),
            ("dev".to_string(), dev_profile),
            ("prod".to_string(), prod_profile),
        ]);

        (config_loader, configs, env)
    });

    let mut group = c.benchmark_group("configuration_management");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    // Benchmark configuration loading
    group.bench_function("load_configuration", |b| {
        b.to_async(&rt).iter(|| async {
            let mut config = configs["base"].clone();
            config.apply_profile(&configs["dev"]);
            black_box(config)
        });
    });

    // Benchmark profile switching
    let profile_switches = vec![
        (
            "dev_to_prod",
            domain::config::Profile::Dev,
            domain::config::Profile::Prod,
        ),
        (
            "prod_to_dev",
            domain::config::Profile::Prod,
            domain::config::Profile::Dev,
        ),
    ];

    for (switch_name, from_profile, to_profile) in profile_switches {
        group.bench_with_input(
            BenchmarkId::new("profile_switch", switch_name),
            &(from_profile, to_profile),
            |b, (from, to)| {
                b.to_async(&rt).iter(|| async {
                    let mut config = configs["base"].clone();
                    config.profile = from.clone();
                    config.apply_profile(&configs["dev"]);

                    let switched_config = config_loader
                        .switch_profile(config, to.clone())
                        .await
                        .expect("Failed to create actor system manager");
                    black_box(switched_config)
                });
            },
        );
    }

    group.finish();

    // Cleanup
    rt.block_on(async {
        env.cleanup().await.expect("Failed to cleanup test environment");
    });
}

/// Benchmark security policy evaluation performance
fn bench_security_policy_evaluation(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create Tokio runtime for benchmark");

    let (policy_engine, test_contexts, env) = rt.block_on(async {
        let env = TestEnvironment::setup().await.expect("Failed to setup test environment for agents");

        let mut policy_engine = infrastructure::config::PolicyIntegrationEngine::new();

        // Setup with production profile for consistent benchmarking
        let mut prod_config = domain::config::MagrayConfig::default();
        prod_config.profile = domain::config::Profile::Prod;
        prod_config.profile_config = Some(domain::config::ProfileConfig::prod());
        prod_config.apply_profile(&domain::config::ProfileConfig::prod());

        policy_engine
            .apply_profile_policy(&prod_config)
            .await
            .expect("Failed to create actor system manager");

        // Create test operation contexts
        let test_contexts = create_benchmark_security_contexts();

        (policy_engine, test_contexts, env)
    });

    let mut group = c.benchmark_group("security_policy_evaluation");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));

    // Benchmark individual policy decisions
    for (context_name, context) in &test_contexts {
        group.bench_with_input(
            BenchmarkId::new("policy_decision", context_name),
            context,
            |b, ctx| {
                b.iter(|| {
                    let decision =
                        policy_engine.check_operation_allowed(&ctx.operation, black_box(ctx));
                    black_box(decision)
                });
            },
        );
    }

    // Benchmark batch policy evaluation
    let batch_sizes = vec![10, 50, 100, 200];

    for batch_size in batch_sizes {
        group.bench_with_input(
            BenchmarkId::new("batch_evaluation", batch_size),
            &batch_size,
            |b, &size| {
                b.iter(|| {
                    let mut decisions = Vec::with_capacity(size);
                    for i in 0..size {
                        let context = &test_contexts[i % test_contexts.len()].1;
                        let decision = policy_engine
                            .check_operation_allowed(&context.operation, black_box(context));
                        decisions.push(decision);
                    }
                    black_box(decisions)
                });
            },
        );
    }

    group.finish();

    // Cleanup
    rt.block_on(async {
        env.cleanup().await.expect("Failed to cleanup test environment");
    });
}

/// Benchmark memory usage patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create Tokio runtime for benchmark");

    let mut group = c.benchmark_group("memory_patterns");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(5));

    // Benchmark tool context builder memory usage
    group.bench_function("tool_context_builder_memory", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let start = std::time::Instant::now();

            for _i in 0..iters {
                let config = tools::context::ToolSelectionConfig::default();
                let builder = tools::context::ToolContextBuilder::new(config);

                // Simulate tool selection operations
                for j in 0..10 {
                    let context = create_benchmark_context(&format!("Memory test query {}", j));
                    let _result = builder.build_context(black_box(context)).await.expect("Test operation should succeed");
                }

                // Force cleanup
                drop(builder);
            }

            start.elapsed()
        });
    });

    // Benchmark agent system memory usage
    group.bench_function("agent_system_memory", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let start = std::time::Instant::now();

            for _i in 0..iters {
                let system_config = orchestrator::SystemConfig::default();
                let comm_config = orchestrator::AgentCommunicationConfig::default();
                let manager = orchestrator::ActorSystemManager::new(system_config, comm_config)
                    .await
                    .expect("Failed to create actor system manager");

                // Spawn agents and execute workflows
                for j in 0..5 {
                    let agent = create_benchmark_agent("memory_test");
                    let _agent_id = manager
                        .spawn_agent(orchestrator::AgentType::IntentAnalyzer, Box::new(agent))
                        .await
                        .expect("Failed to create actor system manager");
                }

                // Execute some workflows
                for j in 0..3 {
                    let _result =
                        execute_benchmark_workflow(&manager, &format!("Memory workflow {}", j))
                            .await
                            .expect("Failed to create actor system manager");
                }

                manager.shutdown().await.expect("Test operation should succeed");
            }

            start.elapsed()
        });
    });

    group.finish();
}

// Helper functions

async fn create_builder_with_real_embeddings(
    config: tools::context::ToolSelectionConfig,
) -> Result<tools::context::ToolContextBuilder, anyhow::Error> {
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

fn create_benchmark_context(query: &str) -> tools::context::ToolSelectionContext {
    tools::context::ToolSelectionContext {
        query: query.to_string(),
        project_context: Some(tools::context::ProjectContext {
            language: Some("rust".to_string()),
            framework: None,
            repository_type: Some("git".to_string()),
            project_files: vec!["Cargo.toml".to_string()],
            dependencies: vec!["tokio".to_string()],
        }),
        system_context: tools::context::SystemContext {
            platform: "linux".to_string(),
            has_network: true,
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

fn create_benchmark_agent(agent_type: &'static str) -> BenchmarkAgent {
    BenchmarkAgent::new(agent_type)
}

#[derive(Debug)]
struct BenchmarkAgent {
    id: orchestrator::ActorId,
    agent_type: &'static str,
}

impl BenchmarkAgent {
    fn new(agent_type: &'static str) -> Self {
        Self {
            id: orchestrator::ActorId::new(),
            agent_type,
        }
    }
}

#[async_trait::async_trait]
impl orchestrator::BaseActor for BenchmarkAgent {
    fn id(&self) -> orchestrator::ActorId {
        self.id
    }

    fn actor_type(&self) -> &'static str {
        self.agent_type
    }

    async fn handle_message(
        &mut self,
        _message: orchestrator::ActorMessage,
        _context: &orchestrator::ActorContext,
    ) -> Result<(), orchestrator::ActorError> {
        // Minimal processing for benchmarking
        tokio::time::sleep(Duration::from_micros(100)).await; // Simulate work
        Ok(())
    }
}

async fn execute_benchmark_workflow(
    manager: &orchestrator::ActorSystemManager,
    task_description: &str,
) -> Result<(), anyhow::Error> {
    // Execute a simplified workflow for benchmarking
    let intent_message = orchestrator::AgentMessage::AnalyzeIntent {
        user_input: task_description.to_string(),
        context: Some(serde_json::json!({"benchmark": true})),
    };

    manager
        .send_to_agent_type(orchestrator::AgentType::IntentAnalyzer, intent_message)
        .await?;

    tokio::time::sleep(Duration::from_millis(1)).await; // Allow processing

    Ok(())
}

fn create_benchmark_base_config() -> domain::config::MagrayConfig {
    let mut config = domain::config::MagrayConfig::default();
    config.database.connection_string = Some("sqlite:benchmark.db".to_string());
    config
}

fn create_benchmark_dev_profile() -> domain::config::ProfileConfig {
    domain::config::ProfileConfig::dev()
}

fn create_benchmark_prod_profile() -> domain::config::ProfileConfig {
    domain::config::ProfileConfig::prod()
}

fn create_benchmark_security_contexts() -> Vec<(
    String,
    infrastructure::config::policy_integration::OperationContext,
)> {
    vec![
        (
            "file_read_low_risk".to_string(),
            infrastructure::config::policy_integration::OperationContext {
                operation: "file_read".to_string(),
                tool_name: Some("file_reader".to_string()),
                risk_level: infrastructure::config::RiskLevel::Low,
                resource_requirements:
                    infrastructure::config::policy_integration::ResourceRequirements {
                        memory_mb: Some(50),
                        cpu_time_secs: Some(5),
                        network_required: false,
                        filesystem_write: false,
                    },
                user_confirmation: false,
            },
        ),
        (
            "shell_exec_high_risk".to_string(),
            infrastructure::config::policy_integration::OperationContext {
                operation: "shell_exec".to_string(),
                tool_name: Some("shell_executor".to_string()),
                risk_level: infrastructure::config::RiskLevel::High,
                resource_requirements:
                    infrastructure::config::policy_integration::ResourceRequirements {
                        memory_mb: Some(200),
                        cpu_time_secs: Some(60),
                        network_required: false,
                        filesystem_write: true,
                    },
                user_confirmation: false,
            },
        ),
        (
            "network_request_medium_risk".to_string(),
            infrastructure::config::policy_integration::OperationContext {
                operation: "network_request".to_string(),
                tool_name: Some("http_client".to_string()),
                risk_level: infrastructure::config::RiskLevel::Medium,
                resource_requirements:
                    infrastructure::config::policy_integration::ResourceRequirements {
                        memory_mb: Some(100),
                        cpu_time_secs: Some(30),
                        network_required: true,
                        filesystem_write: false,
                    },
                user_confirmation: false,
            },
        ),
    ]
}

// Define benchmark groups
criterion_group! {
    name = benches;
    config = {
        let mut criterion = Criterion::default()
            .warm_up_time(Duration::from_secs(1))
            .measurement_time(Duration::from_secs(10))
            .sample_size(50);

        // Add profiler only on Unix systems with profiling feature (flamegraph not available on Windows)
        #[cfg(all(unix, feature = "profiling"))]
        {
            criterion = criterion.with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
        }

        criterion
    };
    targets =
        bench_tool_selection_pipeline,
        bench_multi_agent_coordination,
        bench_configuration_management,
        bench_security_policy_evaluation,
        bench_memory_patterns
}

criterion_main!(benches);
