// Simple test to understand onnxruntime API
use onnxruntime::{environment::Environment, session::Session};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let environment = Environment::builder()
        .with_name("test")
        .build()?;
    
    // Try to load a model
    let model_path = "models/Qwen3-Embedding-0.6B-ONNX/model.onnx";
    
    if std::path::Path::new(model_path).exists() {
        let session = Session::from_file(&environment, model_path)?;
        println!("Successfully loaded ONNX model");
        
        // Print input/output info
        println!("Inputs: {:?}", session.inputs);
        println!("Outputs: {:?}", session.outputs);
    } else {
        println!("Model not found at: {}", model_path);
    }
    
    Ok(())
}