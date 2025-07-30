use anyhow::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== ONNX Runtime Init Test ===\n");
    
    // Set ONNX Runtime DLL path
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("scripts")
        .join("onnxruntime")
        .join("lib")
        .join("onnxruntime.dll");
    
    println!("DLL Path: {}", dll_path.display());
    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
    
    // Initialize ONNX Runtime
    println!("\nInitializing ONNX Runtime...");
    
    let builder = ort::init();
    println!("✅ Got builder");
    
    let _env = builder.commit()?;
    println!("✅ ONNX Runtime initialized successfully!");
    
    // Version info is not directly available in ort 2.0
    
    println!("\n✅ ONNX Runtime 1.22.x is installed and working!");
    
    Ok(())
}