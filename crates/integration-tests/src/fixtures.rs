//! Test fixtures for integration testing
//!
//! Provides pre-configured test data and environments for various integration scenarios

use crate::common::TestFixture;
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;

/// Tool Context Builder test fixtures
pub struct ToolContextFixtures;

impl ToolContextFixtures {
    /// Create fixture with sample tools for testing Tool Context Builder
    pub async fn create_sample_tools(fixture: &mut TestFixture) -> Result<()> {
        // Sample tool registry data
        let tools_data = json!({
            "tools": [
                {
                    "name": "file_reader",
                    "description": "Read files from the filesystem safely",
                    "category": "FileSystem",
                    "version": "1.2.0",
                    "usage": "file_reader --path <file_path> [--encoding <utf8|ascii>]",
                    "examples": [
                        "file_reader --path ./config.json",
                        "file_reader --path /etc/hosts --encoding ascii"
                    ],
                    "permissions": {
                        "fs_read_roots": ["./", "/tmp"],
                        "fs_write_roots": [],
                        "net_allowlist": [],
                        "allow_shell": false
                    },
                    "supports_dry_run": true,
                    "platforms": ["linux", "mac", "win"],
                    "risk_score": 2
                },
                {
                    "name": "web_fetch",
                    "description": "Fetch data from web URLs with JSON parsing support",
                    "category": "Web",
                    "version": "2.0.1",
                    "usage": "web_fetch --url <url> [--headers <json>] [--timeout <seconds>]",
                    "examples": [
                        "web_fetch --url https://api.github.com/users/octocat",
                        "web_fetch --url https://httpbin.org/json --timeout 30"
                    ],
                    "permissions": {
                        "fs_read_roots": [],
                        "fs_write_roots": [],
                        "net_allowlist": ["*"],
                        "allow_shell": false
                    },
                    "supports_dry_run": true,
                    "platforms": ["linux", "mac", "win"],
                    "risk_score": 3
                },
                {
                    "name": "shell_exec",
                    "description": "Execute system shell commands with output capture",
                    "category": "System",
                    "version": "1.0.0",
                    "usage": "shell_exec --command <cmd> [--timeout <seconds>]",
                    "examples": [
                        "shell_exec --command 'ls -la'",
                        "shell_exec --command 'ps aux' --timeout 10"
                    ],
                    "permissions": {
                        "fs_read_roots": ["./"],
                        "fs_write_roots": ["./tmp"],
                        "net_allowlist": [],
                        "allow_shell": true
                    },
                    "supports_dry_run": true,
                    "platforms": ["linux", "mac"],
                    "risk_score": 7
                },
                {
                    "name": "git_status",
                    "description": "Get git repository status and file changes",
                    "category": "Git",
                    "version": "1.1.0",
                    "usage": "git_status [--porcelain] [--branch]",
                    "examples": [
                        "git_status",
                        "git_status --porcelain --branch"
                    ],
                    "permissions": {
                        "fs_read_roots": ["./"],
                        "fs_write_roots": [],
                        "net_allowlist": [],
                        "allow_shell": false
                    },
                    "supports_dry_run": false,
                    "platforms": ["linux", "mac", "win"],
                    "risk_score": 1
                }
            ]
        });

        fixture.create_test_data("sample_tools", tools_data).await?;
        Ok(())
    }

    /// Create test queries for tool selection
    pub fn create_test_queries() -> Vec<(String, serde_json::Value)> {
        vec![
            (
                "file_operations".to_string(),
                json!({
                    "query": "read configuration file from project directory",
                    "expected_tools": ["file_reader", "git_status"],
                    "context": {
                        "language": "rust",
                        "framework": null,
                        "project_files": ["Cargo.toml", "src/main.rs"]
                    }
                }),
            ),
            (
                "web_operations".to_string(),
                json!({
                    "query": "fetch data from REST API and parse JSON response",
                    "expected_tools": ["web_fetch"],
                    "context": {
                        "has_network": true,
                        "risk_tolerance": "medium"
                    }
                }),
            ),
            (
                "system_operations".to_string(),
                json!({
                    "query": "execute system command and capture output",
                    "expected_tools": ["shell_exec"],
                    "context": {
                        "platform": "linux",
                        "risk_tolerance": "high"
                    }
                }),
            ),
        ]
    }
}

