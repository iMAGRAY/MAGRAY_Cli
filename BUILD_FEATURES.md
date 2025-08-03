# MAGRAY Build Features

Этот документ описывает различные варианты сборки MAGRAY CLI с разными feature флагами.

## 🚀 Доступные Features

### CPU Mode (по умолчанию)
```bash
cargo build --release
# или явно
cargo build --release --features=cpu
```
- ✅ Полная функциональность на CPU
- ✅ Без GPU зависимостей
- ✅ Минимальный размер бинарника
- ✅ Совместимо с любой системой

### GPU Mode
```bash
cargo build --release --features=gpu
```
- ✅ GPU ускорение для embeddings
- ✅ Автоматический fallback на CPU
- ⚠️ Требует ONNX Runtime с CUDA
- ⚠️ Больший размер бинарника

### Minimal Mode
```bash
cargo build --release --features=minimal
```
- ✅ Минимальная функциональность
- ✅ Максимально компактный binary
- ⚠️ Без GPU поддержки
- ⚠️ Ограниченные AI возможности

## 🔧 Примеры Сборки

### Development (быстрая сборка)
```bash
cargo build
```

### Production CPU-only
```bash
cargo build --release --features=cpu
strip target/release/magray  # для Linux/macOS
```

### Production с GPU
```bash
cargo build --release --features=gpu
# Убедитесь что ONNX Runtime установлен:
# Windows: установите onnxruntime-gpu
# Linux: apt install onnxruntime-gpu
```

### Docker/CI Minimal
```bash
cargo build --release --features=minimal --target x86_64-unknown-linux-musl
```

## 📊 Сравнение Размеров

| Feature | Binary Size | Dependencies | GPU Support | Статус |
|---------|-------------|--------------|-------------|--------|
| cpu     | ~16 MB      | ONNX CPU     | ❌          | ✅ Стабильно |
| gpu     | ~45 MB      | ONNX+CUDA    | ✅ Fallback | ⚠️ Требует CUDA |
| minimal | ~16 MB      | ONNX CPU     | ❌          | ✅ Стабильно |

**Реальные размеры:** CPU и minimal режимы дают одинаковый размер (~16MB) благодаря conditional compilation

## ⚡ Производительность

### CPU Mode
- Embeddings: ~100ms/текст
- Memory ops: ~10ms
- Startup: ~150ms

### GPU Mode  
- Embeddings: ~20ms/текст (при наличии GPU)
- Fallback: автоматически к CPU
- Startup: ~300ms (инициализация GPU)

## 🎯 Рекомендации

### Для разработки
```bash
cargo run  # CPU mode
```

### Для production серверов
```bash
cargo build --release --features=cpu
```

### Для рабочих станций с GPU
```bash
cargo build --release --features=gpu
```

### Для container/edge deployment
```bash
cargo build --release --features=minimal
```

## 🔍 Диагностика

Проверить какие features активны:
```bash
magray status
```

Выведет информацию о:
- GPU доступности
- Embedded models статус
- Memory system состояние

## ⚠️ Troubleshooting

### GPU не работает
1. Проверьте ONNX Runtime установку
2. Убедитесь что скомпилировано с `--features=gpu`
3. Проверьте CUDA/ROCm драйверы

### Слишком большой binary
1. Используйте `--features=cpu` или `--features=minimal`
2. Примените `strip` после сборки
3. Рассмотрите UPX compression

### Медленная работа
1. Для GPU: используйте `--features=gpu`
2. Для CPU: проверьте количество cores
3. Оптимизируйте через `RUST_LOG=debug`