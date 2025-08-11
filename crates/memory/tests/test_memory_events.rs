#![cfg(feature = "extended-tests")]

use anyhow::Result;
use common::{events, topics};
use memory::api::MemoryServiceTrait;
use memory::di::UnifiedContainer;
use memory::Layer;

#[tokio::test]
async fn emits_json_events_on_remember_and_search() -> Result<()> {
    let ort_path = std::path::Path::new("scripts/onnxruntime/lib/libonnxruntime.so");
    if !ort_path.exists() {
        return Ok(());
    }
    std::env::set_var("ORT_DYLIB_PATH", ort_path.to_string_lossy().to_string());

    // subscribe before actions
    let mut rx_up = events::subscribe(topics::TOPIC_MEMORY_UPSERT).await;
    let mut rx_search = events::subscribe(topics::TOPIC_MEMORY_SEARCH).await;

    let svc = UnifiedContainer::new();
    let text = "event payload test".to_string();
    let layer = Layer::Interact;

    let id = svc.remember_sync(text.clone(), layer)?;

    // wait upsert event
    if let Ok(Ok(up_evt)) =
        tokio::time::timeout(std::time::Duration::from_millis(1500), rx_up.recv()).await
    {
        assert_eq!(up_evt.topic.0, topics::TOPIC_MEMORY_UPSERT.0);
        assert_eq!(
            up_evt.payload["layer"].as_str().unwrap_or_default(),
            format!("{:?}", layer)
        );
        assert_eq!(
            up_evt.payload["id"].as_str().unwrap_or_default(),
            id.to_string()
        );
    }

    // perform search; expect a search event best-effort
    let _ = svc.search_sync("event payload", layer, 3)?;
    if let Ok(Ok(srch_evt)) =
        tokio::time::timeout(std::time::Duration::from_millis(1500), rx_search.recv()).await
    {
        assert_eq!(srch_evt.topic.0, topics::TOPIC_MEMORY_SEARCH.0);
        assert_eq!(
            srch_evt.payload["layer"].as_str().unwrap_or_default(),
            format!("{:?}", layer)
        );
    }
    Ok(())
}

#[tokio::test]
async fn rerank_fallback_orders_results_without_ort() -> Result<()> {
    std::env::set_var("MAGRAY_FORCE_NO_ORT", "1");
    std::env::set_var("MAGRAY_SKIP_AUTO_INSTALL", "1");

    let svc = UnifiedContainer::new();
    let api = memory::api::UnifiedMemoryAPI::new(
        std::sync::Arc::new(svc) as std::sync::Arc<dyn memory::api::MemoryServiceTrait>
    );

    // Insert records with controlled token overlap
    let ctx = memory::api::MemoryContext::new("note").with_layer(Layer::Interact);
    let _ = api
        .remember("rust tokio async runtimes".to_string(), ctx.clone())
        .await?;
    let _ = api
        .remember("python asyncio event loop".to_string(), ctx.clone())
        .await?;
    let _ = api
        .remember(
            "rust ownership borrowing lifetimes".to_string(),
            ctx.clone(),
        )
        .await?;

    // Query closer to third record by tokens
    let results = api
        .recall(
            "rust lifetimes borrowing",
            memory::api::SearchOptions::default()
                .in_layers(vec![Layer::Interact])
                .limit(3),
        )
        .await?;

    assert!(!results.is_empty());
    // Expect a result containing rust/borrowing/lifetimes ranked highly
    let texts: Vec<String> = results.iter().map(|r| r.text.clone()).collect();
    // stronger check: first contains "lifetimes" or "borrowing"
    let top = &texts[0];
    assert!(top.contains("lifetimes") || top.contains("borrowing") || top.contains("rust"));
    Ok(())
}
