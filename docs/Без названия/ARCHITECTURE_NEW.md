# MAGRAY CLI - Архитектура системы

## 📐 Обзор архитектуры

MAGRAY CLI построен на модульной архитектуре с четким разделением ответственности между компонентами.

```
┌─────────────────────────────────────────────────────┐
│                    CLI Interface                     │
│              (Animated, Interactive)                 │
└─────────────────┬───────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────┐
│                  Smart Router                        │
│         (Intent Analysis & Orchestration)            │
└──┬──────────────┬───────────────┬──────────────┬───┘
   │              │               │              │
┌──▼───┐    ┌────▼────┐    ┌────▼────┐    ┌────▼────┐
│ LLM  │    │  Tools  │    │ Memory  │    │   AI    │
│Agent │    │Registry │    │ System  │    │Embeddings│
└──────┘    └─────────┘    └─────────┘    └─────────┘
```

## 🧩 Основные компоненты

### 1. CLI (`crates/cli/`)
**Ответственность**: Пользовательский интерфейс и взаимодействие

- **Animated UI**: Прогресс-бары, спиннеры, цветной вывод
- **Command Parser**: Clap-based парсинг команд
- **Interactive Mode**: REPL с историей и автодополнением
- **Status Display**: Красивый вывод системной информации

```rust
// Основная структура команд
pub enum Command {
    Chat { message: String },
    Smart { query: String },
    Tool { tool: String, args: Vec<String> },
    Gpu { subcommand: Vec<String> },
    Models { subcommand: Vec<String> },
    Status,
    Version,
    Interactive,
}
```

### 2. Smart Router (`crates/router/`)
**Ответственность**: Интеллектуальная маршрутизация и оркестрация

- **Intent Analysis**: Определение типа запроса (chat/tools)
- **Action Planning**: Создание многошаговых планов
- **Execution Engine**: Выполнение планов с обработкой ошибок
- **Context Management**: Управление контекстом между шагами

```rust
pub struct SmartRouter {
    llm_client: LlmClient,
    tool_registry: ToolRegistry,
    intent_analyzer: IntentAnalyzerAgent,
    action_planner: ActionPlannerAgent,
}
```

### 3. LLM System (`crates/llm/`)
**Ответственность**: Интеграция с языковыми моделями

- **Multi-Provider Support**: OpenAI, Anthropic, Local
- **Specialized Agents**:
  - `IntentAnalyzerAgent`: Анализ намерений
  - `ToolSelectorAgent`: Выбор инструментов
  - `ParameterExtractorAgent`: Извлечение параметров
  - `ActionPlannerAgent`: Планирование действий

```rust
pub enum LlmProvider {
    OpenAI { api_key: String, model: String },
    Anthropic { api_key: String, model: String },
    Local { url: String, model: String },
}
```

### 4. Memory System (`crates/memory/`)
**Ответственность**: Многоуровневое хранилище с векторным поиском

#### Три уровня памяти:
1. **Interact Layer** (24h TTL)
   - Текущая сессия
   - Недавние взаимодействия
   - Временный контекст

2. **Insights Layer** (90d TTL)
   - Извлеченные знания
   - Обученные паттерны
   - Агрегированная информация

3. **Assets Layer** (Permanent)
   - Код и документация
   - Важные данные
   - Долгосрочное хранилище

#### Ключевые компоненты:
- **HNSW Index**: O(log n) векторный поиск
- **Time-based Indices**: Эффективная промоция по времени
- **ML Promotion Engine**: Умная промоция между слоями
- **Embedding Cache**: LRU кэш для эмбеддингов

```rust
pub struct VectorStore {
    indices: HashMap<Layer, HnswIndex>,
    metadata: HashMap<Uuid, RecordMetadata>,
    time_indices: HashMap<Layer, BTreeMap<DateTime<Utc>, Vec<Uuid>>>,
}
```

### 5. AI/Embeddings (`crates/ai/`)
**Ответственность**: ONNX-based эмбеддинги и ранжирование

- **Qwen3 Embeddings**: 1024-мерные векторы (primary), BGE-M3 1024D (legacy)
- **BGE Reranker v2-m3**: Переранжирование результатов
- **GPU Acceleration**: CUDA поддержка с fallback на CPU
- **Batch Processing**: Эффективная обработка батчами
- **Model Registry**: Централизованное управление моделями

```rust
pub struct EmbeddingService {
    embedder: Box<dyn Embedder>,
    reranker: Box<dyn Reranker>,
    tokenizer: OptimizedTokenizer,
    gpu_fallback: GpuFallbackManager,
}
```

### 6. Tools System (`crates/tools/`)
**Ответственность**: Расширяемая система инструментов

Встроенные инструменты:
- **File Operations**: read, write, list
- **Git Operations**: status, commit, diff
- **Web Operations**: search, fetch
- **Shell Operations**: execute commands

```rust
pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput>;
    fn supports_natural_language(&self) -> bool;
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput>;
}
```

