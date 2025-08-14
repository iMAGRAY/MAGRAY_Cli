// @component: {"k":"T","id":"orchestrator_tool_selection_integration","t":"End-to-end test for intelligent tool selection in orchestrator","m":{"cur":0,"tgt":100,"u":"%"},"f":["test","orchestrator","integration","planner"]}

use std::collections::HashMap;
use std::sync::Arc;

use orchestrator::agents::planner::{Planner, PlannerTrait};
use orchestrator::agents::intent_analyzer::{Intent, IntentType, IntentContext};
use tools::registry::{SecurityConfig, SecureToolRegistry};
use uuid::Uuid;
use chrono::Utc;

/// Test that Planner can be created with intelligent tool selection
#[tokio::test]
async fn test_planner_with_intelligent_tool_selection() {
    let security_config = SecurityConfig::default();
    let tool_registry = Arc::new(SecureToolRegistry::new(security_config));
    
    let planner_result = Planner::with_intelligent_tool_selection(tool_registry);
    assert!(planner_result.is_ok(), "Planner with intelligent tool selection should be created successfully");
    
    let planner = planner_result.unwrap();
    assert!(planner.has_intelligent_tool_selection(), "Planner should have intelligent tool selection enabled");
}

/// Test that Planner can build plans using intelligent tool selection
#[tokio::test]
async fn test_intelligent_plan_building() {
    let security_config = SecurityConfig::default();
    let tool_registry = Arc::new(SecureToolRegistry::new(security_config));
    
    let planner = Planner::with_intelligent_tool_selection(tool_registry)
        .expect("Failed to create planner with intelligent tool selection");
    
    // Create test intent for tool execution
    let intent = Intent {
        id: Uuid::new_v4(),
        intent_type: IntentType::ExecuteTool {
            tool_name: "file_manager".to_string(),
        },
        parameters: HashMap::new(),
        confidence: 0.9,
        context: IntentContext {
            session_id: Uuid::new_v4(),
            user_id: Some("test_user".to_string()),
            timestamp: Utc::now(),
            environment: HashMap::from([
                ("os".to_string(), "windows".to_string()),
                ("project_type".to_string(), "rust".to_string()),
            ]),
            conversation_history: vec![],
        },
    };
    
    let plan = planner.build_plan(&intent).await;
    assert!(plan.is_ok(), "Plan building with intelligent tool selection should succeed");
    
    let plan = plan.unwrap();
    assert_eq!(plan.intent_id, intent.id, "Plan should reference the correct intent");
    assert!(!plan.steps.is_empty(), "Plan should have at least one step");
    
    // Check that metadata includes intelligent selection information
    assert!(plan.metadata.contains_key("intelligent_selection_used"), "Plan should include intelligent selection metadata");
    if let Some(serde_json::Value::Bool(used)) = plan.metadata.get("intelligent_selection_used") {
        assert!(*used, "Plan should indicate intelligent selection was used");
    }
}

/// Test plan building for different intent types
#[tokio::test]
async fn test_intelligent_selection_for_different_intents() {
    let security_config = SecurityConfig::default();
    let tool_registry = Arc::new(SecureToolRegistry::new(security_config));
    
    let planner = Planner::with_intelligent_tool_selection(tool_registry)
        .expect("Failed to create planner with intelligent tool selection");
    
    let intent_types = vec![
        IntentType::ExecuteTool {
            tool_name: "git_status".to_string(),
        },
        IntentType::AskQuestion {
            question: "What files were modified?".to_string(),
        },
        IntentType::FileOperation {
            operation: "read".to_string(),
            path: "/tmp/test.txt".to_string(),
        },
        IntentType::MemoryOperation {
            operation: "search".to_string(),
        },
    ];
    
    for intent_type in intent_types {
        let intent = Intent {
            id: Uuid::new_v4(),
            intent_type: intent_type.clone(),
            parameters: HashMap::new(),
            confidence: 0.8,
            context: IntentContext {
                session_id: Uuid::new_v4(),
                user_id: Some("test_user".to_string()),
                timestamp: Utc::now(),
                environment: HashMap::new(),
                conversation_history: vec![],
            },
        };
        
        let plan = planner.build_plan(&intent).await;
        assert!(plan.is_ok(), "Plan building should succeed for intent type: {:?}", intent_type);
        
        let plan = plan.unwrap();
        assert!(!plan.steps.is_empty(), "Plan should have steps for intent type: {:?}", intent_type);
    }
}

