use anyhow::Result;

#[cfg(not(feature = "use_real_onnx"))]
fn main() {
    println!("This example requires the 'use_real_onnx' feature");
    println!("Run with: cargo run --example download_models --features use_real_onnx");
}

#[cfg(feature = "use_real_onnx")]
#[tokio::main]
async fn main() -> Result<()> {
    use memory::model_downloader::{ModelDownloader, ModelDownloadConfig};
    
    // Инициализируем логирование
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    println!("ONNX Model Downloader for MAGRAY CLI");
    println!("=====================================\n");
    
    let downloader = ModelDownloader::new()?;
    
    // Конфигурации моделей
    let configs = vec![
        ModelDownloadConfig::qwen3_embedding(),
        ModelDownloadConfig::qwen3_reranker(),
    ];
    
    for config in configs {
        println!("\nChecking model: {}", config.model_name);
        println!("Repository: {}", config.repo_id);
        println!("Target directory: {}", config.target_dir.display());
        
        if downloader.needs_download(&config).await? {
            println!("Model needs to be downloaded.");
            println!("This will download approximately 1GB of data.");
            println!("Press Enter to continue or Ctrl+C to cancel...");
            
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            
            match downloader.download(&config).await {
                Ok(_) => println!("✓ Model downloaded successfully!"),
                Err(e) => {
                    eprintln!("✗ Failed to download model: {}", e);
                    eprintln!("Error details: {:?}", e);
                }
            }
        } else {
            println!("✓ Model already downloaded and complete");
        }
    }
    
    println!("\nAll done!");
    Ok(())
}