use anyhow::Result;

fn main() -> Result<()> {
    println!("Testing ORT initialization...");
    
    // Устанавливаем переменную окружения для ORT
    std::env::set_var("ORT_DYLIB_PATH", "C:\\Users\\1\\Documents\\GitHub\\MAGRAY_Cli\\target\\debug");
    
    println!("ORT_DYLIB_PATH set to: {}", std::env::var("ORT_DYLIB_PATH").unwrap_or_default());
    
    // Инициализируем ORT
    println!("Initializing ORT...");
    ort::init().commit()?;
    
    println!("✓ ORT initialized successfully!");
    
    // Создаём простой Session builder для проверки
    println!("Creating session builder...");
    let builder = ort::session::Session::builder()?;
    println!("✓ Session builder created successfully!");
    
    Ok(())
}