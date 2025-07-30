use anyhow::Result;
use ndarray::{Array2, ArrayViewD};
use ort::{inputs, session::Session, value::Tensor};
use std::path::PathBuf;
use tokenizers::tokenizer::Tokenizer;

fn main() -> Result<()> {
    println!("=== Direct ORT Test ===\n");
    
    // Set ONNX Runtime DLL path
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("scripts")
        .join("onnxruntime")
        .join("lib")
        .join("onnxruntime.dll");
    
    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
    
    // Initialize ONNX Runtime
    println!("1. Initializing ONNX Runtime...");
    ort::init().commit()?;
    println!("   ✅ ONNX Runtime initialized!");
    
    // Load model
    println!("\n2. Loading ONNX model...");
    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("model.onnx");
    
    if !model_path.exists() {
        println!("   ❌ Model not found at: {}", model_path.display());
        return Err(anyhow::anyhow!("Model file not found"));
    }
    
    let session = Session::builder()?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(&model_path)?;
    
    println!("   ✅ Model loaded successfully!");
    
    // Load tokenizer
    println!("\n3. Loading tokenizer...");
    let tokenizer_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("tokenizer.json");
    
    let tokenizer = Tokenizer::from_file(&tokenizer_path)
        .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;
    
    println!("   ✅ Tokenizer loaded!");
    
    // Test embedding generation
    println!("\n4. Testing embedding generation...");
    
    let test_text = "Hello, ONNX Runtime!";
    println!("   Text: \"{}\"", test_text);
    
    // Tokenize
    let encoding = tokenizer.encode(test_text, true)
        .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;
    
    let input_ids = encoding.get_ids();
    let attention_mask = encoding.get_attention_mask();
    
    println!("   Tokens: {} tokens", input_ids.len());
    
    // Convert to i64 arrays
    let input_ids_i64: Vec<i64> = input_ids.iter().map(|&id| id as i64).collect();
    let attention_mask_i64: Vec<i64> = attention_mask.iter().map(|&m| m as i64).collect();
    
    let n_tokens = input_ids_i64.len();
    let input_ids_array = Array2::from_shape_vec((1, n_tokens), input_ids_i64)?;
    let attention_mask_array = Array2::from_shape_vec((1, n_tokens), attention_mask_i64)?;
    
    // Create tensors
    let input_ids_tensor = Tensor::from_array(input_ids_array)?;
    let attention_mask_tensor = Tensor::from_array(attention_mask_array)?;
    
    // Run inference
    println!("\n5. Running inference...");
    let outputs = session.run(inputs! {
        "input_ids" => input_ids_tensor,
        "attention_mask" => attention_mask_tensor
    }?)?;
    
    println!("   ✅ Inference completed!");
    
    // Extract embeddings
    println!("\n6. Extracting embeddings...");
    let output = outputs.iter().next().unwrap().1;
    let tensor_view: ArrayViewD<f32> = output.try_extract_tensor()?;
    
    let shape = tensor_view.shape();
    println!("   Output shape: {:?}", shape);
    
    // Mean pooling (assuming [batch, sequence, hidden] format)
    let embeddings = if shape.len() == 3 {
        let seq_len = shape[1];
        let hidden_size = shape[2];
        
        let mut pooled = vec![0.0f32; hidden_size];
        for seq_idx in 0..seq_len {
            for hidden_idx in 0..hidden_size {
                pooled[hidden_idx] += tensor_view[[0, seq_idx, hidden_idx]];
            }
        }
        
        // Average
        for val in &mut pooled {
            *val /= seq_len as f32;
        }
        
        pooled
    } else if shape.len() == 2 {
        // Already pooled
        let hidden_size = shape[1];
        let mut result = vec![0.0f32; hidden_size];
        for i in 0..hidden_size {
            result[i] = tensor_view[[0, i]];
        }
        result
    } else {
        return Err(anyhow::anyhow!("Unexpected output shape: {:?}", shape));
    };
    
    println!("   Embedding dimensions: {}", embeddings.len());
    println!("   First 5 values: {:?}", &embeddings[..5.min(embeddings.len())]);
    
    // Normalize
    let norm: f32 = embeddings.iter().map(|x| x * x).sum::<f32>().sqrt();
    println!("   Norm: {:.4}", norm);
    
    println!("\n✅ Success! ONNX Runtime 1.22.x is working with real models!");
    
    Ok(())
}