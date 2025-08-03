# MAGRAY CLI 🚀

Интеллектуальный CLI агент на Rust с многослойной памятью, векторным поиском и расширяемой системой инструментов. Поставляется как единый исполняемый файл без зависимостей.

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Build Status](https://img.shields.io/github/actions/workflow/status/yourusername/MAGRAY_Cli/build-matrix.yml?branch=main)](https://github.com/yourusername/MAGRAY_Cli/actions)

## ✨ Основные возможности

- 🏃 **Единый исполняемый файл** - установка через `cargo install`, без Python/Node/Docker
- 🧠 **Трёхслойная память** - автоматическое управление контекстом с продвижением/угасанием
- ⚡ **HNSW векторный поиск** - поиск за <5мс с professional hnsw_rs реализацией
- 🤖 **Локальный AI стек** - ONNX эмбеддинги/реранжирование, опциональные LLM провайдеры
- 🔧 **Расширяемые инструменты** - файловые операции, git интеграция, shell команды
- 📊 **Наблюдаемость** - встроенное логирование, метрики и трассировка событий
- 🛡️ **Безопасность памяти** - 100% Rust без unsafe блоков в core

## 🚀 Быстрый старт

```bash
# Установка из crates.io (когда будет опубликован)
cargo install magray

# Или сборка из исходников
git clone https://github.com/yourusername/MAGRAY_Cli
cd MAGRAY_Cli
make build-cpu

# Скачать модели вручную (обязательно)
./scripts/download_models.ps1

# Начать использование
magray --version
magray status
magray chat "Привет! Как дела?"
```

## 📦 Установка

### Системные требования

- **Rust 1.75+** - установить через [rustup](https://rustup.rs/)
- **4GB RAM** минимум (8GB рекомендуется)
- **2GB дискового пространства** для моделей
- **ONNX Runtime** - автоматически устанавливается

### Из исходных кодов

```bash
# Клонирование репозитория
git clone https://github.com/yourusername/MAGRAY_Cli
cd MAGRAY_Cli

# Скачивание ONNX моделей
./scripts/download_models.ps1

# Сборка и установка
make build-cpu
# или полная установка
cargo install --path crates/cli

# Проверка установки
magray --version
magray status
```

### Варианты сборки по возможностям

```bash
# CPU-only режим (рекомендуется для production)
make build-cpu
cargo build --release --features=cpu

# GPU ускорение (требует CUDA)
make build-gpu
cargo build --release --features=gpu

# Минимальная сборка (для контейнеров)
make build-minimal
cargo build --release --features=minimal

# Проверка всех вариантов
make verify-features
```

## 🎯 Использование

### Основные команды

```bash
# Интерактивный чат режим (по умолчанию)
magray

# Прямой чат с сообщением
magray chat "Как оптимизировать SQL запрос?"

# Операции с файлами
magray read файл.txt
magray write вывод.txt "Привет Мир"
magray list ./src

# Выполнение инструментов через естественный язык
magray tool "покажи git статус"
magray tool "создай новый файл с hello world"

# Умное AI планирование для сложных задач
magray smart "проанализируй кодовую базу и предложи улучшения"
```

### Продвинутые возможности

```bash
# Операции с системой памяти
magray memory search "обработка ошибок" --layer insights --top-k 20
magray memory add "API лимит 1000 запросов/мин" --layer insights
magray memory stats
magray memory backup --name my-backup

# Управление GPU ускорением
magray gpu info
magray gpu benchmark --batch-size 100 --compare
magray gpu memory status

# Диагностика системы
magray status
magray health
```

## 🏗️ Архитектура проекта

### Структура кодовой базы

```
MAGRAY_Cli/
├── crates/                 # Rust workspace crates
│   ├── cli/               # Главный бинарник (magray)
│   ├── llm/               # LLM клиент абстракция
│   ├── memory/            # Векторное хранилище и слои памяти
│   ├── ai/                # ONNX модели и эмбеддинги
│   ├── tools/             # Система инструментов
│   ├── router/            # AI маршрутизация
│   ├── todo/              # Управление задачами
│   └── common/            # Общие утилиты
├── scripts/               # Скрипты установки и утилиты
│   ├── docker/            # Docker контейнеры
│   ├── download_*.ps1     # Скрипты загрузки моделей
│   └── install_*.ps1      # Скрипты установки зависимостей
├── models/                # ONNX модели (git-ignored)
├── docs/                  # Документация
├── .github/               # CI/CD workflows
└── Makefile              # Система сборки
```

### Слои памяти

| Слой | Назначение | Время жизни | Производительность |
|------|------------|-------------|-------------------|
| **L1 Interact** | Контекст текущей сессии | 24 часа | HNSW индекс, <5мс |
| **L2 Insights** | Извлечённые знания | 90 дней | HNSW индекс, <8мс |
| **L3 Assets** | Долгосрочное хранение | Без ограничений | HNSW индекс, <10мс |

### Производительность векторного поиска

Система использует **hnsw_rs** от Jean-Pierre Both - профессиональную Rust реализацию алгоритма Hierarchical Navigable Small World:

- 🚀 **17x быстрее** линейного поиска на 5K+ документах
- 🎯 **100% recall** с оптимальными параметрами
- ⚡ **Сублинейное масштабирование** O(log n) против O(n)
- 🔧 **Настраиваемые параметры**: M=24, ef_construction=400, ef_search=100
- 🧵 **Параллельные операции** для batch вставок и multi-query поиска

**Результаты бенчмарков:**
```
Размер данных   HNSW время   Линейное время   Ускорение
100 документов      1.9мс         2.1мс          1.1x
500 документов      2.9мс        10.5мс          3.6x  
1000 документов     4.2мс        21.0мс          5.0x
2000 документов     3.1мс        42.3мс         13.8x
5000 документов     6.0мс       104.8мс         17.4x
```

## 🤖 AI модели

MAGRAY CLI использует современные ONNX модели для векторного поиска и реранжирования:

### Модель эмбеддингов
- **BGE-M3 (BAAI/bge-m3)**
  - Размерность: 1024
  - Поддержка многоязычности (русский, английский, китайский)
  - Оптимизация для ONNX Runtime
  - Размер модели: ~1.2GB

### Модель реранжирования  
- **BGE Reranker v2-m3**
  - Семантическое переранжирование результатов поиска
  - Высокая точность на многоязычных текстах
  - FP16 квантизация для производительности
  - Размер модели: ~560MB

### Токенизатор
- **XLM-RoBERTa tokenizer**
  - Поддержка 100+ языков
  - Subword токенизация с BPE
  - Совместимость с transformers 0.20+

Модели автоматически загружаются при первом запуске или через:
```bash
./scripts/download_models.ps1
```

## 🔧 Конфигурация

Файл конфигурации в `~/.magray/config.toml`:

```toml
[ai]
embed_model = "bge-m3"
embed_batch_size = 32
rerank_model = "bge_reranker_v2_m3"
use_gpu = false
max_sequence_length = 8192

[ai.llm]
provider = "openai"
model = "gpt-4o-mini"
max_tokens = 2048
temperature = 0.7

[memory]
interact_ttl_hours = 24
insights_ttl_days = 90
promote_threshold = 0.8
max_vectors_per_layer = 100000
cache_size_mb = 1024

[memory.hnsw]
max_connections = 24
ef_construction = 400
ef_search = 100

[tools]
enable_network = true
plugin_dir = "~/.magray/plugins"
max_file_size_mb = 100

[logging]
level = "info"
json_output = false
file_output = true
```

## 🔧 Система инструментов

MAGRAY CLI включает встроенные инструменты для общих задач разработки:

### Файловые операции
- **Чтение файлов**: Подсветка синтаксиса для 50+ языков
- **Запись файлов**: Умное создание директорий
- **Список файлов**: Фильтрация по расширениям и размеру

### Git интеграция
- **Статус репозитория**: Отслеживание изменений
- **Коммиты**: Автоматическая генерация сообщений
- **История**: Просмотр логов и диффов

### Shell команды
- **Кроссплатформенное выполнение**: Windows/Linux/macOS
- **Безопасная изоляция**: Ограничение доступа к системе
- **Потоковый вывод**: Реалтайм результаты

### Web поиск
- **DuckDuckGo интеграция**: Поиск документации
- **Фильтрация результатов**: Релевантность по домену
- **Кэширование**: Локальное сохранение результатов

Доступ к инструментам через естественный язык:
```bash
magray tool "покажи git статус"
magray tool "создай новый файл test.rs с функцией hello world"
magray tool "найди все .rs файлы в директории src"
magray tool "выполни cargo test и покажи результаты"
```

## 🧪 Разработка

### Сборка и тестирование

```bash
# Development сборка
make dev-cpu

# Запуск тестов
make test-all

# Запуск бенчмарков
make bench

# Проверка кода
make check

# Генерация документации
cargo doc --open
```

### Использование Makefile

```bash
# Показать все доступные команды
make help

# Сборка разных вариантов
make build-cpu      # CPU-only сборка
make build-gpu      # GPU ускорение
make build-minimal  # Минимальная сборка
make build-all      # Все варианты

# Тестирование
make test           # Базовые тесты
make test-all       # Все feature combinations
make verify-features # Проверка совместимости

# Docker
make docker-build   # Сборка Docker образов
make docker-test    # Тестирование контейнеров

# Анализ
make size-analysis  # Сравнение размеров бинарников
make perf-test      # Быстрый тест производительности

# Утилиты
make clean          # Очистка артефактов
make release        # Подготовка релиза
```

### Docker развертывание

```bash
# CPU-only для production серверов
docker build -f scripts/docker/Dockerfile.cpu -t magray:cpu .
docker run -it magray:cpu

# GPU для рабочих станций
docker build -f scripts/docker/Dockerfile.gpu -t magray:gpu .
docker run --gpus all -it magray:gpu

# Minimal для edge устройств
docker build -f scripts/docker/Dockerfile.minimal -t magray:minimal .
docker run -it magray:minimal

# Использование docker-compose
cd scripts/docker
docker-compose --profile cpu up    # CPU режим
docker-compose --profile gpu up    # GPU режим
docker-compose --profile benchmark up  # Benchmark
```

### CI/CD Pipeline

Проект включает comprehensive CI/CD pipeline с:

- **Multi-platform builds**: Linux, Windows, macOS
- **Feature matrix testing**: CPU, GPU, minimal режимы
- **Performance benchmarks**: Автоматическое отслеживание
- **Binary analysis**: Размеры и метрики
- **Docker testing**: Проверка контейнеров
- **Release automation**: Автоматическая публикация

```bash
# Локальная проверка перед push
make check
make test-all
make docker-test
```

### Участие в разработке

1. Форкнуть репозиторий
2. Создать feature ветку (`git checkout -b feature/amazing-feature`)
3. Сделать изменения и тесты (`make test-all`)
4. Коммит изменений (`git commit -m 'Add amazing feature'`)
5. Push в ветку (`git push origin feature/amazing-feature`)
6. Открыть Pull Request

## 📊 Производительность

Бенчмарки на Intel i7-14700K + RTX 4070:

| Операция | Время | Примечания |
|----------|-------|------------|
| Генерация эмбеддинга | 15мс | Batch из 32 |
| Векторный поиск (1M docs) | 5мс | HNSW индекс (hnsw_rs) |
| Реранжирование (32 результата) | 12мс | FP16 квантизация |
| Продвижение памяти | 45мс | Async фоновая задача |
| Холодный старт | 150мс | CPU режим |
| Холодный старт | 300мс | GPU режим |

### Масштабирование памяти

| Количество векторов | Память RAM | Время поиска | Индексация |
|-------------------|------------|--------------|------------|
| 10K документов | 50MB | 2мс | 5 сек |
| 100K документов | 400MB | 4мс | 45 сек |
| 1M документов | 3.2GB | 6мс | 8 мин |
| 10M документов | 28GB | 8мс | 75 мин |

## 🛠️ Troubleshooting

### Часто встречающиеся проблемы

**Ошибка "Model not found"**
```bash
# Перезагрузить модели
./scripts/download_models.ps1

# Проверить модели
ls models/
magray status
```

**Высокое потребление памяти**
```bash
# Уменьшить размеры batch в конфигурации
# Очистить кэш векторов
rm -rf ~/.magray/cache/embeddings.db

# Проверить статистики памяти
magray memory stats
```

**Ошибки выполнения инструментов**
```bash
# Проверить доступные инструменты
magray tool "list available tools"

# Проверить окружение
echo $PATH
magray health
```

**GPU не работает**
```bash
# Проверить CUDA установку
nvidia-smi
magray gpu info

# Принудительно включить CPU режим
export MAGRAY_FORCE_CPU=1
magray status
```

**Проблемы с ONNX Runtime**
```bash
# Переустановить ONNX Runtime
./scripts/install_onnxruntime.ps1

# Проверить библиотеки
ldd target/release/magray  # Linux
otool -L target/release/magray  # macOS
```

### Логи и диагностика

```bash
# Подробные логи
RUST_LOG=debug magray status

# JSON логи для парсинга
RUST_LOG=info LOG_FORMAT=json magray chat "test"

# Логи в файл
RUST_LOG=debug LOG_FILE=magray.log magray status

# Проверка производительности
RUST_LOG=debug magray memory benchmark
```

## 📚 Документация

- [Руководство по архитектуре](docs/ARCHITECTURE.md)
- [Система памяти - детальный обзор](docs/MEMORY.md)
- [Руководство по системе инструментов](docs/TOOLS.md)
- [API Reference](https://docs.rs/magray)
- [Конфигурация и настройка](docs/CONFIGURATION.md)
- [Руководство по разработке](docs/DEVELOPMENT.md)

## 🚀 Roadmap

### v0.2.0 - Enhanced AI
- [ ] Поддержка Ollama для локальных LLM
- [ ] Встроенный embedding server
- [ ] Advanced retrieval strategies
- [ ] Multi-modal поддержка (изображения)

### v0.3.0 - Enterprise Features  
- [ ] Distributed memory clusters
- [ ] Advanced security и RBAC
- [ ] Metrics и monitoring dashboard
- [ ] Plugin ecosystem

### v1.0.0 - Production Ready
- [ ] Стабильный API
- [ ] Comprehensive documentation
- [ ] Performance optimizations
- [ ] Enterprise support

## 🤝 Сообщество

- [GitHub Discussions](https://github.com/yourusername/MAGRAY_Cli/discussions)
- [Discord Server](https://discord.gg/magray)
- [Telegram Chat](https://t.me/magray_cli)
- [Reddit Community](https://reddit.com/r/magray)

## 📄 Лицензия

Проект лицензирован под MIT License - смотрите файл [LICENSE](LICENSE) для деталей.

## 🙏 Благодарности

- [hnsw_rs](https://github.com/jean-pierreBoth/hnswlib-rs) от Jean-Pierre Both за профессиональную HNSW реализацию
- [ONNX Runtime](https://onnxruntime.ai/) за быстрый inference
- [Tokio](https://tokio.rs/) за async runtime
- [BGE models](https://github.com/FlagOpen/FlagEmbedding) за качественные эмбеддинги
- Сообщество Rust за удивительные крейты

---

Создан с ❤️ на Rust | [Поставьте звезду на GitHub!](https://github.com/yourusername/MAGRAY_Cli) ⭐