# LayeredMemory - Storage Architecture

#memory #layered-architecture #storage #index #query #cache #decomposition

> **Статус**: 95% готов | **Архитектурная декомпозиция**: God Object → Layered Clean Architecture

## 📋 Трансформация архитектуры

[[DIMemoryService]] (1482 строки монолита) → [[LayeredMemory]] представляет собой кардинальную декомпозицию на специализированные слои с четким разделением ответственности.

### 🎯 Достижения декомпозиции

- ✅ **Single Responsibility**: Каждый слой имеет одну четкую ответственность
- ✅ **Dependency Inversion**: Все слои зависят от абстракций, не от конкретных реализаций  
- ✅ **Interface Segregation**: Минимальные интерфейсы для каждого слоя
- ✅ **Open/Closed**: Легко расширяемые слои без модификации существующих
- ✅ **Liskov Substitution**: Любая реализация слоя может заменить другую
- 🔄 **Backward Compatibility**: Wrapper для smooth migration

## 🏗️ Архитектура слоев

```mermaid
graph TB
    subgraph "🎯 Query Layer (Business Logic)"
        A[SemanticQueryLayer] --> B[High-level Search]
        A --> C[Context Assembly]
        A --> D[Result Ranking]
        A --> E[Promotion Logic]
    end
    
    subgraph "🚀 Cache Layer (Performance)"
        F[LRUCacheLayer] --> G[Embedding Cache]
        F --> H[Query Cache]  
        F --> I[TTL Management]
        F --> J[Cache Analytics]
    end
    
    subgraph "📊 Index Layer (Vectors)"
        K[HNSWIndexLayer] --> L[Vector Operations]
        K --> M[[[HNSW Ultra-Performance]]]
        K --> N[Batch Processing]
        K --> O[SIMD Optimization]
    end
    
    subgraph "💾 Storage Layer (Persistence)"
        P[SQLiteStorageLayer] --> Q[CRUD Operations]
        P --> R[Batch Operations]
        P --> S[Transaction Management]
        P --> T[Backup/Restore]
    end
    
    subgraph "🎭 Layer Coordination"  
        U[LayerOrchestrator] --> A
        U --> F
        U --> K
        U --> P
        U --> V[Health Monitoring]
        U --> W[Circuit Breakers]
    end
    
    A --> F
    F --> K  
    K --> P
    U --> X[LayeredMemoryBuilder]
    
    style A fill:#e8f5e9,stroke:#4caf50,stroke-width:3px
    style F fill:#fff3e0,stroke:#ff9800,stroke-width:3px
    style K fill:#e1f5fe,stroke:#2196f3,stroke-width:3px
    style P fill:#f3e5f5,stroke:#9c27b0,stroke-width:3px
    style U fill:#ffebee,stroke:#f44336,stroke-width:3px
```

## 🔧 Реализация слоев

### 💾 Storage Layer - Persistence

**Файл**: `crates/memory/src/layers/storage.rs`

Ответственность: Физическое хранение данных без знания о бизнес-логике

```rust
#[async_trait]  
pub trait StorageLayer: Send + Sync {
    /// Сохранить запись в хранилище
    async fn store(&self, record: &Record) -> Result<()>;
    
    /// Batch операции для производительности
    async fn store_batch(&self, records: &[&Record]) -> Result<usize>;
    
    /// Транзакционные операции
    async fn transaction<F, R>(&self, operation: F) -> Result<R>
    where F: FnOnce(&Transaction) -> Result<R>;
    
    /// Backup операции
    async fn backup(&self, path: &str) -> Result<BackupMetadata>;
}

pub struct SQLiteStorageLayer {
    pool: Pool<Sqlite>,
    metrics: Arc<StorageMetrics>,
}
```

### 📊 Index Layer - Vector Operations

**Файл**: `crates/memory/src/layers/index.rs` 

Ответственность: Векторные операции и индексирование

