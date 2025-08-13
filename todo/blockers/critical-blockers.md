# 🚨 КРИТИЧЕСКИЕ БЛОКЕРЫ - НЕМЕДЛЕННОЕ ИСПРАВЛЕНИЕ

> **Эти задачи БЛОКИРУЮТ использование архитектурного ядра. Без них проект нефункционален.**

**⏰ Общее время**: 29 часов концентрированной работы  
**🎯 Результат**: Функциональный MVP с multi-agent workflow, memory system и basic UX

---

## БЛОКЕР 1: CLI Integration ❌ NOT_STARTED
**⏰ Время**: 2-3 часа | **🔥 Приоритет**: URGENT

**ПРОБЛЕМА**: 11,796 строк multi-agent orchestrator недоступны через CLI  
**РЕШЕНИЕ**: Интегрировать AgentOrchestrator в main.rs

### Задачи:

#### **БЛОКЕР-1.1** [2ч] Заменить UnifiedAgentV2 на AgentOrchestrator в main.rs
- **Шаги**: Импорт AgentOrchestrator (30м) → Замена инициализации (60м) → Тестирование (30м)
- **Критерий**: CLI использует multi-agent workflow
- **Статус**: ❌ NOT_STARTED

#### **БЛОКЕР-1.2** [1ч] Обновить CLI commands для orchestrator integration  
- **Шаги**: Обновить command handlers (30м) → Тестирование (30м)
- **Критерий**: Все CLI команды работают с orchestrator
- **Статус**: ❌ NOT_STARTED

**💡 Связанные файлы**:
- `crates/cli/src/main.rs`
- `crates/orchestrator/src/orchestrator.rs` 
- `crates/cli/src/commands/*.rs`

---

## БЛОКЕР 2: Qwen3 Embeddings ❌ NOT_STARTED
**⏰ Время**: 4-6 часов | **🔥 Приоритет**: URGENT

**ПРОБЛЕМА**: embeddings_qwen3.rs пустой (1 byte), memory system нефункционален  
**РЕШЕНИЕ**: Реализовать embedding generation

### Задачи:

#### **БЛОКЕР-2.1** [3ч] Реализовать Qwen3EmbeddingProvider
- **Шаги**: ONNX model loading (90м) → Tokenization (60м) → Embedding generation (30м)
- **Критерий**: Qwen3 генерирует embeddings
- **Статус**: ❌ NOT_STARTED

#### **БЛОКЕР-2.2** [2ч] Интегрировать embeddings в memory system
- **Шаги**: Memory service integration (60м) → Тестирование (60м)
- **Критерий**: Memory indexing работает с Qwen3
- **Статус**: ❌ NOT_STARTED

#### **БЛОКЕР-2.3** [1ч] Оптимизация и тестирование
- **Шаги**: Performance tuning (30м) → Integration tests (30м)
- **Критерий**: Embeddings performance приемлемый
- **Статус**: ❌ NOT_STARTED

**💡 Связанные файлы**:
- `crates/ai/src/embeddings_qwen3.rs` (сейчас пустой!)
- `crates/memory/src/*.rs`
- `models/Qwen3-Embedding-0.6B-ONNX/`

---

## БЛОКЕР 3: Tool Context Builder ❌ NOT_STARTED
**⏰ Время**: 6-8 часов | **🔥 Приоритет**: HIGH

**ПРОБЛЕМА**: Intelligent tool selection отсутствует  
**РЕШЕНИЕ**: Реализовать tool selection/reranking pipeline

### Задачи:

#### **БЛОКЕР-3.1** [3ч] Создать ToolContextBuilder
- **Шаги**: Context builder structure (90м) → Tool metadata extraction (60м) → Basic selection (30м)
- **Критерий**: ToolContextBuilder создает contexts
- **Статус**: ❌ NOT_STARTED

#### **БЛОКЕР-3.2** [3ч] Реализовать Qwen3 reranking для tools
- **Шаги**: Reranking pipeline (90м) → Integration с context builder (60м) → Тестирование (30м)
- **Критерий**: Tools ранжируются по relevance
- **Статус**: ❌ NOT_STARTED

#### **БЛОКЕР-3.3** [2ч] Интегрировать в orchestrator workflow
- **Шаги**: Orchestrator integration (60м) → End-to-end testing (60м)
- **Критерий**: Planner использует intelligent tool selection
- **Статус**: ❌ NOT_STARTED

**💡 Связанные файлы**:
- `crates/tools/src/context/builder.rs`
- `crates/tools/src/context/reranker.rs`
- `models/Qwen3-Reranker-0.6B-ONNX/`

---

## БЛОКЕР 4: Basic TUI Framework ❌ NOT_STARTED
**⏰ Время**: 8-12 часов | **🔥 Приоритет**: MEDIUM

**ПРОБЛЕМА**: Полное отсутствие TUI, Plan→Preview→Execute недоступен  
**РЕШЕНИЕ**: Минимальный TUI для MVP

### Задачи:

#### **БЛОКЕР-4.1** [4ч] Создать базовый TUI framework
- **Шаги**: TUI crate setup (60м) → Basic layout (90м) → Event handling (90м)
- **Критерий**: TUI запускается и отображается
- **Статус**: ❌ NOT_STARTED

#### **БЛОКЕР-4.2** [4ч] Реализовать plan viewer
- **Шаги**: Plan visualization (120м) → Interactive navigation (120м)
- **Критерий**: ActionPlan отображается в TUI
- **Статус**: ❌ NOT_STARTED

#### **БЛОКЕР-4.3** [4ч] Добавить basic diff display
- **Шаги**: Diff viewer (120м) → Accept/reject buttons (120м)
- **Критерий**: Plan→Preview→Execute workflow работает
- **Статус**: ❌ NOT_STARTED

**💡 Связанные файлы**:
- `crates/ui/src/` (нужно создать)
- `src/app.rs`, `src/main.rs` (TUI components)

---

## 🎯 План исправления блокеров

### Последовательность выполнения:

1. **[2-3ч] CLI Integration** - разблокирует orchestrator
2. **[4-6ч] Qwen3 Embeddings** - разблокирует memory system  
3. **[6-8ч] Tool Context Builder** - разблокирует intelligent tool selection
4. **[8-12ч] Basic TUI** - разблокирует Plan→Preview→Execute

### Milestone проверки:

- ✅ **После блокера 1**: CLI команды используют orchestrator
- ✅ **После блокера 2**: Memory indexing и поиск работают
- ✅ **После блокера 3**: Intelligent tool selection работает
- ✅ **После блокера 4**: Full MVP workflow доступен

### Критерии готовности MVP:

- [ ] CLI интегрирован с multi-agent orchestrator
- [ ] Memory system генерирует и использует embeddings
- [ ] Tool selection использует AI reranking
- [ ] Basic TUI отображает Plan→Preview→Execute

---

## 🔗 Связанные разделы

- **Security готовность**: [../phases/p0-security.md](../phases/p0-security.md) - 85% готово
- **Core архитектура**: [../phases/p1-core.md](../phases/p1-core.md) - ожидает блокеры
- **Прогресс-метрики**: [../progress/metrics.json](../progress/metrics.json)
- **Временные оценки**: [../architecture/time-estimates.md](../architecture/time-estimates.md)

---

*⚠️ ВАЖНО: Блокеры должны решаться в указанном порядке из-за зависимостей между компонентами*