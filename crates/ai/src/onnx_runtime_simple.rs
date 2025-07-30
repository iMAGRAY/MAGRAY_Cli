use crate::{AiError, Result};
use ort::{
    session::{Session, builder::GraphOptimizationLevel},
};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};

/// Simplified ONNX Runtime session wrapper
pub struct OrtSession {
    session: Arc<Session>,
    model_name: String,
    model_path: PathBuf,
}

impl OrtSession {
    /// Initialize ONNX Runtime environment (call once at startup)
    pub fn init_environment() -> Result<()> {
        ort::init()
            .with_name("magray")
            .commit()
            .map_err(|e| AiError::ModelLoadError(format!("Failed to init ORT: {}", e)))?;
        
        info!("ONNX Runtime environment initialized");
        Ok(())
    }
    
    /// Create a new ONNX session
    pub fn new(model_name: String, model_path: PathBuf, use_gpu: bool) -> Result<Self> {
        info!("Loading ONNX model: {} from {:?}", model_name, model_path);
        
        if !model_path.exists() {
            return Err(AiError::ModelLoadError(format!("Model file not found: {:?}", model_path)));
        }
        
        // Create session builder
        let builder = Session::builder()
            .map_err(|e| AiError::ModelLoadError(format!("Failed to create session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| AiError::ModelLoadError(format!("Failed to set optimization: {}", e)))?
            .with_intra_threads(num_cpus::get())
            .map_err(|e| AiError::ModelLoadError(format!("Failed to set threads: {}", e)))?;
        
        // TODO: Add GPU support when ort 2.0 stabilizes
        if use_gpu {
            debug!("GPU requested but not yet supported in this version");
        }
        
        // Load the model
        let session = builder
            .commit_from_file(&model_path)
            .map_err(|e| AiError::ModelLoadError(format!("Failed to load model: {}", e)))?;
        
        info!("Successfully loaded ONNX model: {}", model_name);
        
        Ok(Self {
            session: Arc::new(session),
            model_name,
            model_path,
        })
    }
    
    pub fn model_name(&self) -> &str {
        &self.model_name
    }
    
    pub fn model_path(&self) -> &std::path::Path {
        &self.model_path
    }
    
    /// Get model input/output information (simplified for ort 2.0)
    pub fn get_input_info(&self) -> Result<Vec<(String, Vec<i64>)>> {
        let inputs = self.session.inputs
            .iter()
            .map(|input| {
                let name = input.name.clone();
                // Dynamic shape for now
                let shape = vec![-1, -1];
                (name, shape)
            })
            .collect();
        
        Ok(inputs)
    }
    
    pub fn get_output_info(&self) -> Result<Vec<(String, Vec<i64>)>> {
        let outputs = self.session.outputs
            .iter()
            .map(|output| {
                let name = output.name.clone();
                // Expected shapes based on model type
                let shape = if name.contains("embed") || name.contains("hidden") {
                    vec![-1, -1, 768]
                } else {
                    vec![-1, -1]
                };
                (name, shape)
            })
            .collect();
        
        Ok(outputs)
    }
    
    /// Run inference for embeddings (simplified - returns mock data for now)
    pub fn run_embeddings(&self, batch_size: usize, _seq_len: usize) -> Result<Vec<Vec<f32>>> {
        debug!("Running embedding inference for batch size: {}", batch_size);
        
        // TODO: Implement real inference when API stabilizes
        // For now, return mock embeddings
        let embedding_dim = 768;
        let mut embeddings = Vec::new();
        
        for i in 0..batch_size {
            let mut embedding = vec![0.0f32; embedding_dim];
            // Create different embeddings for each input
            for j in 0..embedding_dim {
                embedding[j] = ((i * embedding_dim + j) as f32 * 0.001).sin();
            }
            // L2 normalize
            let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for v in embedding.iter_mut() {
                    *v /= norm;
                }
            }
            embeddings.push(embedding);
        }
        
        Ok(embeddings)
    }
    
    /// Run reranking inference (simplified - returns mock scores for now)
    pub fn run_reranking(&self, batch_size: usize) -> Result<Vec<f32>> {
        debug!("Running reranking inference for batch size: {}", batch_size);
        
        // TODO: Implement real inference when API stabilizes
        // For now, return mock scores
        let scores: Vec<f32> = (0..batch_size)
            .map(|i| 1.0 / (1.0 + i as f32))
            .collect();
        
        Ok(scores)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ort_session_creation() {
        // Initialize environment
        let _ = OrtSession::init_environment();
        
        // This test requires model files to be present
        let model_path = PathBuf::from("crates/memory/models/Qwen3-Embedding-0.6B-ONNX/model.onnx");
        if model_path.exists() {
            let session = OrtSession::new(
                "Qwen3-Embedding-0.6B".to_string(),
                model_path,
                false
            );
            
            assert!(session.is_ok());
            
            let session = session.unwrap();
            assert_eq!(session.model_name(), "Qwen3-Embedding-0.6B");
            
            // Check inputs/outputs
            let inputs = session.get_input_info().unwrap();
            let outputs = session.get_output_info().unwrap();
            
            assert!(!inputs.is_empty());
            assert!(!outputs.is_empty());
        }
    }
}