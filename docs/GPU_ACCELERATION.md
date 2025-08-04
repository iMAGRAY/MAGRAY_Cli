# GPU Acceleration Guide for MAGRAY CLI

Это руководство объясняет, как включить и использовать GPU ускорение в MAGRAY CLI для более быстрой работы с эмбеддингами и ранжированием.

## Требования

- NVIDIA GPU с поддержкой CUDA
- CUDA Toolkit 11.8 или выше
- cuDNN 8.9 или выше
- TensorRT (опционально, для максимальной производительности)
- ONNX Runtime 2.0.0-rc.4+ с CUDA провайдерами

## Сборка с поддержкой GPU

Для включения GPU поддержки, соберите MAGRAY с флагом `gpu`:

```bash
cargo build --release --features gpu
```

## Архитектура GPU системы

### Компоненты

1. **GpuDetector** - Определение доступных GPU через nvidia-smi
2. **GpuConfig** - Конфигурация параметров GPU
3. **AutoDeviceSelector** - Автоматический выбор CPU/GPU на основе бенчмарков
4. **GpuMemoryPool** - Эффективное управление GPU памятью
5. **OptimizedEmbeddingServiceV2** - Оптимизированный сервис с GPU поддержкой

### Автоматическое определение GPU

MAGRAY автоматически определяет наличие GPU и его характеристики:

```rust
let detector = GpuDetector::detect();
if detector.available {
    println!("GPU: {}", detector.devices[0].name);
    println!("Memory: {} GB", detector.devices[0].total_memory_mb / 1024);
    println!("CUDA: {}", detector.cuda_version);
}
```

## Конфигурация

### Автоматический режим (рекомендуется)

```rust
// Автоматически выберет лучшее устройство
let (service, decision) = SmartEmbeddingFactory::create_optimized(config).await?;
println!("Используется: {}", if decision.use_gpu { "GPU" } else { "CPU" });
```

### Ручная конфигурация

```rust
let config = EmbeddingConfig {
    model_name: "bge-m3".to_string(),
    use_gpu: true,
    gpu_config: Some(GpuConfig {
        device_id: 0,
        gpu_mem_limit: 4 * 1024 * 1024 * 1024, // 4GB
        use_tensorrt: true,
        enable_fp16: true,
        ..Default::default()
    }),
    batch_size: 128, // Больший batch для GPU
    ..Default::default()
};
```

### Переменные окружения

```bash
# Включить GPU ускорение
export MAGRAY_USE_GPU=true

# Выбрать конкретное GPU устройство (по умолчанию: 0)
export CUDA_VISIBLE_DEVICES=0

# Лимит GPU памяти в MB (по умолчанию: авто)
export MAGRAY_GPU_MEMORY_LIMIT=4096

# Включить TensorRT оптимизацию
export MAGRAY_USE_TENSORRT=true
```

## Оптимизация производительности

### Динамический размер батча

MAGRAY автоматически подбирает оптимальный размер батча на основе:
- Доступной GPU памяти
- Compute capability устройства
- Размера модели

Рекомендуемые размеры:
- **RTX 4090 (24GB)**: batch_size = 256-512
- **RTX 3090 (24GB)**: batch_size = 256
- **RTX 3080 (10GB)**: batch_size = 128
- **RTX 3070 (8GB)**: batch_size = 64
- **Старые GPU (<8GB)**: batch_size = 32

### Memory Pooling

MAGRAY использует пул памяти для минимизации аллокаций:

```rust
// Автоматически управляется
GPU_MEMORY_POOL.with_buffer(size, |buffer| {
    // Используйте buffer для операций
    Ok(result)
})?;
```

### Метрики производительности

Встроенные метрики позволяют отслеживать производительность:

```rust
let metrics = service.get_metrics();
println!("Tokens/sec: {:.1}", metrics.tokens_per_second());
println!("Cache hit rate: {:.1}%", metrics.cache_hit_rate() * 100.0);
```

## Мониторинг

### Проверка использования GPU

```bash
# NVIDIA System Management Interface
nvidia-smi

# Мониторинг в реальном времени
watch -n 1 nvidia-smi

# Детальная информация о GPU
nvidia-smi -q
```

### Встроенная диагностика

```bash
# Запустить тест GPU
cargo run --example test_gpu_acceleration --features gpu
```

## Решение проблем

### Частые проблемы

1. **"CUDA not found" ошибка**
   ```bash
   # Проверьте установку CUDA
   nvcc --version
   
   # Добавьте CUDA в PATH
   export PATH=/usr/local/cuda/bin:$PATH
   export LD_LIBRARY_PATH=/usr/local/cuda/lib64:$LD_LIBRARY_PATH
   ```