/// Configuration Profile test fixtures
pub struct ConfigProfileFixtures;

impl ConfigProfileFixtures {
    /// Create development profile configuration
    pub async fn create_dev_profile(fixture: &mut TestFixture) -> Result<std::path::PathBuf> {
        let dev_config = r#"
# Development Profile Configuration
[profile]
name = "dev"
description = "Development environment with relaxed security"

[logging]
level = "debug"
structured = true
file_output = true

[security]
default_mode = "ask"
risk_level = "medium"
permissive_mode = true

[sandbox]
enabled = true
strict_mode = false

[tools]
require_signed_tools = false
allow_experimental = true

[performance]
cache_enabled = true
batch_size = 50
timeout_secs = 30

[emergency_overrides]
enabled = true
require_token = false
"#;

        fixture.create_config_file("dev", dev_config).await
    }

    /// Create production profile configuration
    pub async fn create_prod_profile(fixture: &mut TestFixture) -> Result<std::path::PathBuf> {
        let prod_config = r#"
# Production Profile Configuration
[profile]
name = "prod"
description = "Production environment with strict security"

[logging]
level = "info"
structured = true
file_output = true

[security]
default_mode = "deny"
risk_level = "low"
permissive_mode = false

[sandbox]
enabled = true
strict_mode = true

[tools]
require_signed_tools = true
allow_experimental = false

[performance]
cache_enabled = true
batch_size = 100
timeout_secs = 60

[emergency_overrides]
enabled = false
require_token = true
"#;

        fixture.create_config_file("prod", prod_config).await
    }

    /// Create base configuration
    pub async fn create_base_config(fixture: &mut TestFixture) -> Result<std::path::PathBuf> {
        let base_config = r#"
# Base Configuration
[database]
connection_string = "sqlite:memory:"
pool_size = 10

[ai]
embedding_model = "qwen3emb"
reranker_model = "qwen3_reranker"
batch_size = 8
max_length = 512

[memory]
index_type = "hnsw"
vector_dim = 1024
cache_size_mb = 512

[network]
timeout_secs = 30
retry_attempts = 3
user_agent = "MAGRAY-CLI/1.0"
"#;

        fixture.create_config_file("base", base_config).await
    }
}

/// Multi-Agent Orchestration test fixtures
pub struct OrchestrationFixtures;

impl OrchestrationFixtures {
    /// Create sample agent configuration
    pub async fn create_agent_config(fixture: &mut TestFixture) -> Result<()> {
        let agent_config = json!({
            "system_config": {
                "max_agents_per_type": 3,
                "default_timeout_secs": 30,
                "enable_health_checks": true,
                "health_check_interval_secs": 10
            },
            "communication_config": {
                "enable_broadcasting": true,
                "message_timeout_secs": 5,
                "max_pending_requests": 100,
                "enable_request_tracing": true
            },
            "agent_types": {
                "intent_analyzer": {
                    "enabled": true,
                    "max_instances": 2,
                    "timeout_secs": 15
                },
                "planner": {
                    "enabled": true,
                    "max_instances": 2,
                    "timeout_secs": 30
                },
                "executor": {
                    "enabled": true,
                    "max_instances": 3,
                    "timeout_secs": 60
                },
                "critic": {
                    "enabled": true,
                    "max_instances": 1,
                    "timeout_secs": 20
                },
                "scheduler": {
                    "enabled": true,
                    "max_instances": 1,
                    "timeout_secs": 10
                }
            }
        });

        fixture
            .create_test_data("agent_config", agent_config)
            .await?;
        Ok(())
    }

