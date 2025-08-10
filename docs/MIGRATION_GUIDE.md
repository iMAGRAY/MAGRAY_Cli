# Migration Guide: От старой системы к новой

## 🔄 Обзор изменений

### Было (v0.1)
- LanceDB для векторного поиска
- O(n) линейный поиск
- Два promotion engine (legacy + optimized)
- Mock AI модели
- Сложный API с множеством методов

### Стало (v1.0)
- HNSW через `hnsw_rs` 
- O(log n) поиск
- Единый PromotionEngine
- Реальные ONNX модели (Qwen3)
- Unified API

## 📦 Изменения в зависимостях

### Удалены
```toml
# Больше не нужны
lancedb = "0.x"
arrow = "x.x"
```

### Добавлены
```toml
hnsw_rs = "0.3"
ort = "2.0.0-rc.10"  # ONNX Runtime 2.0
```

## 🔧 API изменения

### 1. Инициализация

**Было:**
```rust
let store = LanceDBStore::new(config)?;
let promotion = PromotionEngine::new(store.clone(), config);
let optimized_promotion = OptimizedPromotionEngine::new(store.clone(), config, db)?;
```

**Стало:**
```rust
let service = MemoryService::new(config).await?;
let api = UnifiedMemoryAPI::new(Arc::new(service));
```

### 2. Сохранение записей

**Было:**
```rust
let record = Record {
    id: Uuid::new_v4(),
    content: "text".to_string(),  // Поле content
    embedding: None,              // Optional
    layer: Layer::ShortTerm,      // Старое название
    // ... много полей
};
store.insert(&record)?;
```

**Стало:**
```rust
// Простой способ
let id = api.remember(
    "text".to_string(),
    MemoryContext::new("type").with_tags(vec!["tag"])
).await?;

// Или через service
let record = Record {
    text: "text".to_string(),    // Поле text, не content
    embedding: vec![],            // Заполнится автоматически
    layer: Layer::Interact,       // Новые названия слоёв
    // ... остальное
};
service.insert(record).await?;
```

### 3. Поиск

**Было:**
```rust
let results = store.search(query, layer, k)?;
// Или
let results = service.search(query, Some(layer), Some(k), Some(threshold)).await?;
```

**Стало:**
```rust
// Через API
let results = api.recall(query, SearchOptions::new().limit(k)).await?;

// Через service (builder pattern)
let results = service
    .search(query)
    .with_layer(layer)
    .top_k(k)
    .min_score(threshold)
    .execute()
    .await?;
```

### 4. Promotion

**Было:**
```rust
// Два разных метода
let stats = service.run_promotion_cycle().await?;         // Legacy O(n)
let stats = service.run_optimized_promotion_cycle().await?; // Optimized O(log n)
```

**Стало:**
```rust
// Один метод, всегда оптимизированный
let stats = service.run_promotion_cycle().await?;
// Или через API
let stats = api.optimize_memory().await?;
```

### 5. Названия слоёв

| Старое | Новое |
|--------|-------|
| ShortTerm | Interact |
| MediumTerm | Insights |
| LongTerm | Assets |
| Ephemeral | (удалён) |
| Semantic | (объединён с Assets) |

## 🔄 Пошаговая миграция

### Шаг 1: Обновите Cargo.toml

```toml
[dependencies]
memory = { path = "../memory", version = "1.0" }
ai = { path = "../ai", version = "1.0" }

# Удалите старые зависимости
# lancedb = ...
# arrow = ...
```

### Шаг 2: Обновите импорты

```rust
// Было
use memory::{
    LanceDBStore, 
    PromotionEngine, 
    OptimizedPromotionEngine,
    Layer::ShortTerm,
};

// Стало
use memory::{
    MemoryService,
    UnifiedMemoryAPI, 
    MemoryContext,
    Layer::Interact,
};
```

### Шаг 3: Мигрируйте данные

```rust
use memory::migration::{migrate_from_lancedb, MigrationConfig};

// Автоматическая миграция
let config = MigrationConfig {
    source_path: "path/to/old/lancedb",
    target_path: "path/to/new/hnswdb",
    batch_size: 1000,
};

migrate_from_lancedb(config).await?;
```

