use crate::{AiError, Result};
use std::path::Path;
use tracing::{info, warn};

/// Mock session for development/testing
pub struct MockSession {
    model_name: String,
}

impl MockSession {
    pub fn new(model_name: String) -> Self {
        Self { model_name }
    }
}

/// Mock model loader for development/testing  
pub struct MockModelLoader {
    models_dir: std::path::PathBuf,
}

impl MockModelLoader {
    pub fn new(models_dir: impl AsRef<Path>) -> Result<Self> {
        let models_dir = models_dir.as_ref().to_path_buf();
        
        if !models_dir.exists() {
            warn!("Models directory not found: {} (using mock)", models_dir.display());
        }
        
        info!("Initialized mock model loader");
        
        Ok(Self { models_dir })
    }
    
    pub fn load_model(&self, model_name: &str, _use_gpu: bool) -> Result<MockSession> {
        info!("Loading mock model: {}", model_name);
        Ok(MockSession::new(model_name.to_string()))
    }
    
    pub fn model_exists(&self, model_name: &str) -> bool {
        self.models_dir.join(model_name).exists()
    }
    
    pub fn list_models(&self) -> Result<Vec<String>> {
        let mut models = Vec::new();
        
        if self.models_dir.exists() {
            for entry in std::fs::read_dir(&self.models_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    models.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
        
        models.sort();
        Ok(models)
    }
    
    pub fn get_tokenizer_path(&self, model_name: &str) -> std::path::PathBuf {
        self.models_dir.join(model_name).join("tokenizer.json")
    }
}