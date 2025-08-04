use llm::agents::*;
use llm::{LlmClient, LlmProvider};
use mockito::Server;
use std::collections::HashMap;

#[tokio::test]
async fn test_tool_selector_agent_creation() {
    let mock_client = create_mock_client().await;
    let agent = ToolSelectorAgent::new(mock_client);
    
    // Agent should be created successfully
    // No public methods to test except select_tool
}

#[tokio::test]
async fn test_tool_selector_simple_commands() {
    let mut server = Server::new_async().await;
    
    let mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"tool_name\": \"file_read\",\n\"confidence\": 0.9,\n\"reasoning\": \"User wants to read a file\"\n}"
                }
            }]
        }"#)
        .create_async()
        .await;
    
    let client = create_mock_client_with_server(&server).await;
    let agent = ToolSelectorAgent::new(client);
    
    let available_tools = vec!["file_read".to_string(), "file_write".to_string(), "shell_exec".to_string()];
    let selection = agent.select_tool("read file config.json", &available_tools).await;
    assert!(selection.is_ok());
    
    let tool_selection = selection.unwrap();
    assert_eq!(tool_selection.tool_name, "file_read");
    assert!(tool_selection.confidence > 0.8);
    assert!(!tool_selection.reasoning.is_empty());
    
    mock.assert_async().await;
}

#[tokio::test]
async fn test_tool_selector_complex_queries() {
    let mut server = Server::new_async().await;
    
    let mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"tool_name\": \"shell_exec\",\n\"confidence\": 0.85,\n\"reasoning\": \"User wants to execute system commands\"\n}"
                }
            }]
        }"#)
        .create_async()
        .await;
    
    let client = create_mock_client_with_server(&server).await;
    let agent = ToolSelectorAgent::new(client);
    
    let available_tools = vec!["file_read".to_string(), "shell_exec".to_string(), "web_search".to_string()];
    let selection = agent.select_tool("find all Python files in the current directory and count lines of code", &available_tools).await;
    assert!(selection.is_ok());
    
    let tool_selection = selection.unwrap();
    assert_eq!(tool_selection.tool_name, "shell_exec");
    assert!(tool_selection.confidence > 0.8);
    
    mock.assert_async().await;
}

#[tokio::test]
async fn test_parameter_extractor_agent_creation() {
    let mock_client = create_mock_client().await;
    let agent = ParameterExtractorAgent::new(mock_client);
    
    // Agent should be created successfully
}

#[tokio::test]
async fn test_parameter_extractor_file_operations() {
    let mut server = Server::new_async().await;
    
    let mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"parameters\": {\"path\": \"config.json\"},\n\"confidence\": 0.95,\n\"missing_params\": []\n}"
                }
            }]
        }"#)
        .create_async()
        .await;
    
    let client = create_mock_client_with_server(&server).await;
    let agent = ParameterExtractorAgent::new(client);
    
    let required_params = vec!["path".to_string()];
    let extraction = agent.extract_parameters("read file config.json", "file_read", &required_params).await;
    assert!(extraction.is_ok());
    
    let param_extraction = extraction.unwrap();
    assert!(param_extraction.parameters.contains_key("path"));
    assert_eq!(param_extraction.parameters["path"], "config.json");
    assert!(param_extraction.confidence > 0.9);
    assert!(param_extraction.missing_params.is_empty());
    
    mock.assert_async().await;
}

#[tokio::test]
async fn test_parameter_extractor_shell_commands() {
    let mut server = Server::new_async().await;
    
    let mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"parameters\": {\"command\": \"dir\"}, \n\"confidence\": 0.9,\n\"missing_params\": []\n}"
                }
            }]
        }"#)
        .create_async()
        .await;
    
    let client = create_mock_client_with_server(&server).await;
    let agent = ParameterExtractorAgent::new(client);
    
    let required_params = vec!["command".to_string()];
    let extraction = agent.extract_parameters("list all files with details", "shell_exec", &required_params).await;
    assert!(extraction.is_ok());
    
    let param_extraction = extraction.unwrap();
    assert!(param_extraction.parameters.contains_key("command"));
    assert!(param_extraction.parameters["command"].contains("dir"));
    
    mock.assert_async().await;
}

#[tokio::test]
async fn test_parameter_extractor_missing_parameters() {
    let mut server = Server::new_async().await;
    
    let mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"parameters\": {\"command\": \"mkdir\"}, \n\"confidence\": 0.6,\n\"missing_params\": [\"directory_name\"]\n}"
                }
            }]
        }"#)
        .create_async()
        .await;
    
    let client = create_mock_client_with_server(&server).await;
    let agent = ParameterExtractorAgent::new(client);
    
    let required_params = vec!["command".to_string(), "directory_name".to_string()];
    let extraction = agent.extract_parameters("create a directory", "shell_exec", &required_params).await;
    assert!(extraction.is_ok());
    
    let param_extraction = extraction.unwrap();
    assert!(param_extraction.parameters.contains_key("command"));
    assert!(!param_extraction.missing_params.is_empty());
    assert!(param_extraction.confidence < 0.8);
    
    mock.assert_async().await;
}

