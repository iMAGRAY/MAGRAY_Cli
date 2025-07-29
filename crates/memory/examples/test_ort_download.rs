use anyhow::Result;

fn main() -> Result<()> {
    println!("Testing ORT 2.0 with automatic download...");
    
    // Явно устанавливаем переменную для загрузки
    std::env::set_var("ORT_DYLIB_PATH", "");
    
    println!("\nCurrent directory: {}", std::env::current_dir()?.display());
    println!("Executable path: {}", std::env::current_exe()?.display());
    
    // Инициализируем ORT - должно автоматически скачать библиотеки
    println!("\nInitializing ONNX Runtime (should download if needed)...");
    
    match ort::init()
        .with_name("magray_download_test")
        .commit() {
        Ok(_) => {
            println!("✓ ORT initialized successfully!");
            
            // Проверяем, что можем создать сессию
            println!("\nTesting session creation...");
            match ort::session::Session::builder() {
                Ok(builder) => {
                    println!("✓ Session builder created!");
                    
                    // Пробуем задать параметры
                    match builder
                        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3) {
                        Ok(_) => println!("✓ Optimization level set!"),
                        Err(e) => println!("✗ Failed to set optimization level: {}", e),
                    }
                }
                Err(e) => println!("✗ Failed to create session builder: {}", e),
            }
        }
        Err(e) => {
            println!("✗ Failed to initialize ORT: {}", e);
            println!("\nTroubleshooting:");
            println!("1. Make sure you have internet connection for downloading");
            println!("2. Try running with: cargo clean && cargo build");
            println!("3. Check if antivirus is blocking downloads");
            
            // Проверяем переменные окружения
            println!("\nEnvironment variables:");
            for (key, value) in std::env::vars() {
                if key.contains("ORT") || key.contains("ONNX") {
                    println!("  {} = {}", key, value);
                }
            }
        }
    }
    
    Ok(())
}