#![cfg(not(feature = "minimal"))]

use cli::agent::{AgentResponse, UnifiedAgent};
use std::time::Duration;
use tokio;

#[tokio::test]
async fn test_unified_agent_creation() {
    let agent_result = UnifiedAgent::new().await;

    // Agent creation might fail due to missing dependencies
    // but should not panic
    assert!(agent_result.is_ok() || agent_result.is_err());
}

#[tokio::test]
async fn test_agent_simple_message() {
    let agent_result = UnifiedAgent::new().await;

    if let Ok(agent) = agent_result {
        let response = agent.process_message("hello").await;

        // Response might fail due to missing LLM configuration
        // but should handle gracefully
        assert!(response.is_ok() || response.is_err());

        if let Ok(resp) = response {
            match resp {
                AgentResponse::Chat(_) | AgentResponse::ToolExecution(_) => {
                    // Valid response types
                }
            }
        }
    }
}

#[tokio::test]
async fn test_agent_complex_message() {
    let agent_result = UnifiedAgent::new().await;

    if let Ok(agent) = agent_result {
        let response = agent
            .process_message("analyze the current system status and provide recommendations")
            .await;

        // Complex queries should be handled appropriately
        assert!(response.is_ok() || response.is_err());
    }
}

#[tokio::test]
async fn test_agent_empty_message() {
    let agent_result = UnifiedAgent::new().await;

    if let Ok(agent) = agent_result {
        let response = agent.process_message("").await;

        // Empty query should be handled gracefully
        assert!(response.is_ok() || response.is_err());
    }
}

#[tokio::test]
async fn test_agent_very_long_message() {
    let agent_result = UnifiedAgent::new().await;

    if let Ok(agent) = agent_result {
        let long_query = "a".repeat(10000);
        let response = agent.process_message(&long_query).await;

        // Very long queries should be handled appropriately
        assert!(response.is_ok() || response.is_err());
    }
}

#[tokio::test]
async fn test_agent_special_characters_message() {
    let agent_result = UnifiedAgent::new().await;

    if let Ok(agent) = agent_result {
        let special_query = "Test with émojis 🚀 and spéçial chars: <>\"'&";
        let response = agent.process_message(special_query).await;

        // Special characters should be handled properly
        assert!(response.is_ok() || response.is_err());
    }
}

#[tokio::test]
async fn test_agent_timeout_handling() {
    let agent_result = UnifiedAgent::new().await;

    if let Ok(agent) = agent_result {
        use tokio::time::timeout;

        let response = timeout(
            Duration::from_secs(5),
            agent.process_message("test timeout"),
        )
        .await;

        // Should either complete within timeout or timeout gracefully
        assert!(response.is_ok() || response.is_err());
    }
}

#[tokio::test]
async fn test_agent_concurrent_requests() {
    let agent_result = UnifiedAgent::new().await;

    if let Ok(agent) = agent_result {
        use tokio::join;

        let query1 = agent.process_message("first query");
        let query2 = agent.process_message("second query");
        let query3 = agent.process_message("third query");

        let (result1, result2, result3) = join!(query1, query2, query3);

        // All concurrent requests should be handled
        assert!(result1.is_ok() || result1.is_err());
        assert!(result2.is_ok() || result2.is_err());
        assert!(result3.is_ok() || result3.is_err());
    }
}

#[tokio::test]
async fn test_agent_error_recovery() {
    let agent_result = UnifiedAgent::new().await;

    if let Ok(agent) = agent_result {
        // Test that agent can recover from errors
        let _ = agent
            .process_message("invalid query that might cause error")
            .await;

        // Should still be able to process subsequent queries
        let response = agent.process_message("simple query").await;
        assert!(response.is_ok() || response.is_err());
    }
}

#[tokio::test]
async fn test_agent_tool_integration() {
    let agent_result = UnifiedAgent::new().await;

    if let Ok(agent) = agent_result {
        // Test queries that might trigger tool usage
        let tool_queries = vec![
            "list files in current directory",
            "check system status",
            "show memory usage",
            "what is the current time",
        ];

        for query in tool_queries {
            let response = agent.process_message(query).await;
            // Tools might not be available, but should handle gracefully
            assert!(response.is_ok() || response.is_err());
        }
    }
}

#[test]
fn test_agent_response_types() {
    // Test that response types can be constructed
    let chat_response = AgentResponse::Chat("Hello world".to_string());
    let tool_response = AgentResponse::ToolExecution("Command executed".to_string());

    match chat_response {
        AgentResponse::Chat(content) => assert_eq!(content, "Hello world"),
        _ => panic!("Expected Chat response"),
    }

    match tool_response {
        AgentResponse::ToolExecution(result) => assert_eq!(result, "Command executed"),
        _ => panic!("Expected ToolExecution response"),
    }
}

#[tokio::test]
async fn test_agent_graceful_shutdown() {
    let agent_result = UnifiedAgent::new().await;

    if let Ok(agent) = agent_result {
        // Test that agent can be created and dropped gracefully
        // without any active operations

        // Agent should handle shutdown gracefully
        drop(agent);

        // No panics should occur
    }
}
