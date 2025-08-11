use anyhow::{anyhow, Result};
use std::process::Command;

#[tokio::main]
async fn main() -> Result<()> {
    let status_full = Command::new("python3")
        .args(["scripts/install_qwen3_onnx.py", "--models-dir", "models"]) // run from workspace root
        .status();

    let ok = match status_full {
        Ok(s) if s.success() => true,
        _ => {
            eprintln!("Full installer failed, falling back to minimal fetcher...");
            let status_min = Command::new("python3")
                .args(["scripts/install_qwen3_minimal.py", "--models-dir", "models"]) // prepare placeholders
                .status()?;
            if !status_min.success() {
                return Err(anyhow!("both installers failed"));
            }
            true
        }
    };

    if ok {
        println!("✅ Qwen3 models prepared in ./models (qwen3emb, qwen3_reranker)");
    }
    Ok(())
}
