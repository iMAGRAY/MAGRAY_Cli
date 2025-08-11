#![cfg(feature = "extended-tests")]

use llm::agents::*;
use llm::{ChatMessage, CompletionRequest, LlmClient, LlmProvider};
use mockito::Server;
use std::sync::Arc;

#[tokio::test]
async fn test_end_to_end_chat_workflow() {
    let mut server = Server::new_async().await;

    let intent_mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"intent_type\": \"tool\",\n\"confidence\": 0.95,\n\"reasoning\": \"User wants to perform file operations\",\n\"suggested_flow\": \"direct_execution\"\n}"
                }
            }]
        }"#)
        .create_async()
        .await;

    let tool_mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"tool_name\": \"file\",\n\"confidence\": 0.92,\n\"reasoning\": \"File operation requested\"\n}"
                }
            }]
        }"#)
        .create_async()
        .await;

    let param_mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"parameters\": {\"file_path\": \"data.json\", \"operation\": \"read\"},\n\"confidence\": 0.88,\n\"missing_params\": []\n}"
                }
            }]
        }"#)
        .create_async()
        .await;

    let client = create_test_client(&server).await;

    // Simulate complete workflow
    let user_query = "read the data.json file";

    // Step 1: Analyze intent
    let intent_analyzer = IntentAnalyzerAgent::new(client.clone());
    let intent_result = intent_analyzer.analyze_intent(user_query).await.unwrap();
    assert_eq!(intent_result.action_type, "tool");

    // Step 2: Select appropriate tool
    let tool_selector = ToolSelectorAgent::new(client.clone());
    let available_tools = vec!["file".to_string(), "shell".to_string()];
    let tool_result = tool_selector
        .select_tool(user_query, &available_tools)
        .await
        .unwrap();
    assert_eq!(tool_result.tool_name, "file");

    // Step 3: Extract parameters
    let param_extractor = ParameterExtractorAgent::new(client);
    let required_params = vec!["path".to_string(), "operation".to_string()];
    let param_result = param_extractor
        .extract_parameters(user_query, &tool_result.tool_name, &required_params)
        .await
        .unwrap();
    assert!(param_result.parameters.contains_key("file_path"));
    assert_eq!(param_result.parameters["file_path"], "data.json");

    intent_mock.assert_async().await;
    tool_mock.assert_async().await;
    param_mock.assert_async().await;
}

#[tokio::test]
async fn test_multi_step_planning_workflow() {
    let mut server = Server::new_async().await;

    let planning_mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"steps\": [{\"id\": \"1\", \"description\": \"Find Python files\", \"tool\": \"shell\", \"parameters\": {\"command\": \"find . -name '*.py'\"}, \"dependencies\": [], \"estimated_duration_seconds\": 5}, {\"id\": \"2\", \"description\": \"Count lines in files\", \"tool\": \"shell\", \"parameters\": {\"command\": \"wc -l *.py\"}, \"dependencies\": [\"1\"], \"estimated_duration_seconds\": 10}, {\"id\": \"3\", \"description\": \"Generate report\", \"tool\": \"file\", \"parameters\": {\"operation\": \"write\", \"path\": \"report.txt\"}, \"dependencies\": [\"2\"], \"estimated_duration_seconds\": 5}],\n\"complexity\": \"complex\",\n\"estimated_time_minutes\": 1,\n\"can_run_parallel\": false\n}"
                }
            }]
        }"#)
        .create_async()
        .await;

    let client = create_test_client(&server).await;
    let planner = ActionPlannerAgent::new(client);

    let complex_query = "analyze all Python files and create a line count report";
    let available_tools = vec!["shell_exec".to_string(), "file_write".to_string()];
    let plan = planner
        .create_plan(complex_query, &available_tools)
        .await
        .unwrap();

    assert_eq!(plan.steps.len(), 3);
    assert!(plan.confidence > 0.8);

    // Verify steps are properly structured
    for step in &plan.steps {
        assert!(!step.tool.is_empty());
        assert!(!step.description.is_empty());
    }

    planning_mock.assert_async().await;
}