    /// Create sample workflow test cases
    pub fn create_workflow_test_cases() -> Vec<(String, serde_json::Value)> {
        vec![
            (
                "simple_task".to_string(),
                json!({
                    "user_input": "Create a new user account with email john@example.com",
                    "expected_intent": {
                        "action": "create_user",
                        "confidence": 0.95,
                        "parameters": {
                            "email": "john@example.com"
                        }
                    },
                    "expected_plan_steps": [
                        "validate_email",
                        "check_user_exists",
                        "create_user_record",
                        "send_welcome_email"
                    ],
                    "execution_mode": "dry_run"
                }),
            ),
            (
                "complex_workflow".to_string(),
                json!({
                    "user_input": "Analyze project dependencies and generate security report",
                    "expected_intent": {
                        "action": "security_analysis",
                        "confidence": 0.85,
                        "parameters": {
                            "scope": "dependencies"
                        }
                    },
                    "expected_plan_steps": [
                        "scan_project_files",
                        "extract_dependencies",
                        "check_vulnerabilities",
                        "generate_report",
                        "recommend_fixes"
                    ],
                    "execution_mode": "dry_run"
                }),
            ),
        ]
    }
}

/// Security Integration test fixtures
pub struct SecurityFixtures;

impl SecurityFixtures {
    /// Create security policy test scenarios
    pub async fn create_security_scenarios(fixture: &mut TestFixture) -> Result<()> {
        let security_scenarios = json!({
            "scenarios": [
                {
                    "name": "low_risk_file_read",
                    "operation": "file_read",
                    "tool": "file_reader",
                    "risk_level": "low",
                    "resource_requirements": {
                        "memory_mb": 50,
                        "cpu_time_secs": 5,
                        "network_required": false,
                        "filesystem_write": false
                    },
                    "expected_decisions": {
                        "dev": "allow",
                        "prod": "allow"
                    }
                },
                {
                    "name": "high_risk_shell_exec",
                    "operation": "shell_exec",
                    "tool": "shell_executor",
                    "risk_level": "high",
                    "resource_requirements": {
                        "memory_mb": 100,
                        "cpu_time_secs": 30,
                        "network_required": false,
                        "filesystem_write": true
                    },
                    "expected_decisions": {
                        "dev": "ask",
                        "prod": "deny"
                    }
                },
                {
                    "name": "medium_risk_network",
                    "operation": "network_request",
                    "tool": "http_client",
                    "risk_level": "medium",
                    "resource_requirements": {
                        "memory_mb": 75,
                        "cpu_time_secs": 15,
                        "network_required": true,
                        "filesystem_write": false
                    },
                    "expected_decisions": {
                        "dev": "ask",
                        "prod": "ask"
                    }
                }
            ]
        });

        fixture
            .create_test_data("security_scenarios", security_scenarios)
            .await?;
        Ok(())
    }

    /// Create audit event test data
    pub fn create_audit_events() -> Vec<serde_json::Value> {
        vec![
            json!({
                "event_type": "policy_decision",
                "timestamp": "2025-08-12T10:45:00Z",
                "operation": "file_read",
                "tool": "file_reader",
                "decision": "allow",
                "risk_level": "low",
                "user_confirmed": false
            }),
            json!({
                "event_type": "tool_execution",
                "timestamp": "2025-08-12T10:45:30Z",
                "tool": "file_reader",
                "args": {"path": "./test.txt"},
                "success": true,
                "duration_ms": 150
            }),
            json!({
                "event_type": "policy_violation",
                "timestamp": "2025-08-12T10:46:00Z",
                "operation": "shell_exec",
                "tool": "shell_executor",
                "violation": "command_blacklisted",
                "blocked_command": "rm -rf /"
            }),
        ]
    }
}

/// End-to-End workflow test fixtures
pub struct E2EFixtures;