/// Test fallback behavior when intelligent tool selection is not available
#[tokio::test]
async fn test_fallback_to_basic_planning() {
    // Create planner without intelligent tool selection
    let planner = Planner::new();
    assert!(!planner.has_intelligent_tool_selection(), "Basic planner should not have intelligent tool selection");
    
    let intent = Intent {
        id: Uuid::new_v4(),
        intent_type: IntentType::ExecuteTool {
            tool_name: "basic_tool".to_string(),
        },
        parameters: HashMap::new(),
        confidence: 0.7,
        context: IntentContext {
            session_id: Uuid::new_v4(),
            user_id: Some("test_user".to_string()),
            timestamp: Utc::now(),
            environment: HashMap::new(),
            conversation_history: vec![],
        },
    };
    
    let plan = planner.build_plan(&intent).await;
    assert!(plan.is_ok(), "Basic plan building should still work");
    
    let plan = plan.unwrap();
    assert!(!plan.steps.is_empty(), "Basic plan should have steps");
    
    // Check that intelligent selection metadata is not present or false
    if let Some(serde_json::Value::Bool(used)) = plan.metadata.get("intelligent_selection_used") {
        assert!(!*used, "Basic planner should not use intelligent selection");
    }
}

/// Test performance of intelligent tool selection
#[tokio::test]
async fn test_intelligent_selection_performance() {
    let security_config = SecurityConfig::default();
    let tool_registry = Arc::new(SecureToolRegistry::new(security_config));
    
    let planner = Planner::with_intelligent_tool_selection(tool_registry)
        .expect("Failed to create planner with intelligent tool selection");
    
    let intent = Intent {
        id: Uuid::new_v4(),
        intent_type: IntentType::ExecuteTool {
            tool_name: "performance_test_tool".to_string(),
        },
        parameters: HashMap::new(),
        confidence: 0.9,
        context: IntentContext {
            session_id: Uuid::new_v4(),
            user_id: Some("test_user".to_string()),
            timestamp: Utc::now(),
            environment: HashMap::new(),
            conversation_history: vec![],
        },
    };
    
    let start_time = std::time::Instant::now();
    let plan = planner.build_plan(&intent).await;
    let elapsed = start_time.elapsed();
    
    assert!(plan.is_ok(), "Performance test should succeed");
    assert!(elapsed.as_millis() < 200, "Plan building with intelligent tool selection should complete within 200ms for empty registry");
}

/// Test that plan validation still works with intelligent tool selection
#[tokio::test]
async fn test_plan_validation_with_intelligent_selection() {
    let security_config = SecurityConfig::default();
    let tool_registry = Arc::new(SecureToolRegistry::new(security_config));
    
    let planner = Planner::with_intelligent_tool_selection(tool_registry)
        .expect("Failed to create planner with intelligent tool selection");
    
    let intent = Intent {
        id: Uuid::new_v4(),
        intent_type: IntentType::ExecuteTool {
            tool_name: "validation_test_tool".to_string(),
        },
        parameters: HashMap::new(),
        confidence: 0.8,
        context: IntentContext {
            session_id: Uuid::new_v4(),
            user_id: Some("test_user".to_string()),
            timestamp: Utc::now(),
            environment: HashMap::new(),
            conversation_history: vec![],
        },
    };
    
    let plan = planner.build_plan(&intent).await;
    assert!(plan.is_ok(), "Plan building should succeed");
    
    let plan = plan.unwrap();
    let validation = planner.validate_plan(&plan).await;
    assert!(validation.is_ok(), "Plan validation should work with intelligent selection");
    
    let validation_result = validation.unwrap();
    // Since tools don't exist in registry, validation should report missing tools
    assert!(!validation_result.is_valid, "Validation should detect missing tools");
    assert!(!validation_result.errors.is_empty(), "Validation should report errors for missing tools");
}