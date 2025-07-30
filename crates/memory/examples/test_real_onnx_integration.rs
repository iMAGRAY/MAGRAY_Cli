use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, warn, error};
use tracing_subscriber;

use memory::{
    MemoryConfig, MemoryService,
    Layer, Record,
};
use ai::{AiConfig, EmbeddingConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Testing real ONNX integration with memory system");

    // Set ONNX Runtime DLL path for Windows
    #[cfg(target_os = "windows")]
    {
        let dll_path = std::env::current_dir()?
            .join("scripts/onnxruntime/lib/onnxruntime.dll");
        if dll_path.exists() {
            std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
            info!("✅ Set ORT_DYLIB_PATH: {}", dll_path.display());
        } else {
            warn!("ONNX Runtime DLL not found at: {}", dll_path.display());
        }
    }

    // Initialize ONNX Runtime
    ort::init().commit()?;
    info!("✅ ONNX Runtime initialized");

    // Configure with real models
    let models_dir = PathBuf::from("crates/memory/models");
    if !models_dir.exists() {
        error!("Models directory not found at: {:?}", models_dir);
        error!("Expected models in: crates/memory/models/");
        return Ok(());
    }

    let memory_config = MemoryConfig {
        db_path: PathBuf::from("test_onnx_db"),
        cache_path: PathBuf::from("test_onnx_cache"),
        promotion: Default::default(),
        ai_config: AiConfig {
            models_dir: models_dir.clone(),
            embedding: EmbeddingConfig {
                model_name: "Qwen3-Embedding-0.6B-ONNX".to_string(),
                batch_size: 8,
                max_length: 512,
                use_gpu: false,
            },
            reranking: Default::default(),
        },
    };

    // Initialize memory service
    info!("Initializing memory service with real ONNX models...");
    let memory_service = match MemoryService::new(memory_config).await {
        Ok(service) => {
            info!("✅ Memory service initialized successfully with real ONNX!");
            service
        }
        Err(e) => {
            error!("❌ Failed to initialize memory service: {}", e);
            return Ok(());
        }
    };

    // Test data
    let test_records = vec![
        "The quick brown fox jumps over the lazy dog",
        "Machine learning is a subset of artificial intelligence",
        "Rust is a systems programming language focused on safety",
        "Vector databases enable semantic search capabilities",
        "ONNX provides interoperability between ML frameworks",
    ];

    // Insert test records
    info!("\nInserting {} test records...", test_records.len());
    for (i, text) in test_records.iter().enumerate() {
        let record = Record {
            id: uuid::Uuid::new_v4(),
            text: text.to_string(),
            layer: Layer::Interact,
            kind: "test".to_string(),
            tags: vec![format!("test-{}", i)],
            project: "onnx-test".to_string(),
            session: "test-session".to_string(),
            embedding: vec![], // Will be computed by service
            ts: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
            access_count: 0,
            score: 0.0,
        };

        match memory_service.insert(record).await {
            Ok(_) => info!("  ✅ Inserted: \"{}\"", &text[..40.min(text.len())]),
            Err(e) => error!("  ❌ Failed to insert: {}", e),
        }
    }

    // Test search
    let queries = vec![
        "programming language",
        "artificial intelligence",
        "database search",
    ];

    info!("\nTesting semantic search with real embeddings:");
    for query in queries {
        info!("\n🔍 Searching for: \"{}\"", query);
        
        match memory_service.search(query)
            .with_layer(Layer::Interact)
            .top_k(3)
            .execute()
            .await 
        {
            Ok(results) => {
                if results.is_empty() {
                    warn!("  No results found");
                } else {
                    for (i, result) in results.iter().enumerate() {
                        info!("  {}. [score: {:.3}] \"{}\"", 
                            i + 1, 
                            result.score, 
                            &result.text[..60.min(result.text.len())]
                        );
                    }
                }
            }
            Err(e) => error!("  Search failed: {}", e),
        }
    }

    // Check cache stats
    let (entries, hits, misses) = memory_service.cache_stats();
    let hit_rate = memory_service.cache_hit_rate();
    info!("\n📊 Cache Statistics:");
    info!("  Entries: {}", entries);
    info!("  Hits: {}", hits);
    info!("  Misses: {}", misses);
    info!("  Hit rate: {:.1}%", hit_rate * 100.0);

    // Cleanup
    std::fs::remove_dir_all("test_onnx_db").ok();
    std::fs::remove_dir_all("test_onnx_cache").ok();

    info!("\n✅ Real ONNX integration test completed!");

    Ok(())
}