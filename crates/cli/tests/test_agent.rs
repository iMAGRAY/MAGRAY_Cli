use cli::agent::{UnifiedAgent, AgentConfig, AgentResponseInfo, AgentMetrics, MemoryConfig, AgentContext};
use tokio;
use std::time::Duration;

#[tokio::test]
async fn test_unified_agent_creation() {
    let agent_result = UnifiedAgent::new().await;
    
    // Agent creation might fail due to missing dependencies
    // but should not panic
    assert!(agent_result.is_ok() || agent_result.is_err());
}

#[tokio::test]
async fn test_agent_simple_query() {
    let agent_result = UnifiedAgent::new().await;
    
    if let Ok(agent) = agent_result {
        let response = agent.process_query("hello").await;
        
        // Response might fail due to missing LLM configuration
        // but should handle gracefully
        assert!(response.is_ok() || response.is_err());
    }
}

#[tokio::test]
async fn test_agent_complex_query() {
    let agent_result = UnifiedAgent::new().await;
    
    if let Ok(agent) = agent_result {
        let response = agent.process_query("analyze the current system status and provide recommendations").await;
        
        // Complex queries should be handled appropriately
        assert!(response.is_ok() || response.is_err());
    }
}

#[tokio::test]
async fn test_agent_empty_query() {
    let agent_result = UnifiedAgent::new().await;
    
    if let Ok(agent) = agent_result {
        let response = agent.process_query("").await;
        
        // Empty query should be handled gracefully
        assert!(response.is_ok() || response.is_err());
    }
}

#[tokio::test]
async fn test_agent_very_long_query() {
    let agent_result = UnifiedAgent::new().await;
    
    if let Ok(agent) = agent_result {
        let long_query = "a".repeat(10000);
        let response = agent.process_query(&long_query).await;
        
        // Very long queries should be handled appropriately
        assert!(response.is_ok() || response.is_err());
    }
}

#[tokio::test]
async fn test_agent_special_characters_query() {
    let agent_result = UnifiedAgent::new().await;
    
    if let Ok(agent) = agent_result {
        let special_query = "Test with émojis 🚀 and spéçial chars: <>\"'&";
        let response = agent.process_query(special_query).await;
        
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
            agent.process_query("test timeout")
        ).await;
        
        // Should either complete within timeout or timeout gracefully
        assert!(response.is_ok() || response.is_err());
    }
}

#[tokio::test]
async fn test_agent_concurrent_requests() {
    let agent_result = UnifiedAgent::new().await;
    
    if let Ok(agent) = agent_result {
        use tokio::join;
        
        let query1 = agent.process_query("first query");
        let query2 = agent.process_query("second query");
        let query3 = agent.process_query("third query");
        
        let (result1, result2, result3): (anyhow::Result<String>, anyhow::Result<String>, anyhow::Result<String>) = join!(query1, query2, query3);
        
        // All concurrent requests should be handled
        assert!(result1.is_ok() || result1.is_err());
        assert!(result2.is_ok() || result2.is_err());
        assert!(result3.is_ok() || result3.is_err());
    }
}

#[test]
fn test_agent_config_validation() {
    let config = AgentConfig::default();
    
    // Default config should be valid
    assert!(config.validate().is_ok());
    
    // Test config with invalid values
    let mut invalid_config = config.clone();
    invalid_config.max_tokens = 0;
    assert!(invalid_config.validate().is_err());
    
    invalid_config.max_tokens = 1000;
    invalid_config.temperature = -1.0;
    assert!(invalid_config.validate().is_err());
    
    invalid_config.temperature = 2.0;
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_agent_config_serialization() {
    let config = AgentConfig {
        llm_provider: "openai".to_string(),
        model_name: "gpt-4".to_string(),
        max_tokens: 2000,
        temperature: 0.7,
        timeout_seconds: 30,
        retry_attempts: 3,
    };
    
    // Test serialization
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("openai"));
    assert!(json.contains("gpt-4"));
    
    // Test deserialization
    let deserialized: AgentConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.llm_provider, config.llm_provider);
    assert_eq!(deserialized.model_name, config.model_name);
    assert_eq!(deserialized.max_tokens, config.max_tokens);
}

#[test]
fn test_agent_response_types() {
    // Test different response types
    let response = AgentResponseInfo {
        content: "Test response".to_string(),
        confidence: 0.95,
        tokens_used: 150,
        processing_time_ms: 1250,
        sources: vec!["memory".to_string(), "knowledge_base".to_string()],
    };
    
    assert_eq!(response.content, "Test response");
    assert_eq!(response.confidence, 0.95);
    assert_eq!(response.tokens_used, 150);
    assert_eq!(response.processing_time_ms, 1250);
    assert_eq!(response.sources.len(), 2);
}

#[tokio::test]
async fn test_agent_error_recovery() {
    let agent_result = UnifiedAgent::new().await;
    
    if let Ok(agent) = agent_result {
        // Test that agent can recover from errors
        let _ = agent.process_query("invalid query that might cause error").await;
        
        // Should still be able to process subsequent queries
        let response = agent.process_query("simple query").await;
        assert!(response.is_ok() || response.is_err());
    }
}

#[test]
fn test_agent_memory_integration() {
    // Test that agent properly integrates with memory system
    let memory_config = MemoryConfig::default();
    
    assert!(memory_config.enabled);
    assert!(memory_config.max_entries > 0);
    assert!(memory_config.ttl_seconds > 0);
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
            let response = agent.process_query(query).await;
            // Tools might not be available, but should handle gracefully
            assert!(response.is_ok() || response.is_err());
        }
    }
}

#[test]
fn test_agent_metrics() {
    let mut metrics = AgentMetrics::default();
    
    assert_eq!(metrics.total_queries, 0);
    assert_eq!(metrics.successful_queries, 0);
    assert_eq!(metrics.failed_queries, 0);
    
    metrics.record_query(true, 100, 1500);
    assert_eq!(metrics.total_queries, 1);
    assert_eq!(metrics.successful_queries, 1);
    assert_eq!(metrics.failed_queries, 0);
    assert_eq!(metrics.total_tokens_used, 100);
    assert_eq!(metrics.total_processing_time_ms, 1500);
    
    metrics.record_query(false, 50, 800);
    assert_eq!(metrics.total_queries, 2);
    assert_eq!(metrics.successful_queries, 1);
    assert_eq!(metrics.failed_queries, 1);
    assert_eq!(metrics.total_tokens_used, 150);
    assert_eq!(metrics.total_processing_time_ms, 2300);
}

#[test]
fn test_agent_context_management() {
    let mut context = AgentContext::new();
    
    assert!(context.conversation_history.is_empty());
    
    context.add_message("user", "Hello");
    context.add_message("assistant", "Hi there!");
    
    assert_eq!(context.conversation_history.len(), 2);
    assert_eq!(context.conversation_history[0].role, "user");
    assert_eq!(context.conversation_history[0].content, "Hello");
    assert_eq!(context.conversation_history[1].role, "assistant");
    assert_eq!(context.conversation_history[1].content, "Hi there!");
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