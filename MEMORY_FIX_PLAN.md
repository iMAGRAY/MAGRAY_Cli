# План исправления системы памяти MAGRAY CLI

## 1. Исправить проблему компиляции (КРИТИЧНО)

### Проблема
Конфликт между статической и динамической runtime библиотеками при линковке ORT.

### Решение
```toml
# В crates/memory/Cargo.toml изменить:
[dependencies]
ort = { version = "2.0.0-rc.10", features = ["download-binaries", "half", "load-dynamic"] }

# Или использовать стабильную версию:
ort = { version = "1.16", features = ["download-binaries"] }
```

## 2. Добавить гибкую конфигурацию путей к моделям

### Файл: `crates/memory/src/coordinator.rs`
```rust
// Вместо жестко заданных путей:
let models_base_path = std::env::var("MAGRAY_MODELS_PATH")
    .unwrap_or_else(|_| "./models".to_string());
    
let vectorizer = Arc::new(
    VectorizerService::new(
        PathBuf::from(&models_base_path).join("Qwen3-Embedding-0.6B-ONNX")
    ).await?
) as Arc<dyn Vectorizer>;
```

## 3. Исправить имена файлов моделей

### Файл: `crates/memory/src/onnx_models.rs`
```rust
// Строка 36: изменить
let model_file = model_path.join("model.onnx");
// На:
let model_file = if model_path.join("model_fp16.onnx").exists() {
    model_path.join("model_fp16.onnx")
} else {
    model_path.join("model.onnx")
};
```

## 4. Добавить fallback на mock модели

### Файл: `crates/memory/src/semantic_flexible.rs` (новый)
```rust
use crate::mock_models::{MockEmbeddingModel, MockRerankerModel};

pub async fn create_vectorizer_with_fallback<P: AsRef<Path>>(
    model_path: P
) -> Result<Arc<dyn Vectorizer>> {
    match VectorizerService::new(&model_path).await {
        Ok(service) => {
            info!("Loaded real ONNX embedding model");
            Ok(Arc::new(service))
        }
        Err(e) => {
            warn!("Failed to load ONNX model, falling back to mock: {}", e);
            let mock = MockVectorizerService::new().await?;
            Ok(Arc::new(mock))
        }
    }
}
```

## 5. Создать интеграционные тесты

### Файл: `crates/memory/tests/integration_test_fixed.rs`
```rust
#[tokio::test]
async fn test_memory_system_with_mock() {
    std::env::set_var("MAGRAY_USE_MOCK_MODELS", "true");
    
    let config = MemoryConfig::default();
    let coordinator = MemoryCoordinator::new(config).await.unwrap();
    
    // Тестируем базовые операции...
}
```

## 6. Добавить документацию по настройке

### Файл: `models/SETUP.md`
```markdown
# Настройка ONNX моделей

1. Скачайте модели:
   - [Qwen3 Embedding](https://huggingface.co/...)
   - [Qwen3 Reranker](https://huggingface.co/...)

2. Распакуйте в директории:
   - `models/Qwen3-Embedding-0.6B-ONNX/`
   - `models/Qwen3-Reranker-0.6B-ONNX/`

3. Для разработки можно использовать mock модели:
   ```bash
   export MAGRAY_USE_MOCK_MODELS=true
   cargo test -p memory
   ```
```

## 7. Обновить примеры

Обновить `examples/code_search_demo.rs` для работы с mock моделями при их отсутствии.

## Приоритеты исправлений

1. **Высокий**: Исправить проблему компиляции (пункт 1)
2. **Высокий**: Добавить fallback механизм (пункт 4)
3. **Средний**: Исправить пути и имена файлов (пункты 2-3)
4. **Низкий**: Добавить тесты и документацию (пункты 5-7)

## Команды для проверки

```bash
# После исправлений:
cd crates/memory
cargo build
cargo test --lib
cargo run --example simple_memory_test
```