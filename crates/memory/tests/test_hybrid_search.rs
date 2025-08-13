#![cfg(all(
    feature = "extended-tests",
    feature = "orchestration-modules",
    feature = "keyword-search"
))]

use memory::di::core_traits::ServiceResolver;
use memory::orchestration::traits::SearchCoordinator as SearchCoordinatorTrait;
use memory::orchestration::Coordinator;
use memory::orchestration::SearchCoordinator;
use memory::{
    storage::VectorStore,
    types::{Layer, Record, SearchOptions},
};
use std::sync::Arc;

#[tokio::test]
async fn hybrid_prefers_keyword_when_query_matches() {
    // Prepare store
    let temp = tempfile::TempDir::new().expect("Test operation should succeed");
    let store = Arc::new(
        VectorStore::with_config(&temp.path(), memory::HnswRsConfig::default())
            .await
            .expect("Test operation should succeed"),
    );

    // Insert records
    let rec1 = Record {
        id: uuid::Uuid::new_v4(),
        text: "rust ownership borrowing lifetimes".to_string(),
        embedding: vec![0.0; 1024],
        layer: Layer::Interact,
        kind: "note".into(),
        tags: vec![],
        project: "p".into(),
        session: "s".into(),
        ts: chrono::Utc::now(),
        score: 0.0,
        access_count: 0,
        last_access: chrono::Utc::now(),
    };
    let rec2 = Record {
        id: uuid::Uuid::new_v4(),
        text: "python asyncio event loop".to_string(),
        embedding: vec![0.0; 1024],
        layer: Layer::Interact,
        kind: "note".into(),
        tags: vec![],
        project: "p".into(),
        session: "s".into(),
        ts: chrono::Utc::now(),
        score: 0.0,
        access_count: 0,
        last_access: chrono::Utc::now(),
    };
    store
        .insert(&rec1)
        .await
        .expect("Test operation should succeed");
    store
        .insert(&rec2)
        .await
        .expect("Test operation should succeed");

    let container = memory::di::UnifiedContainer::new();
    // Resolve embedding coordinator dependency indirectly via SearchCoordinator::new_production requires real EmbeddingCoordinator
    // We need a dummy EmbeddingCoordinator; UnifiedContainer resolves it in orchestrated profile
    let embedding = container
        .resolve::<memory::orchestration::EmbeddingCoordinator>()
        .expect("Test operation should succeed");

    let coord = SearchCoordinator::new_production(store.clone(), embedding, 8, 128);
    coord
        .initialize()
        .await
        .expect("Test operation should succeed");

    let opts = SearchOptions {
        top_k: 2,
        ..Default::default()
    };
    let out = SearchCoordinatorTrait::hybrid_search(
        &coord,
        "rust lifetimes",
        None,
        Layer::Interact,
        opts,
    )
    .await
    .expect("Test operation should succeed");
    let texts: Vec<String> = out.iter().map(|r| r.text.clone()).collect();
    assert!(
        texts
            .iter()
            .any(|t| t.contains("rust ownership borrowing lifetimes")),
        "{:?}",
        texts
    );
}
