# План реализации MAGRAY CLI согласно архитектуре

## Фаза 1: Core компоненты (критично)

### 1.1 Request Gateway
```rust
// crates/gateway/src/lib.rs
- Парсинг CLI команд и аргументов
- Валидация и нормализация запросов
- Создание структуры Request с контекстом
- Интеграция с Config/Feature Flags
```

### 1.2 TodoService/TaskBoard
```rust
// crates/todo/src/lib.rs
- CRUD операций для задач
- Состояния: Planned, Ready, InProgress, Blocked, Done
- Приоритизация и staleness tracking
- Синхронизация с планировщиком
- Хранение в tasks.db
```

### 1.3 Graph Planner (DAG)
```rust
// crates/planner/src/lib.rs
- Построение DAG из задач
- Топологическая сортировка
- Определение зависимостей
- Интеграция с ToolService и Memory
```

### 1.4 Executor
```rust
// crates/executor/src/lib.rs
- Исполнение DAG по шагам
- Retry логика и таймауты
- Публикация событий в EventBus
- Контроль политик (Policy/Guardrails)
```

## Фаза 2: Интеграция и Services

### 2.1 EventBus
```rust
// crates/eventbus/src/lib.rs
- Pub/sub для событий системы
- Логирование в events.log
- Метрики в metrics.json
- Интеграция с tracing
```

### 2.2 PromptBuilder
```rust
// crates/prompt/src/lib.rs
- Сборка промптов с контекстом из Memory
- Retrieval + Rerank pipeline
- Шаблоны для разных задач
```

### 2.3 Scheduler
```rust
// crates/scheduler/src/lib.rs
- Фоновые задачи (reindex, cleanup)
- Cron-like scheduling
- Интеграция с Memory и Todo
```

### 2.4 Config Service
```rust
// crates/config/src/lib.rs
- Централизованная конфигурация
- Feature flags
- Управление путями DocStore
```

## Фаза 3: Рефакторинг существующего

### 3.1 Обновить CLI
- Использовать Request Gateway вместо прямых вызовов
- Добавить поддержку project_id
- Интегрировать с TodoService

### 3.2 Обновить Router
- Использовать DAG Planner
- Интегрировать с Memory для контекста
- Подключить к EventBus

### 3.3 Создать DocStore
- Реализовать хранение в ~/.ourcli/projects/
- Управление project_id (hash от пути)
- Миграция существующих данных

## Фаза 4: Полировка

### 4.1 Policy/Guardrails
- Лимиты на токены, время, ресурсы
- Контроль доступа к инструментам
- Валидация безопасности

### 4.2 Observability
- Полноценный tracing
- Prometheus-совместимые метрики
- Дашборды и алерты

### 4.3 Тестирование
- Unit тесты для каждого компонента
- Интеграционные тесты
- E2E сценарии

## Приоритеты

1. **TodoService** - основа для управления задачами
2. **Request Gateway + Executor** - обработка команд
3. **EventBus** - наблюдаемость системы
4. **DAG Planner** - умное планирование
5. **DocStore** - правильное хранение данных

## Примерная структура после реализации

```
MAGRAY_Cli/
├── crates/
│   ├── cli/          # Обновленный CLI
│   ├── gateway/      # NEW: Request Gateway
│   ├── todo/         # NEW: TodoService
│   ├── planner/      # NEW: DAG Planner
│   ├── executor/     # NEW: Executor
│   ├── eventbus/     # NEW: EventBus
│   ├── prompt/       # NEW: PromptBuilder
│   ├── scheduler/    # NEW: Scheduler
│   ├── config/       # NEW: Config Service
│   ├── policy/       # NEW: Policy/Guardrails
│   ├── memory/       # Существующий (хороший)
│   ├── llm/          # Существующий (хороший)
│   ├── tools/        # Существующий (хороший)
│   └── router/       # Требует обновления
```