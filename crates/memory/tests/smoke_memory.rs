#[allow(unused_imports)]
use memory::api::{MemoryContext, MemoryServiceTrait, UnifiedMemoryAPI};
use memory::di::UnifiedContainer;
use memory::types::Layer;

#[tokio::test]
async fn smoke_remember_and_search() {
    // If ORT is not configured, skip this test to keep baseline green
    if std::env::var("ORT_DYLIB_PATH").is_err() {
        eprintln!("Skipping memory smoke: ORT_DYLIB_PATH not set");
        return;
    }

    let container = UnifiedContainer::new();
    let api = UnifiedMemoryAPI::new(std::sync::Arc::new(container));

    let id = api
        .remember(
            "hello world".to_string(),
            MemoryContext::new("test").with_layer(Layer::Insights),
        )
        .await
        .expect("remember must succeed");

    let results = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        api.recall("hello", memory::api::SearchOptions::default().limit(5))
            .await
    })
    .await
    .expect("search timeout")
    .expect("search ok");

    assert!(!id.as_bytes().is_empty());
    assert!(results.len() <= 5);
}
