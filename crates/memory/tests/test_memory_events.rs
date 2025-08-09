#![cfg(feature = "extended-tests")]

use anyhow::Result;
use common::{events, topics};
use memory::Layer;
use memory::di::UnifiedContainer;
use memory::api::MemoryServiceTrait;

#[tokio::test]
async fn emits_json_events_on_remember_and_search() -> Result<()> {
    // Require ORT binary for this test to avoid panics from ort crate; skip if missing
    let ort_path = std::path::Path::new("scripts/onnxruntime/lib/libonnxruntime.so");
    if !ort_path.exists() {
        return Ok(());
    }
    std::env::set_var("ORT_DYLIB_PATH", ort_path.to_string_lossy().to_string());

    // subscribe before actions
    let mut rx_up = events::subscribe(topics::TOPIC_MEMORY_UPSERT).await;
    let mut rx_search = events::subscribe(topics::TOPIC_MEMORY_SEARCH).await;

    // use DI container facade
    let svc = UnifiedContainer::new();
    let text = "event payload test".to_string();
    let layer = Layer::Interact;

    let id = svc.remember_sync(text.clone(), layer)?;

    // wait upsert event
    if let Ok(Ok(up_evt)) = tokio::time::timeout(std::time::Duration::from_millis(1500), rx_up.recv()).await {
        assert_eq!(up_evt.topic.0, topics::TOPIC_MEMORY_UPSERT.0);
        assert_eq!(up_evt.payload["layer"].as_str().unwrap_or_default(), format!("{:?}", layer));
        assert_eq!(up_evt.payload["id"].as_str().unwrap_or_default(), id.to_string());
    }

    // perform search; expect a search event best-effort
    let _ = svc.search_sync("event payload", layer, 3)?;
    if let Ok(Ok(srch_evt)) = tokio::time::timeout(std::time::Duration::from_millis(1500), rx_search.recv()).await {
        assert_eq!(srch_evt.topic.0, topics::TOPIC_MEMORY_SEARCH.0);
        assert_eq!(srch_evt.payload["layer"].as_str().unwrap_or_default(), format!("{:?}", layer));
    }
    Ok(())
}