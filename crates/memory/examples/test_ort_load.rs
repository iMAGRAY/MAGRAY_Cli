use anyhow::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("Testing ORT 2.0 with explicit loading...");
    
    // Убираем системную папку из PATH временно
    if let Ok(path_var) = std::env::var("PATH") {
        let filtered_path: Vec<&str> = path_var
            .split(';')
            .filter(|p| !p.to_lowercase().contains("system32"))
            .collect();
        std::env::set_var("PATH", filtered_path.join(";"));
    }
    
    // Очищаем переменную ORT
    std::env::remove_var("ORT_DYLIB_PATH");
    
    // Создаем директорию для загрузки ORT
    let ort_dir = PathBuf::from("target/ort-download");
    std::fs::create_dir_all(&ort_dir)?;
    
    println!("ORT download directory: {}", ort_dir.canonicalize()?.display());
    std::env::set_var("ORT_DOWNLOAD_DIR", ort_dir.canonicalize()?);
    
    // Пробуем загрузить ORT
    println!("\nInitializing ONNX Runtime (should download correct version)...");
    
    match ort::init()
        .with_name("magray_test_load")
        .commit() {
        Ok(_) => {
            println!("✓ ORT initialized successfully!");
            
            // Проверяем версию
            println!("\nChecking ORT version...");
            
            // Создаем простую сессию для проверки
            match ort::session::Session::builder() {
                Ok(builder) => {
                    println!("✓ Session builder created!");
                    
                    // Пробуем установить параметры
                    match builder
                        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
                        .with_intra_threads(1) {
                        Ok(_) => println!("✓ Session configured successfully!"),
                        Err(e) => println!("✗ Failed to configure session: {}", e),
                    }
                }
                Err(e) => println!("✗ Failed to create session builder: {}", e),
            }
        }
        Err(e) => {
            println!("✗ Failed to initialize ORT: {}", e);
            
            // Детальная диагностика
            println!("\nDiagnostics:");
            println!("- Working dir: {}", std::env::current_dir()?.display());
            
            // Проверяем что есть в target
            if let Ok(entries) = std::fs::read_dir("target") {
                println!("- Files in target/:");
                for entry in entries.flatten().take(10) {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.contains("onnx") || name.contains("ort") {
                            println!("    {}", name);
                        }
                    }
                }
            }
            
            // Проверяем системные переменные
            println!("\n- ORT-related environment variables:");
            for (key, value) in std::env::vars() {
                if key.contains("ORT") || key.contains("ONNX") {
                    println!("    {} = {}", key, value);
                }
            }
        }
    }
    
    Ok(())
}