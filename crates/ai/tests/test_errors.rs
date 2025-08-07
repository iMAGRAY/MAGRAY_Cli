use ai::AiError;
use std::io;

#[test]
fn test_ai_error_model_not_found() {
    let error = AiError::ModelNotFound("test-model".to_string());
    assert_eq!(format!("{}", error), "Model not found: test-model");

    // Проверяем что это правильный вариант
    match error {
        AiError::ModelNotFound(name) => assert_eq!(name, "test-model"),
        _ => panic!("Wrong error variant"),
    }
}

#[test]
fn test_ai_error_model_error() {
    let error = AiError::ModelError("Model is corrupted".to_string());
    assert_eq!(format!("{}", error), "Model error: Model is corrupted");
}

#[test]
fn test_ai_error_model_load_error() {
    let error = AiError::ModelLoadError("Failed to load ONNX model".to_string());
    assert_eq!(
        format!("{}", error),
        "Model load error: Failed to load ONNX model"
    );
}

#[test]
fn test_ai_error_inference_error() {
    let error = AiError::InferenceError("GPU out of memory".to_string());
    assert_eq!(format!("{}", error), "Inference error: GPU out of memory");
}

#[test]
fn test_ai_error_tokenizer_error() {
    let error = AiError::TokenizerError("Invalid UTF-8 sequence".to_string());
    assert_eq!(
        format!("{}", error),
        "Tokenizer error: Invalid UTF-8 sequence"
    );
}

#[test]
fn test_ai_error_validation_error() {
    let error = AiError::ValidationError("Invalid batch size: 0".to_string());
    assert_eq!(
        format!("{}", error),
        "Validation error: Invalid batch size: 0"
    );
}

#[test]
fn test_ai_error_config_error() {
    let error = AiError::ConfigError("Missing model path".to_string());
    assert_eq!(format!("{}", error), "Config error: Missing model path");
}

#[test]
fn test_ai_error_network_error() {
    let error = AiError::NetworkError("Connection timeout".to_string());
    assert_eq!(format!("{}", error), "Network error: Connection timeout");
}

#[test]
fn test_ai_error_from_io_error() {
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let ai_error = AiError::from(io_error);

    match ai_error {
        AiError::IoError(e) => assert_eq!(e.kind(), io::ErrorKind::NotFound),
        _ => panic!("Wrong error variant"),
    }
}

#[test]
fn test_ai_error_display() {
    let errors = vec![
        (AiError::ModelError("test".to_string()), "Model error: test"),
        (
            AiError::ModelLoadError("test".to_string()),
            "Model load error: test",
        ),
        (
            AiError::ModelNotFound("test".to_string()),
            "Model not found: test",
        ),
        (
            AiError::InferenceError("test".to_string()),
            "Inference error: test",
        ),
        (
            AiError::TokenizerError("test".to_string()),
            "Tokenizer error: test",
        ),
        (
            AiError::ValidationError("test".to_string()),
            "Validation error: test",
        ),
        (
            AiError::ConfigError("test".to_string()),
            "Config error: test",
        ),
        (
            AiError::NetworkError("test".to_string()),
            "Network error: test",
        ),
    ];

    for (error, expected) in errors {
        assert_eq!(format!("{}", error), expected);
    }
}

#[test]
fn test_ai_error_chain() {
    let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
    let ai_error = AiError::ModelLoadError(format!("Cannot load model: {}", io_error));

    let error_string = format!("{}", ai_error);
    assert!(error_string.contains("Cannot load model"));
    assert!(error_string.contains("Access denied"));
}

#[test]
fn test_ai_error_debug() {
    let error = AiError::ConfigError("Test config error".to_string());
    let debug_str = format!("{:?}", error);

    assert!(debug_str.contains("ConfigError"));
    assert!(debug_str.contains("Test config error"));
}

#[test]
fn test_ai_error_std_error_trait() {
    let error = AiError::ModelNotFound("test".to_string());
    // Проверяем что error реализует std::error::Error
    let _description = error.to_string();
}