#[tokio::test]
async fn test_intent_analyzer_agent_creation() {
    let mock_client = create_mock_client().await;
    let agent = IntentAnalyzerAgent::new(mock_client);
    
    // Agent should be created successfully
}

#[tokio::test]
async fn test_intent_analyzer_chat_vs_tool() {
    let mut server = Server::new_async().await;
    
    // Test chat intent
    let chat_mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"action_type\": \"chat\",\n\"confidence\": 0.92,\n\"reasoning\": \"User is asking a conversational question\"\n}"
                }
            }]
        }"#)
        .create_async()
        .await;
    
    let client = create_mock_client_with_server(&server).await;
    let agent = IntentAnalyzerAgent::new(client);
    
    let decision = agent.analyze_intent("How are you today?").await;
    assert!(decision.is_ok());
    
    let intent_decision = decision.unwrap();
    assert_eq!(intent_decision.action_type, "chat");
    assert!(intent_decision.confidence > 0.9);
    
    chat_mock.assert_async().await;
}

#[tokio::test]
async fn test_intent_analyzer_tool_intent() {
    let mut server = Server::new_async().await;
    
    let tool_mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"action_type\": \"tools\",\n\"confidence\": 0.95,\n\"reasoning\": \"User wants to perform a file operation\"\n}"
                }
            }]
        }"#)
        .create_async()
        .await;
    
    let client = create_mock_client_with_server(&server).await;
    let agent = IntentAnalyzerAgent::new(client);
    
    let decision = agent.analyze_intent("delete the old log files").await;
    assert!(decision.is_ok());
    
    let intent_decision = decision.unwrap();
    assert_eq!(intent_decision.action_type, "tools");
    assert!(intent_decision.confidence > 0.9);
    
    tool_mock.assert_async().await;
}

#[tokio::test]
async fn test_action_planner_agent_creation() {
    let mock_client = create_mock_client().await;
    let agent = ActionPlannerAgent::new(mock_client);
    
    // Agent should be created successfully
}

#[tokio::test]
async fn test_action_planner_simple_task() {
    let mut server = Server::new_async().await;
    
    let mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"steps\": [{\"tool\": \"file_read\", \"description\": \"Read the config file\", \"parameters\": {\"path\": \"config.json\"}}],\n\"reasoning\": \"Simple file read operation\",\n\"confidence\": 0.95\n}"
                }
            }]
        }"#)
        .create_async()
        .await;
    
    let client = create_mock_client_with_server(&server).await;
    let agent = ActionPlannerAgent::new(client);
    
    let available_tools = vec!["file_read".to_string(), "file_write".to_string()];
    let plan = agent.create_plan("read the config file", &available_tools).await;
    assert!(plan.is_ok());
    
    let action_plan = plan.unwrap();
    assert_eq!(action_plan.steps.len(), 1);
    assert!(action_plan.confidence > 0.9);
    
    let step = &action_plan.steps[0];
    assert_eq!(step.tool, "file_read");
    assert!(step.parameters.contains_key("path"));
    
    mock.assert_async().await;
}

#[tokio::test]
async fn test_action_planner_complex_task() {
    let mut server = Server::new_async().await;
    
    let mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\n\"steps\": [{\"tool\": \"shell_exec\", \"description\": \"Find Python files\", \"parameters\": {\"command\": \"dir *.py /s\"}}, {\"tool\": \"file_write\", \"description\": \"Generate summary report\", \"parameters\": {\"path\": \"report.txt\", \"content\": \"Python files analysis\"}}],\n\"reasoning\": \"Multi-step plan for file analysis\",\n\"confidence\": 0.85\n}"
                }
            }]
        }"#)
        .create_async()
        .await;
    
    let client = create_mock_client_with_server(&server).await;
    let agent = ActionPlannerAgent::new(client);
    
    let available_tools = vec!["shell_exec".to_string(), "file_write".to_string(), "file_read".to_string()];
    let plan = agent.create_plan("analyze all Python files and create a report", &available_tools).await;
    assert!(plan.is_ok());
    
    let action_plan = plan.unwrap();
    assert!(action_plan.steps.len() >= 2);
    assert!(action_plan.confidence > 0.8);
    
    // Verify steps have required fields
    for step in &action_plan.steps {
        assert!(!step.tool.is_empty());
        assert!(!step.description.is_empty());
        assert!(!step.parameters.is_empty());
    }
    
    mock.assert_async().await;
}

