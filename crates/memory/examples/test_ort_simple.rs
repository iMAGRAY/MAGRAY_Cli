use anyhow::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("Testing ORT 2.0 initialization...");
    
    // Пробуем найти DLL в разных местах
    let possible_dll_paths = vec![
        std::env::current_exe()?.parent().unwrap().to_path_buf(),
        PathBuf::from("target/debug"),
        PathBuf::from("target/debug/deps"),
        PathBuf::from("C:\\Users\\1\\Documents\\GitHub\\MAGRAY_Cli\\target\\debug"),
        PathBuf::from("C:\\Users\\1\\Documents\\GitHub\\MAGRAY_Cli\\target\\debug\\deps"),
    ];
    
    println!("Searching for ONNX Runtime DLLs...");
    let mut dll_found = false;
    
    for path in &possible_dll_paths {
        println!("Checking: {}", path.display());
        
        // Проверяем разные варианты имён DLL
        let dll_names = vec![
            "onnxruntime.dll",
            "onnxruntime_providers_shared.dll",
            "onnxruntime_providers_cuda.dll",
        ];
        
        for dll_name in &dll_names {
            let dll_path = path.join(dll_name);
            if dll_path.exists() {
                println!("  ✓ Found: {}", dll_path.display());
                dll_found = true;
                
                // Устанавливаем переменную окружения
                std::env::set_var("ORT_DYLIB_PATH", path);
                break;
            }
        }
        
        if dll_found {
            break;
        }
    }
    
    if !dll_found {
        println!("⚠️  Warning: ONNX Runtime DLLs not found in any expected location");
        println!("The ORT crate should download them automatically...");
    }
    
    // Пытаемся инициализировать ORT
    println!("\nInitializing ONNX Runtime...");
    
    match ort::init()
        .with_name("magray_test")
        .commit() {
        Ok(_) => {
            println!("✓ ORT initialized successfully!");
            
            // Пробуем создать session builder для проверки
            println!("\nCreating session builder...");
            match ort::session::Session::builder() {
                Ok(_) => println!("✓ Session builder created successfully!"),
                Err(e) => println!("✗ Failed to create session builder: {}", e),
            }
        }
        Err(e) => {
            println!("✗ Failed to initialize ORT: {}", e);
            println!("\nDebug info:");
            println!("  Current dir: {}", std::env::current_dir()?.display());
            println!("  EXE path: {}", std::env::current_exe()?.display());
            
            if let Ok(path_var) = std::env::var("PATH") {
                println!("  PATH entries:");
                for entry in path_var.split(';').take(5) {
                    println!("    - {}", entry);
                }
            }
        }
    }
    
    Ok(())
}