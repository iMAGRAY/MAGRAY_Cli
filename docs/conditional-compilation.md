# MAGRAY CLI - Conditional Compilation Guide

## Обзор

MAGRAY CLI поддерживает три варианта сборки для различных сценариев использования:

- **minimal** (~5MB) - базовая функциональность для контейнеров и CI
- **cpu** (~20MB) - полная функциональность без GPU
- **gpu** (~50MB) - полная функциональность с GPU ускорением

## Архитектура Features

### Workspace Features (Cargo.toml)

```toml
[workspace.metadata.features]
default = ["cpu"]
minimal = []
cpu = ["ai/cpu", "memory/cpu", "llm/cpu", "embeddings", "reranking"] 
gpu = ["cpu", "ai/gpu", "memory/gpu", "llm/gpu", "cuda", "tensorrt", "gpu-acceleration"]
```

### Crate-уровень Features

#### AI Crate
```toml
minimal = []                                    # Без AI моделей
cpu = ["embeddings", "reranking", "onnx"]      # Полная CPU функциональность
gpu = ["cpu", "cuda", "tensorrt", "directml"]  # GPU ускорение
```

#### Memory Crate
```toml
minimal = []                                    # Простое in-memory хранилище
cpu = ["hnsw-index", "persistence", "backup"]  # Полная память на CPU
gpu = ["cpu", "gpu-acceleration", "cuda"]       # GPU ускорение для индексов
```

#### LLM Crate
```toml
minimal = []                            # Mock провайдеры
cpu = ["anthropic", "openai", "groq"]   # Все API провайдеры
gpu = ["cpu"]                           # Пока что аналогично CPU
```

## Сборка

### Локальная сборка

#### Windows
```batch
# Минимальная версия
scripts\build_minimal.bat

# CPU версия  
scripts\build_cpu.bat

# GPU версия
scripts\build_gpu.bat

# Все варианты (PowerShell)
scripts\build_all.ps1 -Variant all
```

#### Linux/macOS
```bash
# Минимальная версия
./scripts/build_minimal.sh

# CPU версия
./scripts/build_cpu.sh

# GPU версия  
./scripts/build_gpu.sh
```

### Manual сборка

```bash
# Минимальная версия
cargo build --release --no-default-features --features="minimal" --target-dir="target/minimal"

# CPU версия
cargo build --release --no-default-features --features="cpu" --target-dir="target/cpu"

# GPU версия  
cargo build --release --no-default-features --features="gpu" --target-dir="target/gpu"
```

## Оптимизация Сборки

### RUSTFLAGS по вариантам

**Minimal:**
```bash
RUSTFLAGS="-C target-cpu=native -C link-arg=-s"
```

**CPU:**
```bash  
RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=fat -C codegen-units=1"
```

**GPU:**
```bash
RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=fat -C codegen-units=1"
```

## Системные Требования

### Minimal
- Rust toolchain
- Стандартные системные библиотеки

### CPU  
- Rust toolchain
- ONNX Runtime CPU (~100MB загрузка)
- Достаточно RAM для AI моделей

### GPU
- Rust toolchain  
- CUDA Toolkit 12.0+ (Windows/Linux)
- ONNX Runtime GPU (~500MB загрузка)
- CUDA-совместимый GPU с 4GB+ VRAM
- TensorRT (опционально)

## CI/CD Интеграция

### GitHub Actions

Workflow автоматически собирает все три варианта:

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, windows-latest, macos-latest]
    variant: [minimal, cpu, gpu]
    exclude:
      - os: macos-latest
        variant: gpu
```

### Артефакты сборки

После сборки доступны артефакты:
- `magray-minimal-{OS}` 
- `magray-cpu-{OS}`
- `magray-gpu-{OS}` (Linux/Windows)

## Тестирование

### По вариантам

**Minimal:**
```bash
magray --version  # Основная функциональность
```

**CPU:**
```bash
magray --version
magray models list    # AI функциональность
magray memory info    # Память и индексы
```

**GPU:**
```bash
magray --version  
magray gpu info       # GPU детекция
magray models list    # AI на GPU
```

## Docker Интеграция

### Dockerfile примеры

```dockerfile
# Minimal
FROM scratch
COPY target/minimal/release/magray /magray
ENTRYPOINT ["/magray"]

# CPU
FROM debian:bullseye-slim
COPY target/cpu/release/magray /usr/local/bin/
RUN apt-get update && apt-get install -y libgomp1

# GPU  
FROM nvidia/cuda:12.0-runtime-ubuntu22.04
COPY target/gpu/release/magray /usr/local/bin/
```

## Устранение неполадок

### Ошибки сборки

**"feature not found":**
- Проверить конфигурацию features в Cargo.toml
- Убедиться что все optional зависимости указаны

**CUDA ошибки:**
```bash
# Проверить CUDA
nvcc --version

# Установить переменные окружения  
export CUDA_PATH=/usr/local/cuda
export LD_LIBRARY_PATH=$CUDA_PATH/lib64:$LD_LIBRARY_PATH
```

**ONNX Runtime ошибки:**
```bash
# Windows
scripts\download_onnxruntime_gpu.ps1

# Linux
./scripts/install_onnxruntime.sh
```

### Размеры бинарников

**Слишком большие бинарники:**
- Использовать `strip` для удаления debug символов
- Включить LTO: `-C lto=fat`
- Опционально: UPX сжатие

**Отсутствующие функции:**
- Проверить активацию нужных features
- Убедиться что зависимости не optional для данного feature

## Расширение системы

### Добавление нового feature

1. **Добавить в workspace Cargo.toml:**
```toml
my-feature = ["crate/my-feature"]
```

2. **Добавить в crate Cargo.toml:**
```toml
my-feature = ["some-dependency"]
```

3. **Условная компиляция в коде:**
```rust
#[cfg(feature = "my-feature")]
pub fn my_function() {
    // Реализация
}

#[cfg(not(feature = "my-feature"))]
pub fn my_function() {
    // Mock/заглушка
}
```

### Новый вариант сборки

1. Добавить в workspace features
2. Обновить все crate features
3. Создать build скрипт
4. Обновить CI/CD
5. Добавить в документацию

## Лучшие практики

### Features design
- Используйте иерархические features (gpu включает cpu)
- Делайте heavy зависимости optional
- Предоставляйте mock реализации для minimal builds

### Сборка
- Всегда используйте --no-default-features для точного контроля
- Тестируйте каждый вариант отдельно
- Используйте target-dir для изоляции сборок

### Тестирование  
- Интеграционные тесты должны работать со всеми вариантами
- Unit тесты с conditional compilation
- CI/CD тесты для каждого варианта

## Производительность

### Размеры бинарников (примерные)
- **minimal**: ~5MB (только основная логика)
- **cpu**: ~20MB (включая ONNX Runtime CPU)  
- **gpu**: ~50MB (включая CUDA библиотеки)

### Время сборки
- **minimal**: ~2-5 минут
- **cpu**: ~5-10 минут
- **gpu**: ~10-15 минут (зависит от CUDA setup)