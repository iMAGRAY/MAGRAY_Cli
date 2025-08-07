use llm::{LlmClient, LlmProvider};
use router::{ActionPlan, PlannedAction, SmartRouter};
use std::collections::HashMap;

#[test]
fn test_smart_router_structure() {
    // Проверяем что SmartRouter корректно инициализируется
    let llm_client = LlmClient::new(
        LlmProvider::OpenAI {
            api_key: "test-api-key".to_string(),
            model: "gpt-4o-mini".to_string(),
        },
        1000,
        0.7,
    );

    let router = SmartRouter::new(llm_client);

    // Router должен иметь пустой tool registry по умолчанию
    // Это проверяется косвенно через попытку выполнения
    let plan = ActionPlan {
        reasoning: "Test".to_string(),
        confidence: 1.0,
        steps: vec![],
    };

    // Пустой план должен выполняться без ошибок
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(router.execute_plan(&plan));
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[test]
fn test_planned_action_args() {
    // Тест различных типов аргументов
    let mut args = HashMap::new();
    args.insert("string_arg".to_string(), "value".to_string());
    args.insert("number_arg".to_string(), "42".to_string());
    args.insert("bool_arg".to_string(), "true".to_string());
    args.insert("path_arg".to_string(), "/home/user/file.txt".to_string());

    let action = PlannedAction {
        tool: "complex_tool".to_string(),
        description: "Tool with multiple arg types".to_string(),
        args: args.clone(),
        expected_output: "Success".to_string(),
    };

    assert_eq!(action.args.len(), 4);
    assert_eq!(action.args.get("string_arg"), Some(&"value".to_string()));
    assert_eq!(action.args.get("number_arg"), Some(&"42".to_string()));
    assert_eq!(action.args.get("bool_arg"), Some(&"true".to_string()));
    assert_eq!(
        action.args.get("path_arg"),
        Some(&"/home/user/file.txt".to_string())
    );
}

#[test]
fn test_action_plan_confidence_levels() {
    // Тестируем разные уровни confidence
    let high_confidence = ActionPlan {
        reasoning: "Very clear task".to_string(),
        confidence: 0.95,
        steps: vec![],
    };

    let medium_confidence = ActionPlan {
        reasoning: "Somewhat clear task".to_string(),
        confidence: 0.75,
        steps: vec![],
    };

    let low_confidence = ActionPlan {
        reasoning: "Unclear task".to_string(),
        confidence: 0.45,
        steps: vec![],
    };

    assert!(high_confidence.confidence > 0.9);
    assert!(medium_confidence.confidence >= 0.7);
    assert!(low_confidence.confidence < 0.7); // Порог из кода
}

#[test]
fn test_extract_required_params_edge_cases() {
    let llm_client = LlmClient::new(
        LlmProvider::OpenAI {
            api_key: "test".to_string(),
            model: "test".to_string(),
        },
        1000,
        0.7,
    );

    let router = SmartRouter::new(llm_client);

    // Пустая схема
    let params = router.extract_required_params("{}");
    assert!(params.is_empty());

    // Null
    let params = router.extract_required_params("null");
    assert!(!params.is_empty()); // Должен вернуть fallback

    // Массив вместо объекта
    let params = router.extract_required_params("[]");
    assert!(!params.is_empty()); // Должен вернуть fallback

    // Некорректный JSON
    let params = router.extract_required_params("{invalid json");
    assert!(params.contains(&"path".to_string())); // Fallback params
}

#[test]
fn test_complex_action_plan() {
    // Тест сложного многошагового плана
    let plan = ActionPlan {
        reasoning: "Complex multi-step operation".to_string(),
        confidence: 0.85,
        steps: vec![
            PlannedAction {
                tool: "git_status".to_string(),
                description: "Check git status".to_string(),
                args: HashMap::new(),
                expected_output: "Git status information".to_string(),
            },
            PlannedAction {
                tool: "file_read".to_string(),
                description: "Read changed files".to_string(),
                args: HashMap::from([("path".to_string(), "src/main.rs".to_string())]),
                expected_output: "File contents".to_string(),
            },
            PlannedAction {
                tool: "git_commit".to_string(),
                description: "Commit changes".to_string(),
                args: HashMap::from([("message".to_string(), "Update main.rs".to_string())]),
                expected_output: "Commit successful".to_string(),
            },
        ],
    };

    assert_eq!(plan.steps.len(), 3);
    assert_eq!(plan.steps[0].tool, "git_status");
    assert_eq!(plan.steps[1].tool, "file_read");
    assert_eq!(plan.steps[2].tool, "git_commit");

    // Проверяем что у каждого шага есть описание
    for step in &plan.steps {
        assert!(!step.description.is_empty());
        assert!(!step.expected_output.is_empty());
    }
}

#[test]
fn test_format_results_unicode() {
    let llm_client = LlmClient::new(
        LlmProvider::OpenAI {
            api_key: "test".to_string(),
            model: "test".to_string(),
        },
        1000,
        0.7,
    );

    let router = SmartRouter::new(llm_client);

    let plan = ActionPlan {
        reasoning: "Тест с юникодом 🚀".to_string(),
        confidence: 0.9,
        steps: vec![PlannedAction {
            tool: "test".to_string(),
            description: "Тестовое действие ✨".to_string(),
            args: HashMap::new(),
            expected_output: "Результат 📝".to_string(),
        }],
    };

    let results = vec![tools::ToolOutput {
        success: true,
        result: "Успешно выполнено ✅".to_string(),
        formatted_output: Some("🎉 Операция завершена".to_string()),
        metadata: HashMap::new(),
    }];

    let formatted = router.format_results(&plan, &results).unwrap();

    assert!(formatted.contains("🚀"));
    assert!(formatted.contains("✨"));
    assert!(formatted.contains("🎉"));
}
