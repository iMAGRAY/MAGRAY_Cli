use memory::{MemoryCoordinator, MemoryConfig, MemMeta};
use memory::types::ExecutionContext;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println\!("Testing memory system...");
    
    let base_path = std::path::PathBuf::from("./test_memory");
    std::fs::create_dir_all(&base_path)?;
    std::fs::create_dir_all(base_path.join("src/Qwen3-Embedding-0.6B-ONNX"))?;
    std::fs::create_dir_all(base_path.join("src/Qwen3-Reranker-0.6B-ONNX"))?;
    
    let config = MemoryConfig {
        base_path: base_path.clone(),
        sqlite_path: base_path.join("memory.db"),
        blobs_path: base_path.join("blobs"),
        vectors_path: base_path.join("vectors"),
        cache_path: base_path.join("cache.db"),
        ..Default::default()
    };
    
    let coordinator = MemoryCoordinator::new(config).await?;
    let ctx = ExecutionContext::default();
    
    // Basic test
    let mut meta = MemMeta::default();
    meta.content_type = "text/plain".to_string();
    
    let result = coordinator.smart_put("test", b"Hello World", meta, &ctx).await?;
    println\!("Put result: {}", result.success);
    
    let retrieved = coordinator.smart_get("test", &ctx).await?;
    if let Some((data, _, _)) = retrieved {
        println\!("Retrieved: {}", String::from_utf8_lossy(&data));
    }
    
    println\!("Memory system is working\!");
    Ok(())
}
EOF < /dev/null
