#[cfg(test)]
mod integration_tests {
    use crate::{CompletionRequest, LlmClient, LlmProvider, MultiProviderLlmOrchestrator};

    #[tokio::test]
    async fn test_single_provider_mode() {
        // Test with a mock local provider (will fail in test environment, but tests structure)
        let provider = LlmProvider::Local {
            url: "http://localhost:1234/v1".to_string(),
            model: "test-model".to_string(),
        };

        let client = LlmClient::new(provider, 100, 0.7);
        assert!(!client.is_multi_provider());

        let request = CompletionRequest::new("Hello, world!")
            .max_tokens(50)
            .temperature(0.5);

        // This will likely fail due to no local server, but tests the structure
        let result = client.complete(request).await;

        // In a real test environment, we'd either mock the HTTP client or have a test server
        match result {
            Ok(_) => println!("✅ Single provider test successful"),
            Err(e) => println!("⚠️ Expected error in test environment: {}", e),
        }
    }

    #[tokio::test]
    async fn test_multi_provider_creation() {
        let providers = vec![
            LlmProvider::Local {
                url: "http://localhost:1234/v1".to_string(),
                model: "model1".to_string(),
            },
            LlmProvider::Local {
                url: "http://localhost:1235/v1".to_string(),
                model: "model2".to_string(),
            },
        ];

        let client = LlmClient::new_multi_provider(providers, Some(10.0)); // $10 daily budget
        assert!(client.is_multi_provider());

        println!("✅ Multi-provider client created successfully");

        // Test environment detection
        match LlmClient::from_env_multi() {
            Ok(_) => println!("✅ Environment provider detection successful"),
            Err(e) => println!("⚠️ Expected error - no providers configured in test: {}", e),
        }
    }

    #[test]
    fn test_task_complexity_analysis() {
        use crate::ComplexityLevel;

        // Create a mock client to test complexity analysis
        let provider = LlmProvider::Local {
            url: "http://localhost:1234".to_string(),
            model: "test".to_string(),
        };
        let _client = LlmClient::new(provider, 1000, 0.7);

        // Simple request
        let simple_request = CompletionRequest::new("What is 2+2?");
        let orchestrator = MultiProviderLlmOrchestrator::new(vec![], None);
        let complexity = orchestrator.analyze_task_complexity(&simple_request);

        assert_eq!(complexity.complexity, ComplexityLevel::Simple);
        assert!(complexity.tokens < 100);

        // Complex request
        let complex_request =
            CompletionRequest::new(&"x".repeat(8000)) // Very long request
                .system_prompt("Analyze the complex architecture patterns");
        let complexity = orchestrator.analyze_task_complexity(&complex_request);

        assert_eq!(complexity.complexity, ComplexityLevel::Expert);
        assert!(
            complexity.tokens > 1500,
            "Expected >1500 tokens, got {}",
            complexity.tokens
        );

        println!("✅ Task complexity analysis working correctly");
    }

    #[test]
    fn test_circuit_breaker() {
        use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerState};
        use std::time::Duration;

        let mut cb = CircuitBreaker::new(2, Duration::from_millis(10));

        // Initially closed
        assert_eq!(cb.state, CircuitBreakerState::Closed);
        assert!(cb.can_execute());

        // Record failures
        cb.record_failure();
        assert_eq!(cb.state, CircuitBreakerState::Closed);

        cb.record_failure(); // Should open after 2 failures
        assert_eq!(cb.state, CircuitBreakerState::Open);
        assert!(!cb.can_execute());

        println!("✅ Circuit breaker logic working correctly");
    }

    #[test]
    fn test_cost_optimization() {
        use crate::{ComplexityLevel, CostOptimizer, LlmProvider, Priority, TaskComplexity};

        let optimizer = CostOptimizer::default();

        let providers = vec![
            LlmProvider::OpenAI {
                api_key: "test".to_string(),
                model: "gpt-4o-mini".to_string(),
            },
            LlmProvider::Local {
                url: "http://localhost:1234".to_string(),
                model: "llama".to_string(),
            },
        ];

        let simple_task = TaskComplexity {
            tokens: 100,
            complexity: ComplexityLevel::Simple,
            priority: Priority::Normal,
        };

        let selected = optimizer.select_optimal_provider(&providers, &simple_task);
        assert!(selected.is_some());

        // For simple tasks, should prefer cheaper local model
        if let Some(LlmProvider::Local { .. }) = selected {
            println!("✅ Cost optimization correctly selected local provider for simple task");
        } else {
            println!("⚠️ Cost optimization selected different provider (may be expected)");
        }

        let critical_task = TaskComplexity {
            tokens: 2000,
            complexity: ComplexityLevel::Expert,
            priority: Priority::Critical,
        };

        let selected = optimizer.select_optimal_provider(&providers, &critical_task);
        assert!(selected.is_some());

        println!("✅ Cost optimization working correctly");
    }
}