### Шаг 4: Обновите код

#### Поиск и замена

```bash
# В вашем редакторе
Find: "content:"
Replace: "text:"

Find: "Layer::ShortTerm"
Replace: "Layer::Interact"

Find: "Layer::MediumTerm"  
Replace: "Layer::Insights"

Find: "Layer::LongTerm"
Replace: "Layer::Assets"

Find: "run_optimized_promotion_cycle"
Replace: "run_promotion_cycle"
```

#### Обновите структуры

```rust
// Если у вас есть кастомные Record структуры
#[derive(Clone)]
struct MyRecord {
    text: String,      // было content
    // ...
}

impl From<MyRecord> for memory::Record {
    fn from(my: MyRecord) -> Self {
        Self {
            text: my.text,  // было content
            // ...
        }
    }
}
```

## ⚠️ Breaking Changes

### 1. Embedding размерность

- **Было**: 384 (all-MiniLM-L6-v2)
- **Стало**: 1024 (Qwen3)

Необходимо перегенерировать все embeddings!

### 2. Async everywhere

Все операции теперь async:

```rust
// Было
let result = store.insert(&record)?;

// Стало  
let result = service.insert(record).await?;
```

### 3. Config структура

```rust
// Полностью изменилась
let config = MemoryConfig {
    db_path: PathBuf::from("./hnswdb"),     // Не lancedb
    cache_path: PathBuf::from("./cache"),
    promotion: PromotionConfig { /* ... */ },
    ai_config: AiConfig { /* ... */ },       // Новое
    health_config: HealthConfig { /* ... */ }, // Новое
};
```

## 🛠️ Утилиты миграции

### Скрипт проверки совместимости

```rust
use memory::migration::check_compatibility;

match check_compatibility(&old_db_path) {
    Ok(report) => {
        println!("Records: {}", report.total_records);
        println!("Layers: {:?}", report.layers);
        println!("Can migrate: {}", report.can_migrate);
    }
    Err(e) => println!("Incompatible: {}", e),
}
```

### Batch миграция с прогрессом

```rust
use memory::migration::{MigrationManager, MigrationProgress};

let manager = MigrationManager::new(config);
let (tx, rx) = mpsc::channel();

// В отдельном потоке
tokio::spawn(async move {
    while let Ok(progress) = rx.recv() {
        match progress {
            MigrationProgress::Started => println!("Migration started"),
            MigrationProgress::Progress(p) => println!("{}%", p),
            MigrationProgress::Completed => println!("Done!"),
            MigrationProgress::Error(e) => eprintln!("Error: {}", e),
        }
    }
});

manager.migrate_with_progress(tx).await?;
```

## 📋 Чеклист миграции

- [ ] Backup старых данных
- [ ] Обновить зависимости в Cargo.toml
- [ ] Заменить импорты
- [ ] Обновить названия слоёв
- [ ] Изменить `content` на `text`
- [ ] Добавить `.await` ко всем вызовам
- [ ] Мигрировать данные через утилиту
- [ ] Перегенерировать embeddings (Qwen3)
- [ ] Протестировать поиск
- [ ] Проверить promotion
- [ ] Настроить health monitoring

## 🆘 Частые проблемы

### "cannot find type `LanceDBStore`"
Используйте `MemoryService` вместо `LanceDBStore`.

### "no method named `run_optimized_promotion_cycle`"
Используйте `run_promotion_cycle()` - он уже оптимизированный.

### "expected `content`, found `text`"
Поле переименовано с `content` на `text`.

### "the trait `Future` is not implemented"
Добавьте `.await` - все операции теперь async.

### "embedding dimension mismatch"
Нужно перегенерировать embeddings с Qwen3 (1024 размерность).

## 📞 Поддержка

Если возникли проблемы с миграцией:

1. Проверьте [примеры](../crates/memory/examples/)
2. Посмотрите [тесты](../crates/memory/tests/)
3. Создайте issue в репозитории

Удачной миграции! 🚀