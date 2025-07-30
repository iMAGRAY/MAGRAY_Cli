# Memory Crate - MAGRAY CLI

Высокопроизводительная система векторной памяти для AI агентов с HNSW индексированием и многослойной архитектурой.

## 🚀 Основные возможности

- **HNSW Vector Search**: O(log n) поиск по векторам через `hnsw_rs`
- **3-слойная архитектура**: Interact → Insights → Assets
- **Time-based Promotion**: Автоматическое продвижение важных данных
- **BGE-M3 Embeddings**: 1024-мерные векторы с поддержкой русского языка
- **BGE Reranker v2-m3**: Семантическое переранжирование результатов
- **Health Monitoring**: Real-time мониторинг всех компонентов
- **Unified API**: Простой интерфейс для всех операций

## 📦 Установка

```toml
[dependencies]
memory = { path = "../memory" }
```

## 🔧 Быстрый старт

### Базовое использование

```rust
use memory::{MemoryConfig, MemoryService, UnifiedMemoryAPI, MemoryContext};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Создаём конфигурацию
    let config = MemoryConfig::default();
    
    // Инициализируем сервис
    let service = Arc::new(MemoryService::new(config).await?);
    
    // Создаём удобный API
    let api = UnifiedMemoryAPI::new(service);
    
    // Сохраняем информацию
    let id = api.remember(
        "Rust - системный язык программирования".to_string(),
        MemoryContext::new("knowledge")
            .with_tags(vec!["rust", "programming"])
            .with_project("learning")
    ).await?;
    
    println!("Сохранено с ID: {}", id);
    
    // Ищем релевантную информацию
    let results = api.recall("язык rust", Default::default()).await?;
    
    for result in results {
        println!("Найдено: {} (релевантность: {:.2})", 
                 result.text, result.relevance_score);
    }
    
    Ok(())
}
```

### Продвинутое использование

```rust
use memory::{MemoryService, Layer, SearchOptions};

// Поиск с фильтрами
let results = memory_service
    .search("machine learning")
    .with_layers(&[Layer::Insights, Layer::Assets])
    .with_tags(vec!["ai".to_string()])
    .min_score(0.7)
    .top_k(10)
    .execute()
    .await?;

// Прямая работа со слоями
let record = Record {
    id: Uuid::new_v4(),
    text: "Важное открытие в области ИИ".to_string(),
    embedding: vec![], // Будет заполнено автоматически
    layer: Layer::Insights, // Сразу в важный слой
    kind: "discovery".to_string(),
    tags: vec!["ai", "breakthrough"],
    // ... остальные поля
};

memory_service.insert(record).await?;
```

## 🏗️ Архитектура

### Слои памяти

| Слой | Назначение | TTL | Описание |
|------|------------|-----|----------|
| **Interact** | Текущая сессия | 24ч | Временные данные, диалоги |
| **Insights** | Извлечённые знания | 90д | Важные факты и решения |
| **Assets** | Долгосрочное хранение | ∞ | Код, документация, ключевые данные |

### Promotion Engine

```rust
// Автоматическое продвижение данных между слоями
let stats = api.optimize_memory().await?;
println!("Продвинуто в Insights: {}", stats.promoted_to_insights);
println!("Продвинуто в Assets: {}", stats.promoted_to_assets);
```

Promotion работает на основе:
- **Возраста записи**: старые записи кандидаты на продвижение
- **Score**: только записи с высокой релевантностью
- **Access count**: часто используемые записи важнее
- **Time indices**: O(log n) поиск кандидатов через BTreeMap

## 🔍 Vector Search

### HNSW параметры

```rust
use memory::{HnswRsConfig, VectorIndexHnswRs};

let config = HnswRsConfig {
    m: 16,              // Связность графа (больше = точнее, но медленнее)
    ef_c: 200,          // Качество построения индекса
    max_nb_connection: 64, // Макс. связей на узел
    n_threads: 4,       // Потоки для построения
    max_layer: 16,      // Глубина иерархии
    show_progress: true,
};

let index = VectorIndexHnswRs::new(config, 1024); // 1024 = размерность BGE-M3
```

### Производительность

- **Построение индекса**: ~1000 векторов/сек
- **Поиск**: ~5ms для 10K векторов (top-10)
- **Вставка**: ~0.1ms на вектор
- **Память**: ~200 байт на вектор overhead

## 🤖 AI Интеграция

### BGE-M3 Embeddings