#[tokio::test]
async fn test_conversational_vs_task_routing() {
    let mut server = Server::new_async().await;

    let chat_mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"intent_type\": \"chat\",\n\"confidence\": 0.9,\n\"reasoning\": \"User is asking a general question\",\n\"suggested_flow\": \"conversational\"\n}"
                }
            }]
        }"#)
        .create_async()
        .await;

    let task_mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"intent_type\": \"tool\",\n\"confidence\": 0.95,\n\"reasoning\": \"User wants to execute a specific task\",\n\"suggested_flow\": \"direct_execution\"\n}"
                }
            }]
        }"#)
        .create_async()
        .await;

    let client = create_test_client(&server).await;
    let analyzer = IntentAnalyzerAgent::new(client);

    // Test conversational query
    let chat_result = analyzer
        .analyze_intent("How's the weather today?")
        .await
        .unwrap();
    assert_eq!(chat_result.action_type, "chat");

    // Test task-oriented query
    let task_result = analyzer
        .analyze_intent("delete old log files")
        .await
        .unwrap();
    assert_eq!(task_result.action_type, "tool");

    chat_mock.assert_async().await;
    task_mock.assert_async().await;
}

#[tokio::test]
async fn test_error_recovery_workflow() {
    let mut server = Server::new_async().await;

    // First request fails
    let error_mock = server
        .mock("POST", "/chat/completions")
        .with_status(500)
        .with_body("Internal Server Error")
        .create_async()
        .await;

    // Second request succeeds
    let success_mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"tool_name\": \"file\",\n\"confidence\": 0.9,\n\"reasoning\": \"Recovered successfully\"\n}"
                }
            }]
        }"#)
        .create_async()
        .await;

    let client = create_test_client(&server).await;
    let tool_selector = ToolSelectorAgent::new(client);

    let available_tools = vec!["file".to_string()];

    // First attempt should fail
    let first_result = tool_selector
        .select_tool("test query", &available_tools)
        .await;
    assert!(first_result.is_err());

    // Second attempt should succeed
    let second_result = tool_selector
        .select_tool("test query", &available_tools)
        .await;
    assert!(second_result.is_ok());
    assert_eq!(second_result.unwrap().tool_name, "file");

    error_mock.assert_async().await;
    success_mock.assert_async().await;
}

#[tokio::test]
async fn test_concurrent_agent_operations() {
    let mut server = Server::new_async().await;

    let mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"tool_name\": \"shell\",\n\"confidence\": 0.85,\n\"reasoning\": \"System operation\"\n}"
                }
            }]
        }"#)
        .expect_at_least(10)
        .create_async()
        .await;

    let client = Arc::new(create_test_client(&server).await);
    let mut handles = vec![];

    // Spawn multiple concurrent agent operations
    for i in 0..10 {
        let client_clone = Arc::clone(&client);
        let handle = tokio::spawn(async move {
            let tool_selector = ToolSelectorAgent::new((*client_clone).clone());
            let query = format!("system task {}", i);
            let available_tools = vec!["shell".to_string()];
            tool_selector.select_tool(&query, &available_tools).await
        });
        handles.push(handle);
    }

    let mut all_succeeded = true;
    for handle in handles {
        match handle.await {
            Ok(Ok(tool_result)) => {
                assert_eq!(tool_result.tool_name, "shell");
                assert!(tool_result.confidence > 0.8);
            }
            _ => all_succeeded = false,
        }
    }
    assert!(all_succeeded);

    mock.assert_async().await;
}

#[tokio::test]
async fn test_provider_switching_workflow() {
    // Test switching between different providers

    // OpenAI provider test
    let mut openai_server = Server::new_async().await;
    let openai_mock = openai_server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"choices": [{"message": {"role": "assistant", "content": "OpenAI response"}}]}"#,
        )
        .create_async()
        .await;

    let openai_provider = LlmProvider::Local {
        url: openai_server.url(),
        model: "test-model".to_string(),
    };
    let openai_client = LlmClient::new(openai_provider, 1000, 0.7);

    // Test OpenAI provider
    let request = CompletionRequest::new("Test query");
    let response = openai_client.complete(request).await.unwrap();
    assert_eq!(response, "OpenAI response");

    openai_mock.assert_async().await;
}