```rust
#[async_trait]
pub trait IndexLayer: Send + Sync {
    /// Добавить вектор в индекс
    async fn add_vector(&self, id: Uuid, embedding: &[f32], layer: Layer) -> Result<()>;
    
    /// Векторный поиск с HNSW
    async fn search_vectors(
        &self, 
        query: &[f32], 
        layer: Layer,
        k: usize
    ) -> Result<Vec<ScoredRecord>>;
    
    /// Batch векторные операции  
    async fn batch_add(&self, vectors: &[(Uuid, Vec<f32>, Layer)]) -> Result<()>;
}

pub struct HNSWIndexLayer {
    indices: HashMap<Layer, HnswIndex>,
    simd_optimizer: Arc<SIMDOptimizer>,
    gpu_accelerator: Option<Arc<GPUAccelerator>>,
}
```

### 🚀 Cache Layer - Performance Optimization

**Файл**: `crates/memory/src/layers/cache.rs`

Ответственность: Кэширование для производительности

```rust
#[async_trait]
pub trait CacheLayer: Send + Sync {
    /// Получить кэшированный embedding
    async fn get_embedding(&self, text_hash: &str) -> Result<Option<Vec<f32>>>;
    
    /// Кэшировать embedding с TTL
    async fn cache_embedding(&self, text_hash: String, embedding: Vec<f32>) -> Result<()>;
    
    /// Cache analytics
    fn get_cache_stats(&self) -> CacheStats;
    
    /// Adaptive TTL на основе layer
    fn calculate_ttl(&self, layer: Layer) -> Duration;
}

pub struct LRUCacheLayer {
    embedding_cache: Arc<Mutex<LruCache<String, CachedEmbedding>>>,
    query_cache: Arc<Mutex<LruCache<String, CachedQueryResult>>>,
    analytics: Arc<CacheAnalytics>,
}
```

### 🎯 Query Layer - Business Logic

**Файл**: `crates/memory/src/layers/query.rs`

Ответственность: Высокоуровневая бизнес-логика поиска

```rust
#[async_trait]
pub trait QueryLayer: Send + Sync {
    /// Семантический поиск с контекстом
    async fn semantic_search(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>>;
    
    /// Multi-layer поиск
    async fn search_across_layers(&self, query: &str) -> Result<LayeredSearchResult>;
    
    /// Context assembly для LLM
    async fn assemble_context(&self, results: &[SearchResult]) -> Result<String>;
    
    /// Intelligent reranking
    async fn rerank_results(&self, results: &mut [SearchResult], query: &str) -> Result<()>;
}

pub struct SemanticQueryLayer {
    storage: Arc<dyn StorageLayer>,
    index: Arc<dyn IndexLayer>, 
    cache: Arc<dyn CacheLayer>,
    embedding_service: Arc<dyn EmbeddingService>,
    reranker: Option<Arc<dyn RerankerService>>,
}
```

## 🎯 Layer Orchestrator

**Файл**: `crates/memory/src/layers/orchestrator.rs`

Центральный координатор всех слоев:

```rust
pub struct LayerOrchestrator {
    query_layer: Arc<dyn QueryLayer>,
    cache_layer: Arc<dyn CacheLayer>,
    index_layer: Arc<dyn IndexLayer>,
    storage_layer: Arc<dyn StorageLayer>,
    health_monitor: Arc<HealthManager>,
    circuit_breakers: HashMap<&'static str, CircuitBreaker>,
}

impl LayerOrchestrator {
    /// Полный workflow: Query → Cache → Index → Storage
    pub async fn execute_search(&self, query: &str) -> Result<SearchResult> {
        // Circuit breaker protection
        self.circuit_breakers.get("search")
            .unwrap()
            .execute(async {
                self.query_layer.semantic_search(query, &SearchOptions::default()).await
            })
            .await
    }
    
    /// Health check всех слоев
    pub async fn health_check(&self) -> LayeredHealthStatus {
        tokio::join!(
            self.query_layer.health_check(),
            self.cache_layer.health_check(),  
            self.index_layer.health_check(),
            self.storage_layer.health_check()
        ).into()
    }
}
```

