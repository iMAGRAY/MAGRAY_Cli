# 📚 MAGRAY CLI API Reference

**Версия:** 0.2.0  
**Обновлено:** 2025-08-06

---

## 📋 Оглавление

1. [Командный интерфейс](#командный-интерфейс)
2. [Rust API](#rust-api)
3. [Конфигурация](#конфигурация)
4. [Примеры использования](#примеры-использования)
5. [Переменные окружения](#переменные-окружения)
6. [Коды ошибок](#коды-ошибок)

---

## 🖥️ Командный интерфейс

### Основные команды

#### `magray chat [message]`
Интерактивный чат с AI.

```bash
# Интерактивный режим
magray chat

# Одиночное сообщение
magray chat "Как оптимизировать Rust код?"

# Из pipe
echo "Объясни SOLID принципы" | magray chat
```

#### `magray smart <task>`
Умное планирование и выполнение сложных задач.

```bash
magray smart "проанализируй кодовую базу и найди проблемы производительности"
magray smart "создай REST API для управления пользователями"
```

#### `magray tool <action>`
Выполнение действий через естественный язык.

```bash
magray tool "покажи git статус"
magray tool "создай файл test.rs с hello world"
magray tool "найди все TODO в коде"
```

### Команды управления памятью

#### `magray memory <subcommand>`

```bash
# Поиск в памяти
magray memory search "vector search" --layer insights --top-k 20

# Добавление записи
magray memory add "Важная информация" --layer insights --tags "api,docs"

# Статистика
magray memory stats

# Резервное копирование
magray memory backup --name "backup-2025-01-06"

# Восстановление
magray memory restore --name "backup-2025-01-06"

# Промоушн записей между слоями
magray memory promote --from interact --to insights --threshold 0.8
```

### GPU управление

#### `magray gpu <subcommand>`

```bash
# Информация о GPU
magray gpu info

# Статус использования
magray gpu status

# Бенчмарки
magray gpu benchmark --batch-size 100 --iterations 1000

# Очистка кэша
magray gpu clear-cache

# Сравнение CPU vs GPU
magray gpu benchmark --compare
```

### Управление моделями

#### `magray models <subcommand>`

```bash
# Список моделей
magray models list

# Загрузка модели
magray models download qwen3-embeddings

# Информация о модели
magray models info qwen3-embeddings

# Проверка целостности
magray models verify

# Удаление модели
magray models remove bge-m3
```

### Системные команды

#### `magray health`
Проверка здоровья системы.

```bash
magray health
# ✓ LLM Service: Connected
# ✓ Memory Service: Healthy (87% cache hit)
# ✓ GPU: Available (RTX 4070, 12GB)
# ✓ Models: Loaded (Qwen3, BGE-M3)
```

#### `magray status`
Полный статус системы.

```bash
magray status
# === MAGRAY System Status ===
# ✓ LLM Service: Connected (OpenAI)
# ✓ Memory Service: Healthy (1234 records, 87.3% cache hit)
# ℹ Binary: v0.2.0 (16.2 MB)
# ℹ Log Level: info
```

#### `magray performance`
Метрики производительности.

```bash
magray performance
# === MAGRAY Performance Metrics ===
# Cache Hit Rate: 87.3%
# Avg Resolve Time: 12.5μs
# Total Resolves: 15234
# Factory Creates: 234
# Singleton Creates: 45
```

---

## 🦀 Rust API

### Основные трейты

#### RequestProcessorTrait

```rust
use cli::agent_traits::RequestProcessorTrait;

#[async_trait]
pub trait RequestProcessorTrait: Send + Sync {
    async fn process_user_request(
        &self,
        context: RequestContext,
    ) -> Result<ProcessingResult>;
    
    async fn initialize(&mut self) -> Result<()>;
    async fn shutdown(&mut self) -> Result<()>;
}
```

#### MemoryServiceTrait

```rust
use memory::traits::MemoryServiceTrait;

#[async_trait]
pub trait MemoryServiceTrait: Send + Sync {
    async fn search(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>>;
    
    async fn store(
        &self,
        record: MemoryRecord,
    ) -> Result<()>;
    
    async fn promote_memories(&self) -> Result<PromotionStats>;
    
    async fn get_stats(&self) -> MemoryStats;
}
```

### Использование UnifiedAgentV2

```rust
use cli::unified_agent_v2::UnifiedAgentV2;
use cli::agent_traits::{RequestContext, AgentResponse};

#[tokio::main]
async fn main() -> Result<()> {
    // Создание и инициализация агента
    let mut agent = UnifiedAgentV2::new().await?;
    agent.initialize().await?;
    
    // Обработка запроса
    let context = RequestContext {
        message: "Как оптимизировать Rust код?".to_string(),
        session_id: "session-123".to_string(),
        metadata: HashMap::new(),
    };
    
    let result = agent.process_user_request(context).await?;
    
    match result.response {
        AgentResponse::Chat(text) => println!("AI: {}", text),
        AgentResponse::ToolExecution(result) => println!("Result: {}", result),
        AgentResponse::Admin(admin) => println!("Admin: {:?}", admin),
        AgentResponse::Error(err) => eprintln!("Error: {}", err),
    }
    
    Ok(())
}
```

### Работа с памятью

```rust
use memory::{DIMemoryService, SearchOptions, Layer};

#[tokio::main]
async fn main() -> Result<()> {
    let config = memory::default_config()?;
    let memory = DIMemoryService::new(config).await?;
    
    // Поиск
    let options = SearchOptions::builder()
        .layers(vec![Layer::Interact, Layer::Insights])
        .top_k(20)
        .threshold(0.7)
        .build();
    
    let results = memory.search("rust optimization", options).await?;
    
    // Сохранение
    let record = MemoryRecord::new(
        "Rust optimization tip: use iterators instead of loops",
        Layer::Insights,
    );
    memory.store(record).await?;
    
    Ok(())
}
```

### Multi-Provider LLM

```rust
use llm::{LlmClient, ProviderType};

#[tokio::main]
async fn main() -> Result<()> {
    // Multi-provider клиент
    let client = LlmClient::from_env_multi()?;
    
    // Выбор конкретного провайдера
    let response = client
        .with_provider(ProviderType::OpenAI)
        .with_model("gpt-4o-mini")
        .chat("Explain SOLID principles")
        .await?;
    
    // Автоматический failover
    let response = client
        .with_fallback()
        .chat("Complex question")
        .await?;
    
    Ok(())
}
```

### Работа с инструментами

```rust
use tools::{Tool, ToolRegistry, ToolInput};

#[tokio::main]
async fn main() -> Result<()> {
    let registry = ToolRegistry::new();
    
    // Регистрация инструмента
    registry.register(Box::new(FileReadTool::new()));
    registry.register(Box::new(GitStatusTool::new()));
    
    // Выполнение через естественный язык
    let best_tool = registry
        .find_tool_for_request("показать git статус")
        .await?;
    
    let input = ToolInput::from_natural_language("показать git статус");
    let output = best_tool.execute(input).await?;
    
    Ok(())
}
```

---

## ⚙️ Конфигурация

### Файл конфигурации

Расположение: `~/.magray/config.toml`

```toml
[ai]
embed_model = "qwen3"
embed_batch_size = 32
use_gpu = true
max_sequence_length = 8192

[ai.llm]
provider = "openai"
model = "gpt-4o-mini"
max_tokens = 2048
temperature = 0.7
retry_attempts = 3
timeout_seconds = 30

[memory]
database_path = "~/.magray/memory.db"
interact_ttl_hours = 24
insights_ttl_days = 90
assets_ttl_days = 0  # Бесконечно
promote_threshold = 0.8
max_vectors_per_layer = 100000
cache_size_mb = 1024

[memory.hnsw]
max_connections = 24
ef_construction = 400
ef_search = 100
distance = "cosine"

[tools]
enable_network = true
plugin_dir = "~/.magray/plugins"
max_file_size_mb = 100
shell_timeout_seconds = 30

[logging]
level = "info"  # trace, debug, info, warn, error
json_output = false
file_output = true
file_path = "~/.magray/magray.log"
max_file_size_mb = 100
max_files = 5
```

---

## 💡 Примеры использования

### Базовый workflow

```bash
# 1. Инициализация и проверка
magray health

# 2. Интерактивный чат
magray chat
> Как оптимизировать векторный поиск?

# 3. Работа с файлами
magray tool "создай файл optimization.md с советами по оптимизации"
magray tool "добавь в optimization.md раздел про SIMD"

# 4. Анализ кода
magray smart "проанализируй src/ и найди проблемы производительности"

# 5. Сохранение важной информации
magray memory add "SIMD дает 8x ускорение для векторных операций" --layer insights
```

### Сложный пример с пайплайном

```bash
#!/bin/bash

# Анализ проекта и генерация отчета
PROJECT_DIR="/path/to/project"

# 1. Анализ архитектуры
magray smart "проанализируй архитектуру проекта в $PROJECT_DIR" > architecture.md

# 2. Поиск проблем
magray tool "найди все TODO и FIXME в $PROJECT_DIR" > todos.txt

# 3. Генерация документации
magray smart "создай README.md для проекта на основе анализа"

# 4. Проверка безопасности
magray tool "проверь Cargo.toml на устаревшие зависимости"

# 5. Сохранение результатов
magray memory add "Анализ проекта завершен: $(date)" --layer insights
```

### Интеграция в CI/CD

```yaml
# .github/workflows/magray-analysis.yml
name: MAGRAY Code Analysis

on: [push, pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install MAGRAY
        run: |
          curl -L https://github.com/yourusername/MAGRAY_Cli/releases/latest/download/magray-linux-amd64 -o magray
          chmod +x magray
          
      - name: Run Analysis
        env:
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
        run: |
          ./magray smart "проанализируй изменения и проверь на проблемы"
          
      - name: Comment PR
        if: github.event_name == 'pull_request'
        run: |
          ANALYSIS=$(./magray tool "создай краткий отчет об анализе")
          echo "$ANALYSIS" | gh pr comment --body-file -
```

---

## 🌍 Переменные окружения

### Обязательные

| Переменная | Описание | Пример |
|------------|----------|--------|
| `LLM_PROVIDER` | Основной LLM провайдер | `openai`, `anthropic`, `groq` |
| `OPENAI_API_KEY` | API ключ OpenAI | `sk-...` |

### Опциональные

| Переменная | Описание | По умолчанию |
|------------|----------|--------------|
| `ANTHROPIC_API_KEY` | API ключ Anthropic | - |
| `GROQ_API_KEY` | API ключ Groq | - |
| `OLLAMA_URL` | URL Ollama сервера | `http://localhost:11434` |
| `LMSTUDIO_URL` | URL LMStudio | `http://localhost:1234` |
| `RUST_LOG` | Уровень логирования | `info` |
| `LOG_FORMAT` | Формат логов | `text` |
| `MAGRAY_CONFIG` | Путь к конфигу | `~/.magray/config.toml` |
| `MAGRAY_FORCE_CPU` | Отключить GPU | `false` |
| `MAGRAY_CACHE_DIR` | Директория кэша | `~/.magray/cache` |

---

## ❌ Коды ошибок

### Системные ошибки (1xxx)

| Код | Описание | Решение |
|-----|----------|---------|
| 1001 | Не найден конфигурационный файл | Создайте `~/.magray/config.toml` |
| 1002 | Ошибка инициализации | Проверьте логи и переменные окружения |
| 1003 | Недостаточно памяти | Увеличьте лимиты или используйте minimal версию |

### LLM ошибки (2xxx)

| Код | Описание | Решение |
|-----|----------|---------|
| 2001 | Нет доступных провайдеров | Настройте хотя бы один LLM провайдер |
| 2002 | API ключ недействителен | Проверьте API ключи |
| 2003 | Превышен лимит запросов | Подождите или используйте другой провайдер |
| 2004 | Таймаут запроса | Увеличьте timeout или упростите запрос |

### Memory ошибки (3xxx)

| Код | Описание | Решение |
|-----|----------|---------|
| 3001 | База данных недоступна | Проверьте путь к БД и права доступа |
| 3002 | Ошибка индексации | Пересоздайте HNSW индекс |
| 3003 | Превышен лимит векторов | Увеличьте `max_vectors_per_layer` |

### Tool ошибки (4xxx)

| Код | Описание | Решение |
|-----|----------|---------|
| 4001 | Инструмент не найден | Проверьте доступные инструменты |
| 4002 | Ошибка выполнения | Проверьте права доступа и параметры |
| 4003 | Таймаут операции | Увеличьте `shell_timeout_seconds` |

---

## 📖 Дополнительная документация

- [Архитектура](ARCHITECTURE.md) - Детальное описание архитектуры
- [Система памяти](MEMORY_SYSTEM_ARCHITECTURE.md) - 3-слойная память с HNSW
- [GPU ускорение](GPU_ACCELERATION.md) - Настройка и оптимизация GPU
- [Миграция](MIGRATION_GUIDE.md) - Переход на новые версии
- [Troubleshooting](troubleshooting/Troubleshooting%20Guide%20-%20Common%20Issues%20%26%20Solutions.md) - Решение проблем

---

**Создано с ❤️ на Rust** | [GitHub](https://github.com/yourusername/MAGRAY_Cli) ⭐