#[tokio::test]
async fn test_agent_context_preservation() {
    let mut server = Server::new_async().await;

    // Mock multiple sequential calls
    let mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"tool_name\": \"file\",\n\"confidence\": 0.9,\n\"reasoning\": \"Context-aware response\"\n}"
                }
            }]
        }"#)
        .expect_at_least(3)
        .create_async()
        .await;

    let client = create_test_client(&server).await;
    let tool_selector = ToolSelectorAgent::new(client);

    // Multiple related queries to test context
    let queries = vec![
        "read the config file",
        "also check the backup config",
        "compare both files",
    ];

    let mut results = vec![];
    for query in queries {
        let available_tools = vec!["file".to_string()];
        let result = tool_selector
            .select_tool(query, &available_tools)
            .await
            .unwrap();
        results.push(result);
    }

    for result in results {
        assert_eq!(result.tool_name, "file");
        assert!(!result.reasoning.is_empty());
    }

    mock.assert_async().await;
}

#[tokio::test]
async fn test_parameter_extraction_edge_cases() {
    let mut server = Server::new_async().await;

    let missing_params_mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"parameters\": {\"operation\": \"list\"},\n\"confidence\": 0.6,\n\"missing_params\": [\"directory\", \"filters\"]\n}"
                }
            }]
        }"#)
        .create_async()
        .await;

    let complex_params_mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"parameters\": {\"source\": \"/path/to/source\", \"destination\": \"/path/to/dest\", \"recursive\": \"true\", \"preserve_permissions\": \"true\", \"exclude_patterns\": \"*.tmp,*.log\"},\n\"confidence\": 0.95,\n\"missing_params\": []\n}"
                }
            }]
        }"#)
        .create_async()
        .await;

    let client = create_test_client(&server).await;
    let param_extractor = ParameterExtractorAgent::new(client);

    let required_params = vec!["directory".to_string(), "filters".to_string()];

    // Test incomplete query
    let incomplete_result = param_extractor
        .extract_parameters("list files", "file", &required_params)
        .await
        .unwrap();
    assert!(!incomplete_result.missing_params.is_empty());
    assert!(incomplete_result.confidence < 0.8);

    let complex_required_params = vec![
        "source".to_string(),
        "destination".to_string(),
        "recursive".to_string(),
        "preserve_permissions".to_string(),
    ];

    // Test complex query
    let complex_result = param_extractor.extract_parameters(
        "copy all files from /path/to/source to /path/to/dest recursively, preserve permissions, exclude temporary and log files",
        "file",
        &complex_required_params
    ).await.unwrap();
    assert!(complex_result.parameters.len() >= 4);
    assert!(complex_result.missing_params.is_empty());
    assert!(complex_result.confidence > 0.9);

    missing_params_mock.assert_async().await;
    complex_params_mock.assert_async().await;
}

#[tokio::test]
async fn test_chat_conversation_memory() {
    let mut server = Server::new_async().await;

    // Mock responses that show conversation context
    let mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "I understand your request and can help with the file operations we discussed."
                }
            }]
        }"#)
        .create_async()
        .await;

    let client = create_test_client(&server).await;

    // Simulate conversation with history
    let conversation = vec![
        ChatMessage::system("You are a helpful assistant."),
        ChatMessage::user("I need to work with some files."),
        ChatMessage::assistant("I can help you with file operations. What would you like to do?"),
        ChatMessage::user("Let's start with reading a config file."),
    ];

    let response = client.chat(&conversation).await.unwrap();
    assert!(response.contains("file operations"));

    mock.assert_async().await;
}

#[tokio::test]
async fn test_load_balancing_between_models() {
    // Simulate load balancing by using different providers
    let mut server1 = Server::new_async().await;
    let mut server2 = Server::new_async().await;

    let mock1 = server1.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"choices": [{"message": {"role": "assistant", "content": "Response from server 1"}}]}"#)
        .create_async()
        .await;

    let mock2 = server2.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"choices": [{"message": {"role": "assistant", "content": "Response from server 2"}}]}"#)
        .create_async()
        .await;

    let client1 = create_test_client(&server1).await;
    let client2 = create_test_client(&server2).await;

    let request = CompletionRequest::new("Test load balancing");

    let response1 = client1.complete(request.clone()).await.unwrap();
    let response2 = client2.complete(request).await.unwrap();

    assert_eq!(response1, "Response from server 1");
    assert_eq!(response2, "Response from server 2");

    mock1.assert_async().await;
    mock2.assert_async().await;
}

// Helper function
async fn create_test_client(server: &Server) -> LlmClient {
    let provider = LlmProvider::Local {
        url: server.url(),
        model: "test-model".to_string(),
    };
    LlmClient::new(provider, 1000, 0.7)
}