### 7. TODO System (`crates/todo/`)
**Ответственность**: Управление задачами и DAG

- **SQLite Storage**: Персистентное хранение
- **DAG Structure**: Зависимости между задачами
- **Priority Queue**: Приоритизация выполнения
- **Progress Tracking**: Отслеживание прогресса

### 8. Common (`crates/common/`)
**Ответственность**: Общая функциональность

- **Structured Logging**: JSON логирование для production
- **Performance Metrics**: Сбор метрик
- **Error Handling**: Унифицированная обработка ошибок
- **Configuration**: Управление конфигурацией

## 🔄 Поток данных

### Обработка команды чата:
```
User Input → CLI → SmartRouter → IntentAnalyzer → LLM → Response → CLI → User
```

### Обработка команды с инструментами:
```
User Input → CLI → SmartRouter → IntentAnalyzer → ActionPlanner → 
→ ToolSelector → ParameterExtractor → Tool Execution → Result → CLI → User
```

### Работа с памятью:
```
New Data → Embedding Service → Vector Store (Interact) → 
→ ML Promotion Engine → Insights Layer → Assets Layer
```

## 🏗️ Технологический стек

### Основные зависимости:
- **Tokio**: Асинхронный runtime
- **Clap**: Парсинг CLI аргументов
- **ONNX Runtime**: Инференс нейронных сетей
- **hnsw_rs**: HNSW индекс для векторного поиска
- **SQLite/rusqlite**: Хранение метаданных и TODO
- **Sled**: Встраиваемая БД для кэша
- **Reqwest**: HTTP клиент для API
- **Indicatif**: Прогресс-бары и анимации

### Условная компиляция:
```toml
[features]
default = ["cpu"]
cpu = []
gpu = ["ort/cuda"]
minimal = []
```

## 🚀 Оптимизации производительности

### 1. Векторный поиск
- HNSW индекс с M=16, ef=200
- Время поиска: <5мс для 1M векторов
- Параллельный поиск по слоям

### 2. Эмбеддинги
- Батчевая обработка (до 32 текстов)
- GPU ускорение с автоматическим fallback
- Кэширование результатов в Sled

### 3. Память
- Time-based индексы для O(log n) промоции
- Ленивая загрузка векторов
- Сжатие метаданных

### 4. Сборка
- LTO для уменьшения размера
- Strip символов в release
- Target CPU native

## 🔐 Безопасность

### Принципы:
1. **No Secrets in Code**: Все ключи через env
2. **Input Validation**: Проверка всех входных данных
3. **Safe Shell Execution**: Экранирование команд
4. **Path Traversal Protection**: Проверка путей
5. **Rate Limiting**: Ограничение запросов к API

### Изоляция:
- Каждый инструмент в своем контексте
- Ограничения на выполнение команд
- Аудит всех операций

## 📦 Модульность и расширяемость

### Добавление нового инструмента:
```rust
pub struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "my_tool".to_string(),
            description: "My custom tool".to_string(),
            // ...
        }
    }
    
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        // Implementation
    }
}
```

### Добавление нового LLM провайдера:
1. Добавить вариант в `LlmProvider`
2. Реализовать метод в `LlmClient`
3. Обновить `from_env()`

### Добавление нового слоя памяти:
1. Добавить вариант в `Layer` enum
2. Настроить TTL и политики
3. Обновить ML promotion engine

## 🧪 Тестирование

### Уровни тестов:
1. **Unit Tests**: Для каждого модуля
2. **Integration Tests**: Межмодульное взаимодействие
3. **E2E Tests**: Полные сценарии использования
4. **Performance Tests**: Бенчмарки критических путей

### Покрытие:
- Текущее: 35.4%
- Целевое: 80%
- CI/CD: GitHub Actions с матрицей фич

## 📈 Метрики и мониторинг

### Собираемые метрики:
- Время выполнения операций
- Использование памяти
- Cache hit/miss ratio
- Количество токенов LLM
- Ошибки и их типы

### Логирование:
```rust
// Структурированное логирование
let entry = StructuredLogEntry {
    timestamp: Utc::now().to_rfc3339(),
    level: "INFO".to_string(),
    target: module_path!().to_string(),
    message: "Operation completed".to_string(),
    fields: context.into(),
    performance: Some(metrics),
};
```

## 🚢 Deployment

### Docker образы:
- **CPU**: Базовый образ (~100MB)
- **GPU**: С CUDA поддержкой (~2GB)
- **Minimal**: Минимальный набор (~50MB)

### Бинарная поставка:
- Single binary: ~16MB
- Статическая линковка где возможно
- Автоматическая загрузка моделей

## 🔮 Будущие улучшения

1. **Распределенная память**: Sharding для масштабирования
2. **Streaming responses**: Потоковый вывод из LLM
3. **Plugin система**: Динамическая загрузка расширений
4. **Web UI**: Браузерный интерфейс
5. **Collaborative features**: Многопользовательский режим