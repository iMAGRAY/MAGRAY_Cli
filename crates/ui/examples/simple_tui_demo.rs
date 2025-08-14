use ui::tui::TUIApp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🖥  Starting MAGRAY TUI Demo...");
    
    // Создаем TUI приложение
    let mut app = TUIApp::new()?;
    
    println!("🚀 TUI initialized successfully. Press 'q' to quit, 'h' for help.");
    
    // Запускаем TUI
    if let Err(e) = app.run() {
        eprintln!("TUI error: {}", e);
        return Err(e);
    }
    
    println!("👋 TUI demo session ended.");
    Ok(())
}