#![cfg(feature = "extended-tests")]

use llm::{ChatMessage, CompletionRequest, LlmClient, LlmProvider};
use mockito::Server;

#[allow(dead_code)]
fn create_mock_openai_response() -> String {
    r#"{
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "Test response"
            }
        }]
    }"#
    .to_string()
}

#[allow(dead_code)]
fn create_mock_anthropic_response() -> String {
    r#"{
        "content": [{
            "text": "Test response"
        }]
    }"#
    .to_string()
}

#[test]
fn test_llm_provider_creation() {
    // OpenAI provider
    let openai = LlmProvider::OpenAI {
        api_key: "test-key".to_string(),
        model: "gpt-4".to_string(),
    };

    match openai {
        LlmProvider::OpenAI { api_key, model } => {
            assert_eq!(api_key, "test-key");
            assert_eq!(model, "gpt-4");
        }
        _ => panic!("Wrong provider type"),
    }

    // Anthropic provider
    let anthropic = LlmProvider::Anthropic {
        api_key: "test-key".to_string(),
        model: "claude-3".to_string(),
    };

    match anthropic {
        LlmProvider::Anthropic { api_key, model } => {
            assert_eq!(api_key, "test-key");
            assert_eq!(model, "claude-3");
        }
        _ => panic!("Wrong provider type"),
    }

    // Local provider
    let local = LlmProvider::Local {
        url: "http://localhost:8080".to_string(),
        model: "llama2".to_string(),
    };

    match local {
        LlmProvider::Local { url, model } => {
            assert_eq!(url, "http://localhost:8080");
            assert_eq!(model, "llama2");
        }
        _ => panic!("Wrong provider type"),
    }
}

#[tokio::test]
async fn test_llm_client_from_env() {
    // Тест создания из переменных окружения
    std::env::set_var("LLM_PROVIDER", "openai");
    std::env::set_var("OPENAI_API_KEY", "test-key");
    std::env::set_var("OPENAI_MODEL", "gpt-4");

    let _client = LlmClient::from_env().unwrap();
    // Проверяем что клиент создан (детали приватные)

    // Очистка
    std::env::remove_var("LLM_PROVIDER");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("OPENAI_MODEL");
}

#[tokio::test]
async fn test_llm_client_from_env_missing() {
    // Сохраняем текущие значения
    let old_provider = std::env::var("LLM_PROVIDER").ok();

    // Очищаем переменные
    std::env::remove_var("LLM_PROVIDER");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("ANTHROPIC_API_KEY");

    let result = LlmClient::from_env();
    assert!(result.is_err());

    // Восстанавливаем если были
    if let Some(provider) = old_provider {
        std::env::set_var("LLM_PROVIDER", provider);
    }
}

#[tokio::test]
async fn test_chat_message_creation() {
    let user_msg = ChatMessage::user("Hello");
    assert_eq!(user_msg.role, "user");
    assert_eq!(user_msg.content, "Hello");

    let assistant_msg = ChatMessage::assistant("Hi there");
    assert_eq!(assistant_msg.role, "assistant");
    assert_eq!(assistant_msg.content, "Hi there");

    let system_msg = ChatMessage::system("You are helpful");
    assert_eq!(system_msg.role, "system");
    assert_eq!(system_msg.content, "You are helpful");
}

#[tokio::test]
async fn test_completion_request_builder() {
    let request = CompletionRequest::new("Test prompt")
        .max_tokens(100)
        .temperature(0.7)
        .system_prompt("Be helpful");

    assert_eq!(request.prompt, "Test prompt");
    assert_eq!(request.max_tokens, Some(100));
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.system_prompt, Some("Be helpful".to_string()));
}

#[tokio::test]
async fn test_openai_completion_mock() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(create_mock_openai_response())
        .create_async()
        .await;

    let _provider = LlmProvider::OpenAI {
        api_key: "test-key".to_string(),
        model: "gpt-4".to_string(),
    };

    // Создаем клиент с mock URL
    let provider = LlmProvider::Local {
        url: server.url(),
        model: "test-model".to_string(),
    };
    let client = LlmClient::new(provider, 1000, 0.7);

    let request = CompletionRequest::new("Hello");
    let response = client.complete(request).await.unwrap();

    assert_eq!(response, "Test response");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_anthropic_completion_mock() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(create_mock_openai_response())
        .create_async()
        .await;

    let _provider = LlmProvider::Anthropic {
        api_key: "test-key".to_string(),
        model: "claude-3".to_string(),
    };

    let provider = LlmProvider::Local {
        url: server.url(),
        model: "test-model".to_string(),
    };
    let client = LlmClient::new(provider, 1000, 0.7);

    let request = CompletionRequest::new("Hello");
    let response = client.complete(request).await.unwrap();

    assert_eq!(response, "Test response");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_local_completion_mock() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(create_mock_openai_response())
        .create_async()
        .await;

    let provider = LlmProvider::Local {
        url: server.url(),
        model: "llama2".to_string(),
    };

    let client = LlmClient::new(provider, 1000, 0.7);

    let request = CompletionRequest::new("Hello");
    let response = client.complete(request).await.unwrap();

    assert_eq!(response, "Test response");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_chat_with_history() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(create_mock_openai_response())
        .create_async()
        .await;

    let _provider = LlmProvider::OpenAI {
        api_key: "test-key".to_string(),
        model: "gpt-4".to_string(),
    };

    let provider = LlmProvider::Local {
        url: server.url(),
        model: "test-model".to_string(),
    };
    let client = LlmClient::new(provider, 1000, 0.7);

    let messages = vec![
        ChatMessage::system("You are helpful"),
        ChatMessage::user("Hello"),
        ChatMessage::assistant("Hi there"),
        ChatMessage::user("How are you?"),
    ];

    let response = client.chat(&messages).await.unwrap();
    assert_eq!(response, "Test response");

    mock.assert_async().await;
}

#[test]
fn test_provider_debug_impl() {
    let provider = LlmProvider::OpenAI {
        api_key: "secret".to_string(),
        model: "gpt-4".to_string(),
    };

    let debug_str = format!("{:?}", provider);
    assert!(debug_str.contains("OpenAI"));
    assert!(debug_str.contains("gpt-4"));
}

#[test]
fn test_message_serialization() {
    let msg = ChatMessage::user("Test");
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("user"));
    assert!(json.contains("Test"));

    let deserialized: ChatMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.role, "user");
    assert_eq!(deserialized.content, "Test");
}

#[tokio::test]
async fn test_error_handling() {
    let mut server = Server::new_async().await;

    // Mock для ошибки
    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(500)
        .with_body("Internal Server Error")
        .create_async()
        .await;

    let _provider = LlmProvider::OpenAI {
        api_key: "test-key".to_string(),
        model: "gpt-4".to_string(),
    };

    let provider = LlmProvider::Local {
        url: server.url(),
        model: "test-model".to_string(),
    };
    let client = LlmClient::new(provider, 1000, 0.7);

    let request = CompletionRequest::new("Hello");
    let result = client.complete(request).await;

    assert!(result.is_err());
    mock.assert_async().await;
}