## 🏭 Builder Pattern

**Файл**: `crates/memory/src/layers/mod.rs`

Удобное создание полной архитектуры:

```rust
pub struct LayeredMemoryBuilder {
    storage_config: StorageConfig,
    index_config: IndexConfig,
    cache_config: CacheConfig,
    query_config: QueryConfig,
}

impl LayeredMemoryBuilder {
    pub fn new() -> Self { /* ... */ }
    
    pub fn with_sqlite_storage(mut self, config: SqliteConfig) -> Self {
        self.storage_config = StorageConfig::SQLite(config);
        self
    }
    
    pub fn with_hnsw_index(mut self, config: HnswConfig) -> Self {
        self.index_config = IndexConfig::HNSW(config);
        self
    }
    
    pub fn with_lru_cache(mut self, config: LruConfig) -> Self {
        self.cache_config = CacheConfig::LRU(config);
        self
    }
    
    pub async fn build(self) -> Result<LayeredMemoryService> {
        let storage = self.build_storage_layer().await?;
        let index = self.build_index_layer().await?;
        let cache = self.build_cache_layer().await?;
        let query = self.build_query_layer(storage.clone(), index.clone(), cache.clone()).await?;
        
        let orchestrator = LayerOrchestrator::new(query, cache, index, storage).await?;
        
        Ok(LayeredMemoryService::new(orchestrator))
    }
}
```

## 📊 Performance Improvements

### До декомпозиции (DIMemoryService):
- **Размер файла**: 1,482 строки монолита
- **Cyclomatic Complexity**: 95+ (крайне высокая)  
- **Тестируемость**: Монолитный, сложно тестировать изолированно
- **Расширяемость**: Модификация затрагивает весь класс
- **Производительность**: Нет специализированного кэширования

### После декомпозиции (LayeredMemory):
- **Размер компонентов**: 4 слоя по 200-400 строк каждый
- **Cyclomatic Complexity**: <10 для каждого слоя
- **Тестируемость**: Полная изоляция через traits
- **Расширяемость**: Каждый слой расширяется независимо  
- **Производительность**: Специализированный LRU cache, batch operations

## 🔄 Migration Strategy

### Phase 1: Backward Compatibility Wrapper

```rust
// Полная совместимость для существующего кода
pub struct DIMemoryService {
    layered_service: LayeredMemoryService,
}

impl DIMemoryService {
    pub fn new() -> Result<Self> {
        Ok(Self {
            layered_service: LayeredMemoryBuilder::new()
                .with_sqlite_storage(SqliteConfig::default())
                .with_hnsw_index(HnswConfig::default())
                .with_lru_cache(LruConfig::default())
                .build()
                .await?,
        })
    }
    
    // Все legacy методы делегируют в LayeredMemory
    pub async fn store(&self, record: &Record) -> Result<()> {
        self.layered_service.store(record).await
    }
    
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        self.layered_service.search(query).await
    }
}
```

## 🔗 Интеграции

- **[[UnifiedAgentV2]]**: Использует LayeredMemory через MemoryHandler
- **[[HNSW Ultra-Performance]]**: IndexLayer интегрирован с SIMD optimizations
- **[[Multi-Provider LLM]]**: QueryLayer использует embedding services
- **[[Production CI/CD]]**: Тестирование всех слоев в isolation

## 🎯 Следующие шаги

1. **Завершить Backward Compatibility** - 100% API compatibility
2. **Performance Benchmarking** - Измерить улучшения производительности
3. **Advanced Caching Strategies** - Intelligent cache eviction
4. **Monitoring Integration** - Detailed metrics для каждого слоя
5. **Documentation** - Comprehensive API documentation

---

*Последнее обновление: 06.08.2025 | Создано: obsidian-docs-architect*