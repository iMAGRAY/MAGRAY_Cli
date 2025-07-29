use anyhow::Result;
use std::path::PathBuf;
use ort::{
    session::{Session, builder::GraphOptimizationLevel},
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing ONNX Runtime...");
    
    // Проверяем, что ONNX Runtime работает
    println!("ONNX Runtime is available");
    
    // Пробуем загрузить модель
    let model_path = PathBuf::from("C:/Users/1/Documents/GitHub/MAGRAY_Cli/models/Qwen3-Embedding-0.6B-ONNX/model.onnx");
    
    println!("Loading model from: {}", model_path.display());
    
    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(&model_path)?;
    
    println!("Model loaded successfully!");
    
    // Получаем информацию о входах и выходах
    let inputs = session.inputs;
    let outputs = session.outputs;
    
    println!("\nModel inputs:");
    for (i, input) in inputs.iter().enumerate() {
        println!("  {}: {} {:?}", i, input.name, input.input_type);
    }
    
    println!("\nModel outputs:");
    for (i, output) in outputs.iter().enumerate() {
        println!("  {}: {} {:?}", i, output.name, output.output_type);
    }
    
    Ok(())
}