```rust
use ai::{OptimizedEmbeddingService, EmbeddingConfig};

let config = EmbeddingConfig {
    model_name: "bge-m3".to_string(),
    max_length: 512,
    batch_size: 32,
    use_gpu: false, // Или true если есть CUDA
};

let embedding_service = OptimizedEmbeddingService::new(config)?;
let embedding = embedding_service.embed("Текст для векторизации")?;
// embedding: Vec<f32> размером 1024
```

### BGE Reranker v2-m3

```rust
use ai::{RerankingService, RerankingConfig};

let config = RerankingConfig {
    model_name: "bge-reranker-v2-m3".to_string(),
    max_length: 512,
    batch_size: 8,
    use_gpu: false,
};

let reranker = RerankingService::new(&config)?;
let reranked = reranker.rerank(
    "поисковый запрос",
    &["документ 1", "документ 2", "документ 3"]
)?;
```

## 🏥 Health Monitoring

```rust
// Проверка здоровья системы
let health = api.health_check().await?;
println!("Статус: {}", health.status);
println!("Компоненты: {:?}", health.components);

// Детальная диагностика
let detailed = api.full_health_check().await?;
for alert in detailed.alerts {
    println!("[{}] {}: {}", alert.severity, alert.component, alert.message);
}

// Получение метрик компонента
let vector_health = memory_service.get_component_health(ComponentType::VectorStore);
if let Some(stats) = vector_health {
    println!("Vector Store - Success rate: {:.1}%", stats.success_rate * 100.0);
    println!("Avg response time: {:.2}ms", stats.avg_response_time_ms);
}
```

## 📊 Конфигурация

### Полная конфигурация

```rust
use memory::{MemoryConfig, PromotionConfig, HealthConfig};
use std::path::PathBuf;

let config = MemoryConfig {
    // Пути к данным
    db_path: PathBuf::from("./data/magray_memory"),
    cache_path: PathBuf::from("./data/cache"),
    
    // Настройки promotion
    promotion: PromotionConfig {
        interact_ttl_hours: 24,
        insights_ttl_days: 90,
        promote_threshold: 0.7,
        decay_factor: 0.9,
    },
    
    // AI конфигурация
    ai_config: AiConfig {
        models_dir: PathBuf::from("./models"),
        embedding: EmbeddingConfig { /* ... */ },
        reranking: RerankingConfig { /* ... */ },
    },
    
    // Health monitoring
    health_config: HealthConfig {
        check_interval_secs: 60,
        metrics_retention_secs: 3600,
        alert_thresholds: /* ... */,
    },
};
```

### Environment переменные

```bash
# Основные
RUST_LOG=debug
MAGRAY_DATA_DIR=/path/to/data

# AI модели
MAGRAY_MODELS_DIR=/path/to/models
MAGRAY_USE_GPU=false

# Производительность
MAGRAY_BATCH_SIZE=32
MAGRAY_CACHE_SIZE=10000
MAGRAY_HNSW_THREADS=4
```

## 🚀 Примеры

### Batch операции

```rust
use memory::{BatchOperationBuilder, BatchConfig};

let batch = BatchOperationBuilder::new()
    .add_insert(record1)
    .add_insert(record2)
    .add_search("query", Layer::Interact, 5)
    .add_delete(id_to_delete, Layer::Interact)
    .build();

let results = memory_service.execute_batch(batch).await?;
```

### Cache management

```rust
// Статистика кэша
let (hits, misses, size) = memory_service.cache_stats();
println!("Cache: {} hits, {} misses, {} entries", hits, misses, size);

// Очистка кэша
memory_service.clear_cache().await?;
```

### Migration между версиями

```rust
use memory::migration::{MigrationManager, MigrationConfig};

let migration = MigrationManager::new(MigrationConfig {
    source_path: old_db_path,
    target_path: new_db_path,
    batch_size: 1000,
});

migration.migrate_all().await?;
```

## 📈 Бенчмарки

На Intel Core i7-10700K, 32GB RAM, NVMe SSD:

| Операция | Время | Throughput |
|----------|-------|------------|
| Insert (single) | 0.1ms | 10K/sec |
| Insert (batch 100) | 5ms | 20K/sec |
| Search (10K vectors) | 5ms | 200 qps |
| Search (100K vectors) | 15ms | 65 qps |
| Search (1M vectors) | 50ms | 20 qps |
| Promotion cycle | 10ms | - |

## 🐛 Известные проблемы

1. **Cache invalidation**: При прямом изменении записей через store кэш может устареть
2. **Memory usage**: HNSW индекс держит все векторы в памяти (~200MB на 100K записей)
3. **Reranker scores**: Иногда выдаёт неожиданные результаты на коротких текстах

## 🤝 Contributing

См. [CONTRIBUTING.md](../../CONTRIBUTING.md) для деталей.

## 📄 Лицензия

MIT