2. **"Out of memory" ошибка**
   - Уменьшите batch_size
   - Используйте FP16 режим: `enable_fp16 = true`
   - Проверьте другие процессы: `nvidia-smi`

3. **"GPU not detected"**
   - Обновите драйверы NVIDIA
   - Проверьте поддержку CUDA: `nvidia-smi -L`
   - Убедитесь в правильной сборке с `--features gpu`

4. **Низкая производительность GPU**
   - Проверьте thermal throttling: `nvidia-smi -q -d TEMPERATURE`
   - Увеличьте batch_size для лучшей утилизации
   - Включите TensorRT: `use_tensorrt = true`

### Отладка

```bash
# Включить подробное логирование
export RUST_LOG=ai=debug,magray=debug

# Логирование CUDA операций
export CUDA_LAUNCH_BLOCKING=1
```

## Производительность

### Типичные ускорения

| Операция | CPU время | GPU время | Ускорение |
|----------|-----------|-----------|-----------|
| Embed 100 текстов | 2.5с | 0.3с | 8.3x |
| Embed 1000 текстов | 25с | 2.5с | 10x |
| Rerank 50 документов | 1.2с | 0.15с | 8.0x |
| Batch 5000 embeddings | 125с | 10с | 12.5x |

*Результаты зависят от модели GPU и размера батча

### Сравнение GPU моделей

| GPU | VRAM | Рек. batch_size | Tokens/sec |
|-----|------|-----------------|------------|
| RTX 4090 | 24GB | 512 | ~50,000 |
| RTX 3090 | 24GB | 256 | ~35,000 |
| RTX 3080 | 10GB | 128 | ~25,000 |
| RTX 3070 | 8GB | 64 | ~18,000 |
| GTX 1080 Ti | 11GB | 64 | ~12,000 |

## Расширенные настройки

### TensorRT оптимизация

```toml
[ai.gpu]
use_tensorrt = true
tensorrt_cache_dir = ".tensorrt_cache"
tensorrt_cache_size = 1073741824  # 1GB
enable_fp16 = true
tensorrt_workspace_size = 2147483648  # 2GB
```

### Multi-GPU поддержка (в разработке)

```toml
[ai.gpu]
multi_gpu = true
gpu_devices = [0, 1]  # Использовать GPU 0 и 1
distribution_strategy = "round_robin"  # или "model_parallel"
```

## Примеры использования

### Простой пример

```rust
use ai::{EmbeddingConfig, GpuConfig};

let config = EmbeddingConfig {
    use_gpu: true,
    gpu_config: Some(GpuConfig::auto_optimized()),
    ..Default::default()
};

let service = OptimizedEmbeddingServiceV2::new(config)?;
let embeddings = service.embed_batch(texts).await?;
```

### Продвинутый пример с метриками

```rust
// Автоматический выбор устройства
let (service, decision) = SmartEmbeddingFactory::create_optimized(base_config).await?;

println!("Выбрано: {} ({})", 
    if decision.use_gpu { "GPU" } else { "CPU" },
    decision.reason
);

// Обработка с метриками
let embeddings = service.embed_batch(texts).await?;

// Статистика
if let Some(v2_service) = service.downcast_ref::<OptimizedEmbeddingServiceV2>() {
    v2_service.print_metrics();
}
```

## Тестирование

```bash
# Полный тест GPU системы
cargo test --features gpu -- --nocapture

# Бенчмарк производительности
cargo bench --features gpu

# Интеграционный тест
cargo run --example test_gpu_acceleration --features gpu
```

## Roadmap

- [x] Базовая GPU поддержка через ONNX Runtime
- [x] Автоматическое определение GPU
- [x] Memory pooling для GPU
- [x] Динамическая оптимизация batch size
- [x] TensorRT интеграция
- [ ] Multi-GPU поддержка
- [ ] INT8 квантизация для GPU
- [ ] AMD ROCm поддержка
- [ ] Apple Metal поддержка

## История изменений

### v2.0 - Полная GPU система
- Реализован GpuDetector для реального определения GPU через nvidia-smi
- Добавлен AutoDeviceSelector для автоматического выбора CPU/GPU
- Создан GpuMemoryPool для эффективного управления памятью
- Реализована динамическая оптимизация batch size
- Добавлены метрики производительности и профилирование

### v1.0 - Базовая поддержка
- Интеграция ONNX Runtime с CUDA провайдерами
- Простая GPU конфигурация
- Условная компиляция с feature flags