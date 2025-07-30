use std::fmt;

#[derive(Debug)]
pub enum AiError {
    /// Model loading error
    ModelError(String),
    /// Model loading error
    ModelLoadError(String),
    /// Model not found error
    ModelNotFound(String),
    /// Inference error
    InferenceError(String),
    /// Tokenization error
    TokenizerError(String),
    /// Input validation error
    ValidationError(String),
    /// IO error
    IoError(std::io::Error),
    /// Configuration error
    ConfigError(String),
    /// Network error (for remote APIs)
    NetworkError(String),
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiError::ModelError(msg) => write!(f, "Model error: {}", msg),
            AiError::ModelLoadError(msg) => write!(f, "Model load error: {}", msg),
            AiError::ModelNotFound(msg) => write!(f, "Model not found: {}", msg),
            AiError::InferenceError(msg) => write!(f, "Inference error: {}", msg),
            AiError::TokenizerError(msg) => write!(f, "Tokenizer error: {}", msg),
            AiError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AiError::IoError(e) => write!(f, "IO error: {}", e),
            AiError::ConfigError(msg) => write!(f, "Config error: {}", msg),
            AiError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl std::error::Error for AiError {}

impl From<std::io::Error> for AiError {
    fn from(e: std::io::Error) -> Self {
        AiError::IoError(e)
    }
}

impl From<tokenizers::Error> for AiError {
    fn from(e: tokenizers::Error) -> Self {
        AiError::TokenizerError(e.to_string())
    }
}

impl From<ort::Error> for AiError {
    fn from(e: ort::Error) -> Self {
        AiError::ModelLoadError(e.to_string())
    }
}

impl From<ndarray::ShapeError> for AiError {
    fn from(e: ndarray::ShapeError) -> Self {
        AiError::ValidationError(format!("Array shape error: {}", e))
    }
}