use anyhow::Result;
use ort::{inputs, session::Session};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== ORT 2.0 Minimal ONNX Inference Test ===\n");
    
    // Set ONNX Runtime DLL path
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("scripts")
        .join("onnxruntime")
        .join("lib")
        .join("onnxruntime.dll");
    
    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
    
    // Model path
    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("model.onnx");
    
    println!("Model path: {}", model_path.display());
    println!("Model exists: {}", model_path.exists());
    
    if !model_path.exists() {
        println!("\n❌ Model file not found!");
        return Err(anyhow::anyhow!("Model file not found"));
    }
    
    // Initialize ONNX Runtime
    println!("\n1. Initializing ONNX Runtime...");
    ort::init()
        .with_name("test_ort2_minimal")
        .commit()?;
    println!("✅ ORT initialized");
    
    // Create session
    println!("\n2. Creating ONNX session...");
    let session = Session::builder()?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
        .commit_from_file(&model_path)?;
    println!("✅ Session created");
    
    // Check model inputs/outputs
    println!("\n3. Model metadata:");
    println!("Inputs:");
    for (i, input) in session.inputs.iter().enumerate() {
        println!("  {}: {} (type: {:?})", i, input.name, input.input_type);
    }
    println!("Outputs:");
    for (i, output) in session.outputs.iter().enumerate() {
        println!("  {}: {} (type: {:?})", i, output.name, output.output_type);
    }
    
    // Create minimal test input tensors
    println!("\n4. Creating test tensors...");
    
    // Create mock input IDs (typical for transformer models)
    // Using a simple sequence: [101, 7592, 2088, 102] (BERT-like tokens)
    let input_ids = vec![101i64, 7592, 2088, 102];
    let attention_mask = vec![1i64, 1, 1, 1];
    let seq_len = input_ids.len();
    
    // Create tensors with shape [1, seq_len]
    let input_ids_tensor = ort::value::Tensor::from_array(([1, seq_len], input_ids))?;
    let attention_mask_tensor = ort::value::Tensor::from_array(([1, seq_len], attention_mask))?;
    
    println!("✅ Test tensors created");
    println!("   Input shape: [1, {}]", seq_len);
    
    // Run inference
    println!("\n5. Running ONNX inference...");
    
    let mut session = std::sync::Mutex::new(session);
    let mut session_guard = session.lock().unwrap();
    
    let outputs = session_guard.run(inputs![
        "input_ids" => input_ids_tensor,
        "attention_mask" => attention_mask_tensor
    ])?;
    
    println!("✅ Inference completed!");
    
    // Examine outputs
    println!("\n6. Output analysis:");
    for (name, output) in outputs.iter() {
        println!("Output '{}': {:?}", name, output.dtype());
        
        // Try to extract tensor info
        match output.try_extract_tensor::<f32>() {
            Ok((shape, data)) => {
                let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                println!("   Shape: {:?}", shape_vec);
                println!("   Data length: {}", data.len());
                println!("   First 5 values: {:?}", &data[..5.min(data.len())]);
                
                // Calculate some basic stats
                if !data.is_empty() {
                    let min = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                    let max = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                    let mean = data.iter().sum::<f32>() / data.len() as f32;
                    println!("   Stats: min={:.4}, max={:.4}, mean={:.4}", min, max, mean);
                }
            },
            Err(e) => {
                println!("   Could not extract as f32 tensor: {}", e);
                
                // Try as different types
                if let Ok((shape, data)) = output.try_extract_tensor::<i64>() {
                    println!("   Extracted as i64 tensor");
                    let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                println!("   Shape: {:?}", shape_vec);
                    println!("   Data length: {}", data.len());
                    println!("   First 5 values: {:?}", &data[..5.min(data.len())]);
                }
            }
        }
    }
    
    println!("\n✅ ORT 2.0 ONNX inference test completed successfully!");
    println!("✅ Real ONNX models are working with ORT 2.0 API!");
    
    Ok(())
}