impl E2EFixtures {
    /// Create comprehensive end-to-end test scenario
    pub async fn create_comprehensive_scenario(fixture: &mut TestFixture) -> Result<()> {
        let scenario = json!({
            "name": "full_integration_workflow",
            "description": "Complete workflow testing all integrated components",
            "phases": [
                {
                    "phase": "configuration_setup",
                    "steps": [
                        "load_base_config",
                        "apply_dev_profile",
                        "validate_policy_integration"
                    ]
                },
                {
                    "phase": "tool_registration",
                    "steps": [
                        "register_sample_tools",
                        "validate_tool_metadata",
                        "build_tool_embeddings"
                    ]
                },
                {
                    "phase": "agent_initialization",
                    "steps": [
                        "start_agent_system",
                        "spawn_all_agent_types",
                        "verify_agent_health"
                    ]
                },
                {
                    "phase": "workflow_execution",
                    "steps": [
                        "analyze_user_intent",
                        "create_execution_plan",
                        "select_appropriate_tools",
                        "execute_plan_with_tools",
                        "critique_results"
                    ]
                },
                {
                    "phase": "security_validation",
                    "steps": [
                        "verify_policy_enforcement",
                        "check_audit_trail",
                        "validate_sandboxing"
                    ]
                }
            ],
            "success_criteria": {
                "all_phases_complete": true,
                "no_security_violations": true,
                "performance_within_bounds": true,
                "audit_trail_complete": true
            }
        });

        fixture.create_test_data("e2e_scenario", scenario).await?;
        Ok(())
    }
}

/// Performance test fixtures
pub struct PerformanceFixtures;

impl PerformanceFixtures {
    /// Create load testing scenarios
    pub fn create_load_test_scenarios() -> Vec<(String, serde_json::Value)> {
        vec![
            (
                "tool_selection_load".to_string(),
                json!({
                    "name": "Tool Selection Load Test",
                    "concurrent_requests": 50,
                    "total_requests": 1000,
                    "queries": [
                        "read project configuration file",
                        "fetch API data from remote service",
                        "execute build command and capture output",
                        "check git repository status"
                    ],
                    "performance_targets": {
                        "avg_response_time_ms": 200,
                        "p95_response_time_ms": 500,
                        "max_memory_growth_mb": 100
                    }
                }),
            ),
            (
                "agent_coordination_load".to_string(),
                json!({
                    "name": "Agent Coordination Load Test",
                    "concurrent_workflows": 20,
                    "total_workflows": 200,
                    "workflow_types": [
                        "simple_task",
                        "complex_analysis",
                        "multi_step_execution"
                    ],
                    "performance_targets": {
                        "avg_workflow_time_ms": 2000,
                        "p95_workflow_time_ms": 5000,
                        "agent_failure_rate": 0.01
                    }
                }),
            ),
        ]
    }

    /// Create stress testing scenarios
    pub fn create_stress_test_scenarios() -> Vec<(String, serde_json::Value)> {
        vec![
            (
                "memory_pressure".to_string(),
                json!({
                    "name": "Memory Pressure Test",
                    "description": "Test system behavior under memory pressure",
                    "memory_pressure_mb": 1024,
                    "concurrent_operations": 100,
                    "operation_types": [
                        "large_embedding_generation",
                        "complex_tool_selection",
                        "multi_agent_workflows"
                    ],
                    "success_criteria": {
                        "no_oom_errors": true,
                        "graceful_degradation": true,
                        "recovery_after_pressure": true
                    }
                }),
            ),
            (
                "component_failure".to_string(),
                json!({
                    "name": "Component Failure Test",
                    "description": "Test system resilience to component failures",
                    "failure_scenarios": [
                        {
                            "component": "ai_embeddings",
                            "failure_type": "timeout",
                            "expected_fallback": "mock_embeddings"
                        },
                        {
                            "component": "agent_executor",
                            "failure_type": "crash",
                            "expected_fallback": "restart_agent"
                        },
                        {
                            "component": "config_loader",
                            "failure_type": "invalid_config",
                            "expected_fallback": "default_config"
                        }
                    ],
                    "success_criteria": {
                        "system_continues_operation": true,
                        "fallback_mechanisms_work": true,
                        "recovery_time_acceptable": true
                    }
                }),
            ),
        ]
    }
}
