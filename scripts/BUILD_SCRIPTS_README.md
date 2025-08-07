# MAGRAY CLI - Build Scripts

## Обзор скриптов сборки

Система conditional compilation с тремя вариантами сборки:

### 🏗️ Скрипты сборки

| Скрипт | Платформа | Описание |
|--------|-----------|----------|
| `build_minimal.sh/bat` | Linux/Windows | Минимальная сборка (~5MB) |
| `build_cpu.sh/bat` | Linux/Windows | CPU сборка (~20MB) |  
| `build_gpu.sh/bat` | Linux/Windows | GPU сборка (~50MB) |
| `build_all.ps1` | PowerShell | Универсальная сборка всех вариантов |

### ⚡ Быстрый старт

#### Windows
```batch
# Один вариант
scripts\build_cpu.bat

# Все варианты  
powershell -File scripts\build_all.ps1
```

#### Linux/macOS
```bash
# Сделать исполняемыми (один раз)
chmod +x scripts/build_*.sh

# Один вариант
./scripts/build_cpu.sh

# Все варианты (требует PowerShell Core)
pwsh scripts/build_all.ps1
```

### 📦 Варианты сборки

#### Minimal (~5MB)
```bash
# Что включено:
- Базовая CLI функциональность
- Простые команды
- Mock провайдеры для LLM

# Что исключено:
- AI модели и embeddings
- HNSW индексы
- Persistence
- GPU поддержка
```

#### CPU (~20MB)  
```bash
# Что включено:
- Полная AI функциональность на CPU
- ONNX Runtime CPU
- BGE-M3 embeddings
- HNSW индексы  
- Все LLM провайдеры
- Persistence и backup

# Что исключено:
- GPU ускорение
- CUDA/TensorRT
```

#### GPU (~50MB)
```bash
# Что включено:
- Всё из CPU варианта
- GPU ускорение для AI
- CUDA поддержка
- TensorRT оптимизации
- GPU memory pooling

# Требования:
- CUDA Toolkit 12.0+
- CUDA-совместимый GPU
- ONNX Runtime GPU
```

### 🛠️ Настройка окружения

#### GPU Build Requirements

**Windows:**
```powershell
# 1. Установить CUDA Toolkit
# https://developer.nvidia.com/cuda-downloads

# 2. Скачать ONNX Runtime GPU
scripts\download_onnxruntime_gpu.ps1

# 3. Проверить установку
nvcc --version
```

**Linux:**
```bash  
# 1. Установить CUDA Toolkit
sudo apt install nvidia-cuda-toolkit

# 2. Установить ONNX Runtime
./scripts/install_onnxruntime.sh

# 3. Настроить окружение
export CUDA_PATH=/usr/local/cuda
export LD_LIBRARY_PATH=$CUDA_PATH/lib64:$LD_LIBRARY_PATH
```

### 🔧 Параметры сборки

#### build_all.ps1 параметры:
```powershell
# Сборка всех вариантов
scripts\build_all.ps1 -Variant all

# Только один вариант
scripts\build_all.ps1 -Variant cpu

# С очисткой
scripts\build_all.ps1 -Variant all -Clean

# С тестированием  
scripts\build_all.ps1 -Variant cpu -Test
```

### 📁 Структура выходных файлов

```
target/
├── minimal/
│   └── release/
│       └── magray(.exe)     # ~5MB
├── cpu/
│   └── release/  
│       └── magray(.exe)     # ~20MB
└── gpu/
    └── release/
        └── magray(.exe)     # ~50MB
```

### ✅ Тестирование сборок

Каждый скрипт автоматически тестирует результат:

```bash
# Базовые тесты
magray --version

# CPU-специфичные
magray models list

# GPU-специфичные  
magray gpu info
```

### 🐛 Устранение неполадок

#### Ошибки сборки

**"cargo not found":**
```bash
# Установить Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**CUDA ошибки:**
```bash
# Проверить переменные окружения
echo $CUDA_PATH
echo $LD_LIBRARY_PATH

# Windows
echo %CUDA_PATH%
```

**Недостающие библиотеки:**
```bash
# Linux - установить зависимости
sudo apt install build-essential pkg-config

# Windows - использовать Visual Studio Build Tools
```

#### Проблемы с размером

**Слишком большой бинарник:**
```bash
# Включается автоматически в скриптах:
# -C lto=fat           # Link-time optimization
# -C codegen-units=1   # Единый блок кода
# strip               # Удаление debug символов
```

### 🚀 CI/CD Integration

GitHub Actions автоматически использует эти скрипты:

```yaml
# .github/workflows/multi-variant-build.yml
- name: Build CPU variant
  run: ./scripts/build_cpu.sh
```

### 📊 Производительность сборки

| Вариант | Время сборки | Размер | RAM usage |
|---------|--------------|---------|-----------|
| minimal | 2-5 мин | ~5MB | ~500MB |
| cpu | 5-10 мин | ~20MB | ~2GB |
| gpu | 10-15 мин | ~50MB | ~4GB |

### 🔗 Связанные файлы

- `docs/conditional-compilation.md` - Подробная документация
- `.github/workflows/multi-variant-build.yml` - CI/CD
- `Cargo.toml` - Workspace features configuration
- `crates/*/Cargo.toml` - Crate-level features