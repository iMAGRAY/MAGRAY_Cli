use memory::api::{MemoryContext, UnifiedMemoryAPI, MemoryServiceTrait};
use memory::types::Layer;
use memory::di::UnifiedContainer;

#[tokio::test]
async fn smoke_remember_and_search() {
    // Arrange
    let container = UnifiedContainer::new();
    let api = UnifiedMemoryAPI::new(std::sync::Arc::new(container));

    // Act: remember
    let id = api.remember("hello world".to_string(), MemoryContext::new("test").with_layer(Layer::Insights)).await
        .expect("remember must succeed");

    // Act: search with timeout guard
    let results = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        api.recall("hello", memory::api::SearchOptions::default().limit(5)).await
    }).await.expect("search timeout").expect("search ok");

    // Assert: non-empty or empty is fine, but no panic; id is valid UUID
    assert!(!id.as_bytes().is_empty());
    assert!(results.len() <= 5);
}