#[tokio::test]
async fn test_agents_error_handling() {
    let mut server = Server::new_async().await;
    
    // Mock server error
    let error_mock = server.mock("POST", "/chat/completions")
        .with_status(500)
        .with_body("Internal Server Error")
        .expect(4)
        .create_async()
        .await;
    
    let client = create_mock_client_with_server(&server).await;
    
    // Test all agents handle errors gracefully
    let tool_selector = ToolSelectorAgent::new(client.clone());
    let available_tools = vec!["file_read".to_string()];
    let tool_result = tool_selector.select_tool("test query", &available_tools).await;
    assert!(tool_result.is_err());
    
    let param_extractor = ParameterExtractorAgent::new(client.clone());
    let required_params = vec!["path".to_string()];
    let param_result = param_extractor.extract_parameters("test", "file_read", &required_params).await;
    assert!(param_result.is_err());
    
    let intent_analyzer = IntentAnalyzerAgent::new(client.clone());
    let intent_result = intent_analyzer.analyze_intent("test").await;
    assert!(intent_result.is_err());
    
    let action_planner = ActionPlannerAgent::new(client);
    let plan_result = action_planner.create_plan("test", &available_tools).await;
    assert!(plan_result.is_err());
    
    error_mock.assert_async().await;
}

#[test]
fn test_agent_data_structures() {
    // Test ToolSelection
    let tool_selection = ToolSelection {
        tool_name: "file_read".to_string(),
        confidence: 0.95,
        reasoning: "User wants file operations".to_string(),
    };
    
    assert_eq!(tool_selection.tool_name, "file_read");
    assert_eq!(tool_selection.confidence, 0.95);
    
    // Test ParameterExtraction
    let mut params = HashMap::new();
    params.insert("path".to_string(), "config.json".to_string());
    
    let param_extraction = ParameterExtraction {
        parameters: params,
        confidence: 0.9,
        missing_params: vec!["operation".to_string()],
    };
    
    assert!(param_extraction.parameters.contains_key("path"));
    assert_eq!(param_extraction.missing_params.len(), 1);
    
    // Test IntentDecision
    let intent_decision = IntentDecision {
        action_type: "tools".to_string(),
        confidence: 0.88,
        reasoning: "Task-oriented request".to_string(),
    };
    
    assert_eq!(intent_decision.action_type, "tools");
    assert!(intent_decision.confidence > 0.8);
    
    // Test PlanStep
    let plan_step = PlanStep {
        tool: "file_read".to_string(),
        description: "Read file".to_string(),
        parameters: {
            let mut params = HashMap::new();
            params.insert("path".to_string(), "test.txt".to_string());
            params
        },
    };
    
    assert_eq!(plan_step.tool, "file_read");
    assert!(!plan_step.description.is_empty());
    assert!(plan_step.parameters.contains_key("path"));
    
    // Test ActionPlan
    let action_plan = ActionPlan {
        steps: vec![plan_step],
        reasoning: "Simple plan for testing".to_string(),
        confidence: 0.9,
    };
    
    assert_eq!(action_plan.steps.len(), 1);
    assert!(action_plan.confidence > 0.8);
}

#[tokio::test]
async fn test_json_parsing_edge_cases() {
    let mut server = Server::new_async().await;
    
    // Mock malformed JSON response
    let malformed_mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{'tool_name': 'file_read', 'confidence': 0.9, 'reasoning': 'Single quotes JSON'}"
                }
            }]
        }"#)
        .create_async()
        .await;
    
    let client = create_mock_client_with_server(&server).await;
    let agent = ToolSelectorAgent::new(client);
    
    let available_tools = vec!["file_read".to_string()];
    let selection = agent.select_tool("test", &available_tools).await;
    
    // Should handle malformed JSON gracefully (single quotes → double quotes)
    assert!(selection.is_ok());
    let tool_selection = selection.unwrap();
    assert_eq!(tool_selection.tool_name, "file_read");
    
    malformed_mock.assert_async().await;
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
                    "content": "{\n\"tool_name\": \"shell_exec\",\n\"confidence\": 0.85,\n\"reasoning\": \"System operation\"\n}"
                }
            }]
        }"#)
        .expect(2)
        .create_async()
        .await;
    
    let client = create_mock_client_with_server(&server).await;
    
    // Run multiple tool selector operations concurrently
    let tool_selector1 = ToolSelectorAgent::new(client.clone());
    let tool_selector2 = ToolSelectorAgent::new(client);
    
    let available_tools = vec!["shell_exec".to_string()];
    
    let (tool_result1, tool_result2) = tokio::join!(
        tool_selector1.select_tool("run system command", &available_tools),
        tool_selector2.select_tool("execute directory listing", &available_tools)
    );
    
    assert!(tool_result1.is_ok());
    assert!(tool_result2.is_ok());
    
    mock.assert_async().await;
}

// Helper functions
async fn create_mock_client() -> LlmClient {
    let provider = LlmProvider::Local {
        url: "http://localhost:8080".to_string(),
        model: "test-model".to_string(),
    };
    LlmClient::new(provider)
}

async fn create_mock_client_with_server(server: &Server) -> LlmClient {
    let provider = LlmProvider::Local {
        url: server.url(),
        model: "test-model".to_string(),
    };
    LlmClient::new(provider)
}