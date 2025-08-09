use anyhow::Result;
use async_trait::async_trait;
use llm::{LlmClient, LlmProvider};
use router::{ActionPlan, PlannedAction, SmartRouter};
use std::collections::HashMap;
use tools::{Tool, ToolInput, ToolOutput, ToolRegistry, ToolSpec};

// Mock tool для тестирования
struct MockTool {
    name: String,
    should_fail: bool,
}

#[async_trait]
impl Tool for MockTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name.clone(),
            description: "Mock tool for testing".to_string(),
            usage: "mock_tool <test_param>".to_string(),
            examples: vec!["mock_tool test".to_string()],
            input_schema: r#"{"test_param": "string"}"#.to_string(),
            usage_guide: None,
            permissions: None,
            supports_dry_run: false,
        }
    }

    async fn execute(&self, _input: ToolInput) -> Result<ToolOutput> {
        if self.should_fail {
            Err(anyhow::anyhow!("Mock tool failed"))
        } else {
            Ok(ToolOutput {
                success: true,
                result: "Mock result".to_string(),
                formatted_output: Some("✅ Mock executed".to_string()),
                metadata: HashMap::new(),
            })
        }
    }

    async fn parse_natural_language(&self, _query: &str) -> Result<ToolInput> {
        Ok(ToolInput {
            command: self.name.clone(),
            args: HashMap::from([("test_param".to_string(), "test_value".to_string())]),
            context: Some("Mock context".to_string()),
            dry_run: false,
            timeout_ms: None,
        })
    }
}

// Helper для создания тестового роутера с mock tools
async fn create_test_router_with_tools() -> (SmartRouter, ToolRegistry) {
    let llm_client = LlmClient::new(
        LlmProvider::OpenAI {
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
        },
        1000,
        0.7,
    );

    let mut registry = ToolRegistry::new();

    // Добавляем mock tools
    registry.register(
        "mock_success",
        Box::new(MockTool {
            name: "mock_success".to_string(),
            should_fail: false,
        }),
    );

    registry.register(
        "mock_fail",
        Box::new(MockTool {
            name: "mock_fail".to_string(),
            should_fail: true,
        }),
    );

    let router = SmartRouter::new(llm_client);
    (router, registry)
}

#[tokio::test]
async fn test_execute_plan_success() {
    let llm_client = LlmClient::new(
        LlmProvider::OpenAI {
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
        },
        1000,
        0.7,
    );

    let router = SmartRouter::new(llm_client);

    // Создаем план с несуществующим инструментом (должен провалиться)
    let plan = ActionPlan {
        reasoning: "Test execution".to_string(),
        confidence: 0.9,
        steps: vec![PlannedAction {
            tool: "nonexistent_tool".to_string(),
            description: "Try to use nonexistent tool".to_string(),
            args: HashMap::new(),
            expected_output: "Should fail".to_string(),
        }],
    };

    let result = router.execute_plan(&plan).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("не найден"));
}

#[tokio::test]
async fn test_execute_plan_empty() {
    let llm_client = LlmClient::new(
        LlmProvider::OpenAI {
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
        },
        1000,
        0.7,
    );

    let router = SmartRouter::new(llm_client);

    // Пустой план должен выполниться успешно
    let plan = ActionPlan {
        reasoning: "Empty plan".to_string(),
        confidence: 1.0,
        steps: vec![],
    };

    let result = router.execute_plan(&plan).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
async fn test_analyze_and_plan_conversion() {
    // Этот тест проверяет что analyze_and_plan корректно конвертирует
    // результат из ActionPlannerAgent в наш ActionPlan
    let llm_client = LlmClient::new(
        LlmProvider::OpenAI {
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
        },
        1000,
        0.7,
    );

    let router = SmartRouter::new(llm_client);

    // Это интеграционный тест, который требует реального LLM
    // Поэтому мы просто проверяем что метод не паникует
    // В реальном тесте нужен mock LLM client
    let query = "test query";
    let _result = router.analyze_and_plan(query).await;
    // Не проверяем результат, так как требуется реальный API ключ
}

#[tokio::test]
async fn test_process_single_tool_request_no_tools() {
    let llm_client = LlmClient::new(
        LlmProvider::OpenAI {
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
        },
        1000,
        0.7,
    );

    let router = SmartRouter::new(llm_client);

    // Без инструментов должна быть ошибка
    let result = router.process_single_tool_request("do something").await;

    // Ожидаем ошибку, так как нет доступных инструментов
    assert!(result.is_err());
}

#[tokio::test]
async fn test_process_smart_request_simple() {
    let llm_client = LlmClient::new(
        LlmProvider::OpenAI {
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
        },
        1000,
        0.7,
    );

    let router = SmartRouter::new(llm_client);

    // Тест с простым запросом
    let result = router.process_smart_request("read file test.txt").await;

    // Ожидаем ошибку из-за отсутствия реального API ключа
    assert!(result.is_err());
}

#[test]
fn test_extract_required_params_complex_schema() {
    let llm_client = LlmClient::new(
        LlmProvider::OpenAI {
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
        },
        1000,
        0.7,
    );

    let router = SmartRouter::new(llm_client);

    // Тест с вложенной JSON схемой
    let schema = r#"{
        "file": {"type": "string", "required": true},
        "options": {
            "encoding": "utf-8",
            "mode": "read"
        }
    }"#;

    let params = router.extract_required_params(schema);

    // Должны извлечь ключи верхнего уровня
    assert!(params.contains(&"file".to_string()));
    assert!(params.contains(&"options".to_string()));
}

#[test]
fn test_format_results_mixed_success() {
    let llm_client = LlmClient::new(
        LlmProvider::OpenAI {
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
        },
        1000,
        0.7,
    );

    let router = SmartRouter::new(llm_client);

    let plan = ActionPlan {
        reasoning: "Mixed results test".to_string(),
        confidence: 0.8,
        steps: vec![
            PlannedAction {
                tool: "tool1".to_string(),
                description: "First action".to_string(),
                args: HashMap::new(),
                expected_output: "Success".to_string(),
            },
            PlannedAction {
                tool: "tool2".to_string(),
                description: "Second action".to_string(),
                args: HashMap::new(),
                expected_output: "Failure".to_string(),
            },
        ],
    };

    let results = vec![
        ToolOutput {
            success: true,
            result: "Success".to_string(),
            formatted_output: None,
            metadata: HashMap::new(),
        },
        ToolOutput {
            success: false,
            result: "Failed to execute".to_string(),
            formatted_output: None,
            metadata: HashMap::new(),
        },
    ];

    let formatted = router.format_results(&plan, &results).unwrap();

    assert!(formatted.contains("Mixed results test"));
    assert!(formatted.contains("[✓]")); // Success marker
    assert!(formatted.contains("[✗]")); // Failure marker
    assert!(formatted.contains("First action"));
    assert!(formatted.contains("Second action"));
}
