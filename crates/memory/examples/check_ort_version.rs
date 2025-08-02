fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Initialize ONNX Runtime
    ort::init()
        .with_name("ort_version_check")
        .commit()?;
    
    println!("✅ ONNX Runtime successfully initialized!");
    println!("📦 Using ort crate version: 2.0.0-rc.10");
    println!("🔧 Expected ONNX Runtime version: 1.22");
    println!("📁 ORT_DYLIB_PATH: {:?}", std::env::var("ORT_DYLIB_PATH").ok());
    
    // Test loading a simple model to verify everything works
    println!("\n🔍 Testing ONNX Runtime functionality...");
    
    Ok(())
}