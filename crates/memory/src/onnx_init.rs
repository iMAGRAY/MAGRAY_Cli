use std::sync::Once;
use anyhow::Result;
use std::path::PathBuf;

static INIT: Once = Once::new();
static mut INIT_RESULT: Option<Result<()>> = None;

/// Инициализирует ONNX Runtime один раз для всего приложения
pub fn ensure_ort_initialized() -> Result<()> {
    unsafe {
        INIT.call_once(|| {
            INIT_RESULT = Some(initialize_ort());
        });
        
        INIT_RESULT.as_ref().unwrap().as_ref().map(|_| ()).map_err(|e| anyhow::anyhow!("{}", e))
    }
}

fn initialize_ort() -> Result<()> {
    // Сначала проверяем наличие загруженных DLL
    let possible_paths = vec![
        PathBuf::from("target/ort-libs"),
        PathBuf::from("target/debug"),
        PathBuf::from("target/debug/deps"),
        PathBuf::from("target/release"),
        PathBuf::from("target/release/deps"),
        // Абсолютные пути для тестов
        PathBuf::from("C:\\Users\\1\\Documents\\GitHub\\MAGRAY_Cli\\crates\\memory\\target\\ort-libs"),
        PathBuf::from("C:\\Users\\1\\Documents\\GitHub\\MAGRAY_Cli\\target\\debug"),
    ];
    
    // Ищем директорию с onnxruntime.dll
    let mut ort_path = None;
    for path in &possible_paths {
        if path.join("onnxruntime.dll").exists() {
            println!("Found ONNX Runtime DLLs at: {}", path.display());
            ort_path = Some(path.clone());
            break;
        }
    }
    
    if let Some(path) = ort_path {
        // Устанавливаем переменную окружения для ORT
        let canonical_path = path.canonicalize().unwrap_or(path);
        std::env::set_var("ORT_DYLIB_PATH", canonical_path.join("onnxruntime.dll"));
        
        // Добавляем путь в начало PATH для Windows
        if let Ok(current_path) = std::env::var("PATH") {
            let new_path = format!("{};{}", canonical_path.display(), current_path);
            std::env::set_var("PATH", new_path);
        }
    } else {
        println!("Warning: ONNX Runtime DLLs not found in expected locations");
        println!("Please run download_ort.ps1 to download ONNX Runtime 1.20.0");
    }
    
    // Инициализируем ORT
    println!("Initializing ONNX Runtime...");
    
    match ort::init()
        .with_name("magray_cli")
        .commit() {
        Ok(_) => {
            println!("ONNX Runtime initialized successfully");
            Ok(())
        }
        Err(e) => {
            Err(anyhow::anyhow!("Failed to initialize ONNX Runtime: {}. Make sure you have the correct version (1.20.x) installed.", e))
        }
    }
}