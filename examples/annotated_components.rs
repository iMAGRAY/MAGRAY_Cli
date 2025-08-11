// Примеры структурированных комментариев для автогенерации документации

//! @component: UnifiedAgent
//! @file: crates/cli/src/agent.rs:6-70
//! @status: WORKING
//! @performance: O(1) routing, O(n) downstream  
//! @dependencies: LlmClient(✅), SmartRouter(⚠️), IntentAnalyzerAgent(✅)
//! @tests: ❌ No unit tests found
//! @production_ready: 60%
//! @issues: Missing error handling for LLM failures, no timeout handling
//! @upgrade_path: Add retry logic, timeout configuration, error recovery
pub struct UnifiedAgent {
    llm_client: LlmClient,
    smart_router: SmartRouter,
    intent_analyzer: IntentAnalyzerAgent,
}

//! @component: VectorStore 
//! @file: crates/memory/src/storage.rs:16-185
//! @status: WORKING
//! @performance: O(n) linear search - CRITICAL BOTTLENECK
//! @dependencies: sled(✅), bincode(✅)
//! @tests: ❌ No performance tests
//! @production_ready: 15%
//! @issues: Linear search kills performance with >1000 records
//! @upgrade_path: Implement HNSW/IVF-PQ indexes, migrate to LanceDB
//! @mock_level: none
//! @real_implementation: ✅ but inefficient
//! @bottleneck: O(n) cosine similarity for every query
//! @upgrade_effort: 5-7 days (vector index + migration)
pub struct VectorStore {
    db: Arc<sled::Db>,
}

//! @component: EmbeddingService
//! @file: crates/ai/src/embeddings.rs:20-120
//! @status: ENHANCED_MOCK
//! @performance: O(1) mock responses, real inference disabled
//! @dependencies: onnxruntime(❌), tokenizers(✅)
//! @tests: ❌ No integration tests with real models
//! @production_ready: 25%
//! @issues: Real ONNX inference commented out due to API complexity
//! @upgrade_path: Fix onnxruntime integration, add GPU support
//! @mock_level: enhanced
//! @real_implementation: ❌ Commented out
//! @bottleneck: Mock inference prevents real performance testing
//! @upgrade_effort: 3-4 days (ONNX API integration)
pub struct EmbeddingService {
    session: Arc<OnnxSession>,
    tokenizer: Option<TokenizerService>,
    config: EmbeddingConfig,
}

//! @component: PromotionEngine
//! @file: crates/memory/src/promotion.rs:11-154
//! @status: PLACEHOLDER
//! @performance: O(1) mock promotion (does nothing)
//! @dependencies: VectorStore(⚠️), PromotionConfig(✅)
//! @tests: ❌ No tests for promotion logic
//! @production_ready: 5%
//! @issues: find_promotion_candidates() returns empty Vec
//! @upgrade_path: Implement real promotion algorithms, add scoring
//! @mock_level: placeholder
//! @real_implementation: ❌ Stub implementation only
//! @bottleneck: No actual promotion happens
//! @upgrade_effort: 2-3 days (promotion logic implementation)
pub struct PromotionEngine {
    store: Arc<VectorStore>,
    config: PromotionConfig,
}

//! @component: IntentAnalyzerAgent
//! @file: crates/llm/src/agents/intent_analyzer.rs:13-103
//! @status: WORKING
//! @performance: O(1) LLM calls with JSON parsing fallbacks
//! @dependencies: LlmClient(✅), serde_json(✅)
//! @tests: ❌ No unit tests for edge cases
//! @production_ready: 75%
//! @issues: JSON parsing can fail with malformed LLM responses
//! @upgrade_path: Add structured output validation, better error handling
pub struct IntentAnalyzerAgent {
    llm: LlmClient,
}

//! @component: EmbeddingCache
//! @file: crates/memory/src/cache.rs:16-189
//! @status: WORKING
//! @performance: O(1) hash-based lookup with sled persistence
//! @dependencies: sled(✅), bincode(✅), parking_lot(✅)
//! @tests: ✅ Basic tests present
//! @production_ready: 85%
//! @issues: No cache eviction policy, grows indefinitely
//! @upgrade_path: Add LRU eviction, size limits, TTL expiration
pub struct EmbeddingCache {
    db: Arc<sled::Db>,
    stats: Arc<RwLock<CacheStats>>,
}

// Git hook integration example:
// .git/hooks/pre-commit:
// #!/bin/bash
// ./tools/sync_docs.sh
// git add CLAUDE.md

// CI/CD integration example:
// .github/workflows/doc-sync.yml:
// name: Auto-sync Documentation
// on: [push, pull_request]
// jobs:
//   sync-docs:
//     runs-on: ubuntu-latest
//     steps:
//     - uses: actions/checkout@v3
//     - name: Sync documentation
//       run: ./tools/sync_docs.sh
//       run: git diff --exit-code CLAUDE.md || echo "Documentation updated"