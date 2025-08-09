use anyhow::Result;
use ai::model_downloader::ModelDownloader;
#[tokio::main]
async fn main() -> Result<()> {
    let d = ModelDownloader::new("models")?;
    for m in ["bge-m3", "bge-reranker-v2-m3"] { let _ = d.ensure_model(m).await?; }
    println!("✅ ONNX models installed to ./models");
    Ok(())
}
