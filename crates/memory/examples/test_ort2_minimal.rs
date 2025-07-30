use anyhow::Result;
use ort::inputs;

fn main() -> Result<()> {
    println!("Testing minimal ORT 2.0 API...\n");
    
    // Set DLL path
    std::env::set_var("ORT_DYLIB_PATH", 
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()
            .parent().unwrap()
            .join("scripts/onnxruntime/lib/onnxruntime.dll")
            .to_str().unwrap()
    );
    
    // Initialize
    ort::init().commit()?;
    println!("✅ ORT initialized");
    
    // Test different tensor creation methods
    println!("\n1. Testing tensor creation from ndarray:");
    
    // Method 1: From tuple
    let tensor1 = ort::value::Tensor::from_array(([1, 4], vec![1.0f32, 2.0, 3.0, 4.0]))?;
    println!("✅ Created tensor from tuple");
    
    // Method 2: From ndarray - let's see what works
    use ndarray::Array2;
    let array = Array2::<f32>::from_shape_vec((1, 4), vec![1.0, 2.0, 3.0, 4.0])?;
    
    // Try different approaches
    println!("\n2. Testing ndarray conversion:");
    
    // Approach 1: Direct (will likely fail)
    // let tensor2 = ort::value::Tensor::from_array(array)?;
    
    // Approach 2: From shape and vec  
    let shape = array.shape().to_vec();
    let data = array.into_raw_vec();
    let tensor2 = ort::value::Tensor::from_array((shape.as_slice(), data))?;
    println!("✅ Created tensor from ndarray via shape+vec");
    
    // Test inputs! macro
    println!("\n3. Testing inputs! macro:");
    
    // The macro returns Vec<(Cow<'_, str>, SessionInputValue<'_>)> in ORT 2.0
    let inputs_result = inputs![
        "input1" => tensor1,
        "input2" => tensor2
    ];
    
    println!("✅ inputs! macro works, created {} inputs", inputs_result.len());
    
    println!("\n✅ Basic API test complete!");
    
    Ok(())
}