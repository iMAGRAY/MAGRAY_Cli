use router::{SmartRouter, ActionPlan, PlannedAction};
use llm::{LlmClient, LlmProvider};
use std::collections::HashMap;
use tokio;

// Helper функция для создания тестового LLM клиента
fn create_test_llm_client() -> LlmClient {
    LlmClient::new(
        LlmProvider::OpenAI {
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
        },
        500,   // max_tokens
        0.7    // temperature
    )
}

#[test]
fn test_action_plan_creation() {
    let plan = ActionPlan {
        reasoning: "Need to read and process a file".to_string(),
        confidence: 0.85,
        steps: vec![
            PlannedAction {
                tool: "file_read".to_string(),
                description: "Read configuration file".to_string(),
                args: HashMap::from([
                    ("path".to_string(), "config.json".to_string()),
                ]),
                expected_output: "File contents as JSON".to_string(),
            },
            PlannedAction {
                tool: "json_parse".to_string(),
                description: "Parse JSON configuration".to_string(),
                args: HashMap::new(),
                expected_output: "Parsed configuration".to_string(),
            },
        ],
    };
    
    assert_eq!(plan.reasoning, "Need to read and process a file");
    assert_eq!(plan.confidence, 0.85);
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0].tool, "file_read");
    assert_eq!(plan.steps[1].tool, "json_parse");
}

#[test]
fn test_planned_action_structure() {
    let mut args = HashMap::new();
    args.insert("path".to_string(), "/home/test.txt".to_string());
    args.insert("encoding".to_string(), "utf-8".to_string());
    
    let action = PlannedAction {
        tool: "file_read".to_string(),
        description: "Read test file".to_string(),
        args,
        expected_output: "File contents".to_string(),
    };
    
    assert_eq!(action.tool, "file_read");
    assert_eq!(action.description, "Read test file");
    assert_eq!(action.args.get("path"), Some(&"/home/test.txt".to_string()));
    assert_eq!(action.args.get("encoding"), Some(&"utf-8".to_string()));
    assert_eq!(action.expected_output, "File contents");
}

#[test]
fn test_smart_router_creation() {
    let llm_client = create_test_llm_client();
    let router = SmartRouter::new(llm_client);
    
    // Router должен успешно создаться
    // Проверяем через попытку использования
    let _ = router; // Просто проверяем что создается без паники
}

#[test]
fn test_action_plan_serialization() {
    let plan = ActionPlan {
        reasoning: "Test serialization".to_string(),
        confidence: 0.95,
        steps: vec![
            PlannedAction {
                tool: "test_tool".to_string(),
                description: "Test action".to_string(),
                args: HashMap::from([
                    ("key1".to_string(), "value1".to_string()),
                    ("key2".to_string(), "value2".to_string()),
                ]),
                expected_output: "Test output".to_string(),
            },
        ],
    };
    
    // Сериализация в JSON
    let json = serde_json::to_string(&plan).unwrap();
    assert!(json.contains("Test serialization"));
    assert!(json.contains("0.95"));
    assert!(json.contains("test_tool"));
    
    // Десериализация обратно
    let deserialized: ActionPlan = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.reasoning, plan.reasoning);
    assert_eq!(deserialized.confidence, plan.confidence);
    assert_eq!(deserialized.steps.len(), plan.steps.len());
}

#[test]
fn test_empty_action_plan() {
    let plan = ActionPlan {
        reasoning: "No actions needed".to_string(),
        confidence: 1.0,
        steps: vec![],
    };
    
    assert_eq!(plan.steps.len(), 0);
    assert!(plan.steps.is_empty());
}

#[test]
fn test_low_confidence_plan() {
    let plan = ActionPlan {
        reasoning: "Uncertain about the approach".to_string(),
        confidence: 0.3,
        steps: vec![
            PlannedAction {
                tool: "maybe_this".to_string(),
                description: "Try this approach".to_string(),
                args: HashMap::new(),
                expected_output: "Some result".to_string(),
            },
        ],
    };
    
    assert!(plan.confidence < 0.5);
    assert!(plan.confidence < 0.7); // Threshold used in code
}

#[tokio::test]
async fn test_extract_required_params() {
    let llm_client = create_test_llm_client();
    let router = SmartRouter::new(llm_client);
    
    // Test with valid JSON schema
    let schema = r#"{"path": "string", "content": "string", "append": "boolean"}"#;
    let params = router.extract_required_params(schema);
    
    assert!(params.contains(&"path".to_string()));
    assert!(params.contains(&"content".to_string()));
    assert!(params.contains(&"append".to_string()));
}

#[test]
fn test_extract_required_params_invalid_json() {
    let llm_client = create_test_llm_client();
    let router = SmartRouter::new(llm_client);
    
    // Test with invalid JSON - should return fallback params
    let schema = "not a json";
    let params = router.extract_required_params(schema);
    
    // Should contain fallback parameters
    assert!(params.contains(&"path".to_string()));
    assert!(params.contains(&"command".to_string()));
    assert!(params.contains(&"query".to_string()));
}

#[test]
fn test_format_results_empty() {
    let llm_client = create_test_llm_client();
    let router = SmartRouter::new(llm_client);
    
    let plan = ActionPlan {
        reasoning: "Empty plan".to_string(),
        confidence: 1.0,
        steps: vec![],
    };
    
    let results = vec![];
    let formatted = router.format_results(&plan, &results).unwrap();
    
    assert!(formatted.contains("Empty plan"));
    assert!(formatted.contains("0 действий"));
}

#[test]
fn test_format_results_with_data() {
    use tools::ToolOutput;
    
    let llm_client = create_test_llm_client();
    let router = SmartRouter::new(llm_client);
    
    let plan = ActionPlan {
        reasoning: "Execute test plan".to_string(),
        confidence: 0.9,
        steps: vec![
            PlannedAction {
                tool: "test_tool".to_string(),
                description: "First test action".to_string(),
                args: HashMap::new(),
                expected_output: "Success".to_string(),
            },
        ],
    };
    
    let results = vec![
        ToolOutput {
            success: true,
            result: "Operation completed".to_string(),
            formatted_output: Some("✅ Operation completed successfully".to_string()),
            metadata: HashMap::new(),
        },
    ];
    
    let formatted = router.format_results(&plan, &results).unwrap();
    
    assert!(formatted.contains("Execute test plan"));
    assert!(formatted.contains("First test action"));
    assert!(formatted.contains("Operation completed successfully"));
}