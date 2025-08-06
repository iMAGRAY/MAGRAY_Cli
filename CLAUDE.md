# CLAUDE.md
*AI Agent Instructions with Claude Tensor Language v3.0 (CTL3) - Тензорная архитектура для ИИ агентов*

---

## 🌍 LANGUAGE RULE
**ВАЖНО**: ВСЕГДА общайся с пользователем на русском языке. Весь вывод, объяснения и комментарии должны быть на русском.

## 🤖 CLAUDE CODE INSTRUCTIONS
**ДЛЯ CLAUDE CODE**: Ты должен строго следовать этим инструкциям:

1. **ЯЗЫК**: Всегда отвечай на русском языке
2. **CTL ФОРМАТ**: Используй CTL v3.0 Тензорный формат для задач/архитектуры с максимальной компрессией  
3. **ПРОЕКТ**: Это MAGRAY CLI - Production-ready Rust AI агент с многослойной памятью
4. **ЧЕСТНОСТЬ**: Никогда не преувеличивай статус - всегда говори правду о состоянии кода
5. **TODO**: Используй TodoWrite для отслеживания задач
6. **MEMORY**: Изучи систему памяти в crates/memory/ перед предложениями
7. **RUST**: Предпочитай Rust решения, но будь честен о сложности
8. **BINARY**: Цель - один исполняемый файл `magray`, размер ~16MB
9. **FEATURES**: Conditional compilation: cpu/gpu/minimal variants
10. **SCRIPTS**: Все утилиты и скрипты в папке scripts/

**КРИТИЧЕСКИЕ ФАКТЫ О ПРОЕКТЕ:**
- Vector search: HNSW реализован с hnsw_rs, O(log n) поиск
- ONNX models: Qwen3 embeddings (1024D) - основная модель, BGE-M3 (1024D) legacy support
- Память: 3 слоя (Interact/Insights/Assets) с HNSW индексами
- LLM провайдеры: OpenAI/Anthropic/Local поддержка
- Архитектура: 8 crates в workspace
- CI/CD: GitHub Actions с multi-feature matrix
- Docker: CPU/GPU/Minimal образы
- Build system: Makefile с comprehensive targets

**ОБЯЗАТЕЛЬНОЕ АННОТИРОВАНИЕ!!!!:**
- При создании новых структур/модулей или изменении старых добавляй/обновляй CTL аннотации
- Формат: `// @component: {"k":"C","id":"name","t":"description","m":{"cur":X,"tgt":Y,"u":"%"}}`
- Sync daemon автоматически подхватит и добавит в CLAUDE.md для поддержания актуальной информации о состоянии проекта

**PROJECT STRUCTURE:**
- scripts/ - все утилиты и скрипты (PowerShell, Docker, Python)
- scripts/docker/ - Docker образы для CPU/GPU/Minimal
- .github/ - CI/CD workflows для multi-platform builds
- Makefile - основная система сборки
- crates/ - 8 Rust workspace crates
- docs/ - техническая документация


(Existing file content continues...)

## 🤖 ОБЯЗАТЕЛЬНЫЕ ПРАВИЛА ДЛЯ CLAUDE CODE

**ЭТИ ПРАВИЛА НЕ ПОДЛЕЖАТ ОБСУЖДЕНИЮ:**

1. **РУССКИЙ ЯЗЫК ВЕЗДЕ** - каждый ответ, комментарий, объяснение
2. **ЧЕСТНОСТЬ ПРЕЖДЕ ВСЕГО** - никаких преувеличений статуса
3. **CTL v3.0 ТЕНЗОРНЫЙ ФОРМАТ** - компрессированные тензорные операции для задач/архитектуры
4. **TODOWRITE ОБЯЗАТЕЛЬНО** - для каждой многошаговой задачи
5. **ИЗУЧАЙ ПЕРЕД ДЕЙСТВИЕМ** - читай код перед предложениями
6. **ОДНА ЦЕЛЬ = ОДИН ФАЙЛ** - magray binary, а не ourcli
7. **НЕ ХВАСТАЙСЯ УСПЕХАМИ** - в конце каждой задачи сообщай что НЕ сделано
8. **ПОМЕЧАЙ КОД CTL АННОТАЦИЯМИ** - добавляй @component к каждому новому компоненту
9. **ОБЯЗАТЕЛЬНАЯ СИНХРОНИЗАЦИЯ CLAUDE.MD** - после каждой задачи используй @agent-claude-md-orchestrator
10. **ЗАПРЕТ НА МЕТРИКИ ПРОЕКТА В CLAUDE.MD** - НИКОГДА не записывай процентные метрики готовности, покрытия тестами, размеры файлов и другие метрики состояния проекта в CLAUDE.md. CTL аннотации с метриками m.cur/m.tgt должны быть ТОЛЬКО в коде

## 📝 Memory Section

### Additional Memory Guidelines
- **Старайся чаще выступать в роли оркестратора, Всегда используя соответствующих агентов для выполнения подходящих задач, если есть подходящие агенты**

## 🎯 ПРАВИЛА ЭФФЕКТИВНОГО ИСПОЛЬЗОВАНИЯ СПЕЦИАЛИЗИРОВАННЫХ АГЕНТОВ

**КРИТИЧНО**: При работе с MAGRAY_CLI проектом ты ОБЯЗАН использовать специализированных агентов из `.claude/agents/` для максимальной эффективности.

### 🔄 НОВАЯ СИСТЕМА КООРДИНАЦИИ АГЕНТОВ

**ВАЖНО**: Все агенты теперь обязаны использовать файл `agent_workflow.md` в корне проекта для координации работы. Это предотвращает конфликты при параллельной работе нескольких агентов.

**Ключевые компоненты координации:**
- **agent_workflow.md** - центральный файл координации со статусами всех агентов
- **ACTIVE AGENTS** - текущие работающие агенты с их задачами
- **FILE LOCKS** - заблокированные файлы для предотвращения конфликтов
- **WORK QUEUE** - очередь задач для распределения между агентами
- **CONFLICTS** - активные конфликты требующие разрешения

**Приоритеты агентов при конфликтах:**
- P0 (критический): rust-architect-supreme, ai-architecture-maestro, agent-claude-md-orchestrator
- P1 (высокий): rust-refactoring-master, rust-performance-virtuoso  
- P2 (средний): rust-code-optimizer, rust-quality-guardian, devops-orchestration-master
- P3 (низкий): obsidian-docs-architect, ctl-annotation-updater

### 📋 Алгоритм оркестрации агентов:

1. **АНАЛИЗ ЗАДАЧИ** → Определи ключевые аспекты (архитектура, производительность, качество, AI, DevOps)
2. **ВЫБОР СПЕЦИАЛИСТОВ** → Подбери агентов под каждый аспект задачи
3. **ПОСЛЕДОВАТЕЛЬНОЕ ДЕЛЕГИРОВАНИЕ** → Используй агентов в логичном порядке (архитектура → код → тесты → документация)
4. **ИНТЕГРАЦИЯ РЕЗУЛЬТАТОВ** → Объедини рекомендации всех агентов в единое решение
5. **ОБЯЗАТЕЛЬНАЯ СИНХРОНИЗАЦИЯ** → Всегда завершай задачу вызовом @agent-claude-md-orchestrator для обновления CLAUDE.md

### 🗂️ Матрица распределения задач по агентам:

| Тип задачи | Первичный агент | Поддерживающие агенты | Порядок |
|------------|-----------------|----------------------|---------|
| **Декомпозиция God Objects** | @rust-architect-supreme | @rust-refactoring-master → @rust-quality-guardian | 1→2→3 |
| **Оптимизация производительности** | @rust-performance-virtuoso | @ai-architecture-maestro → @rust-code-optimizer | 1→2→3 |
| **Увеличение test coverage** | @rust-quality-guardian | @rust-architect-supreme (для тестируемой архитектуры) | 1+2 |
| **AI/ML компоненты** | @ai-architecture-maestro | @rust-performance-virtuoso → @rust-code-optimizer | 1→2→3 |
| **CI/CD настройка** | @devops-orchestration-master | @rust-quality-guardian (для интеграции тестов) | 1+2 |
| **Архитектурная документация** | @obsidian-docs-architect | @ctl-annotation-updater (для синхронизации) | 1+2 |
| **CTL аннотации** | @ctl-annotation-updater | - | 1 |
| **Рефакторинг кода** | @rust-refactoring-master | @rust-quality-guardian → @obsidian-docs-architect | 1→2→3 |
| **Обновление CLAUDE.md** | @agent-claude-md-orchestrator | - | ЗАВЕРШАЮЩИЙ |

### 🔄 Примеры правильной оркестрации:

**ЗАДАЧА: Исправить UnifiedAgent (God Object с 17 зависимостями)**
```
1. @rust-architect-supreme - Анализ и план декомпозиции на traits
2. @rust-refactoring-master - Пошаговый рефакторинг без поломки функциональности
3. @rust-quality-guardian - Создание unit тестов для каждого нового компонента
4. @ctl-annotation-updater - Добавление CTL аннотаций к новым компонентам
5. @obsidian-docs-architect - Обновление архитектурной документации
6. @agent-claude-md-orchestrator - Финальное обновление CLAUDE.md с новой архитектурой
```

**ЗАДАЧА: Ускорить векторный поиск HNSW**
```
1. @rust-performance-virtuoso - Профилирование и поиск узких мест
2. @ai-architecture-maestro - Оптимизация embedding pipeline и батчинга
3. @rust-code-optimizer - Применение SIMD инструкций и zero-copy
4. @devops-orchestration-master - Настройка метрик производительности
5. @agent-claude-md-orchestrator - Синхронизация CTL аннотаций из кода в CLAUDE.md
```

**ЗАДАЧА: Подготовка к production релизу**
```
1. @rust-quality-guardian - Улучшение покрытия тестами
2. @rust-architect-supreme - Финальная проверка архитектуры на SOLID
3. @rust-performance-virtuoso - Оптимизация критических путей
4. @devops-orchestration-master - Настройка полного CI/CD pipeline
5. @obsidian-docs-architect - Создание production документации
6. @agent-claude-md-orchestrator - Финальное обновление CLAUDE.md перед релизом
```

### ⚠️ ОБЯЗАТЕЛЬНЫЕ ПРАВИЛА:

1. **НЕ ДЕЛАЙ ВСЁ САМ** - если есть специализированный агент, используй его
2. **ОБЪЯСНЯЙ ВЫБОР** - всегда поясняй почему выбрал конкретного агента
3. **СОБЛЮДАЙ ПОРЯДОК** - архитектура → реализация → тесты → документация
4. **ИНТЕГРИРУЙ РЕЗУЛЬТАТЫ** - следи за согласованностью рекомендаций агентов
5. **ИСПОЛЬЗУЙ ПАРАЛЛЕЛЬНО** - когда задачи независимы (например, тесты + документация)
6. **ЗАВЕРШАЙ СИНХРОНИЗАЦИЕЙ** - обязательно используй @agent-claude-md-orchestrator в конце каждой задачи

### 📊 Метрики эффективности использования агентов:

- **Coverage агентами**: какой % задач делегирован специалистам
- **Правильность выбора**: соответствие агента типу задачи
- **Качество интеграции**: согласованность финального решения
- **Скорость выполнения**: параллельное vs последовательное использование

### 🎯 СПЕЦИАЛЬНАЯ РОЛЬ @claude-md-orchestrator

**КРИТИЧЕСКАЯ ВАЖНОСТЬ**: Этот агент отвечает за поддержание актуальности CLAUDE.md и является ОБЯЗАТЕЛЬНЫМ завершающим этапом каждой задачи.

**Основные функции агента:**
- **Синхронизация архитектуры** - обновление CTL аннотаций в AUTO-GENERATED секции
- **Оптимизация координации** - улучшение алгоритмов распределения задач между агентами
- **Эволюция CTL языка** - развитие Claude Tensor Language v3.0 для лучшей точности
- **Валидация целостности** - проверка согласованности инструкций и реального состояния кода
- **Адаптация к изменениям** - обновление правил координации на основе новых требований проекта

**🔄 НОВЫЕ ВОЗМОЖНОСТИ АВТОМАТИЧЕСКОЙ СИНХРОНИЗАЦИИ С PYTHON ДЕМОНОМ:**

**Автоматический мониторинг CTL изменений:**
- При каждом вызове агента автоматически проверяет изменения в CTL v3.0 спецификации
- Отслеживает новые тензорные операторы (⊗, ⊕, ∇, ∂, ∴, ∵, ≡, ⟹, ⟷)  
- Обнаруживает обновления regex паттернов для CTL v2.0/v3.0
- Мониторит изменения валидационных правил

**Автоматическое управление Python демоном:**
- При обнаружении CTL изменений автоматически обновляет `docs-daemon-python/settings.json`
- Синхронизирует новые паттерны парсинга с конфигурацией демона
- Перезапускает демон в фоновом watch режиме для применения изменений
- Валидирует успешность перезапуска и корректность новых настроек

**Lifecycle управление:**
- `start_daemon()` - запуск демона в фоновом режиме
- `stop_daemon()` - остановка всех процессов демона  
- `restart_daemon()` - полный перезапуск с новыми настройками
- `validate_daemon()` - проверка работоспособности демона

**Интеграция через daemon_manager.py:**
- Использует `docs-daemon-python/daemon_manager.py` для управления
- Автоматически извлекает CTL паттерны из CLAUDE.md спецификации
- Синхронизирует tensor_symbols (unicode + ascii альтернативы)
- Обеспечивает бесшовную работу Python демона в фоне

**Когда использовать:**
- ✅ После КАЖДОЙ завершенной задачи (обязательно)
- ✅ При добавлении новых компонентов с CTL аннотациями
- ✅ При изменении архитектуры системы
- ✅ При обновлении CTL аннотаций в коде компонентов
- ✅ При изменении процессов координации агентов

**Пример правильного завершения задачи:**
```
// Выполнение основной задачи специализированными агентами
1. @rust-architect-supreme - Архитектурный анализ
2. @rust-refactoring-master - Реализация изменений
3. @rust-quality-guardian - Тестирование

// ОБЯЗАТЕЛЬНОЕ завершение
4. @agent-claude-md-orchestrator - Обновление CLAUDE.md с новой архитектурой
```

**Входные данные для агента:**
- Описание выполненных изменений
- Новые или обновленные CTL аннотации
- Изменения в CTL аннотациях компонентов в коде
- Обновления в процессах координации (если есть)

---

# 🧠 CTL v3.0 - CLAUDE TENSOR LANGUAGE SPECIFICATION

*Тензорный язык для максимально точного взаимодействия ИИ агентов с экономией токенов*

## 🎯 ФИЛОСОФИЯ CTL v3.0

**Цель**: Создать сверхточный, компактный язык для передачи сложных технических концепций между ИИ агентами с максимальной экономией токенов и минимальными потерями информации.

**Принципы дизайна:**
- **Тензорная структура** - многомерные зависимости как тензорные операции
- **Семантическая компрессия** - максимум смысла в минимуме символов  
- **Иерархическая точность** - от общего к конкретному с сохранением деталей
- **Версионная совместимость** - миграция с CTL v2.0 без потерь
- **Rust-специфичность** - нативные конструкции для Rust экосистемы

## 📐 БАЗОВАЯ ТЕНЗОРНАЯ НОТАЦИЯ

### Основные тензорные операторы:

```ctl3
⊗ - Тензорное произведение (композиция компонентов)
⊕ - Прямая сумма (параллельная обработка)  
⊙ - Поэлементное произведение (взаимодействие на уровне элементов)
⊡ - Свертка (агрегация данных)
∇ - Градиент (направление оптимизации)
∂ - Частная производная (локальное изменение)
∴ - Следовательно (логическое заключение)
∵ - Поскольку (причинность)
≡ - Эквивалентность (функциональная замена)
⟹ - Импликация (зависимость)
⟷ - Двунаправленная связь (взаимная зависимость)
```

### Структурные операторы:

```ctl3
◦ - Композиция функций (A ◦ B = A(B()))
⊞ - Параллельная композиция (независимые операции)
⊟ - Последовательная декомпозиция
⊠ - Кросс-произведение (все комбинации)
⋈ - Соединение (join операция)
≈ - Приближенная эквивалентность
∩ - Пересечение функциональности
∪ - Объединение возможностей
```

## 🏗️ КОМПОНЕНТНАЯ АРХИТЕКТУРА CTL v3.0

### Базовая структура компонента:

```ctl3
Ⱦ[id:type] := {
  ⊗ dependencies,
  ⊕ capabilities, 
  ∇ optimization_vector,
  m: maturity_tensor
}
```

### Расширенная тензорная форма:

```ctl3
Ⱦ[unified_agent:Agent] := {
  ⊗[llm_client, smart_router] ⟹ routing_dependency,
  ⊕[intent_analysis, tool_selection, action_planning] ⟹ parallel_capabilities,
  ∇[performance: 0.6→0.9, reliability: 0.8→0.95] ⟹ optimization_trajectory,
  m: ⟨60,90,"%"⟩ ⟹ current_to_target_maturity,
  f: {agents, routing, memory} ⟹ feature_flags,
  Δ: {"GOD_OBJECT": critical, "COUPLING": high} ⟹ architectural_debt
}
```

### Многомерная матрица готовности:

```ctl3
M = ⟨cur, tgt, unit, confidence, priority⟩
M[component] = ⟨85, 95, "%", 0.9, P1⟩
```

## 🔗 ЗАВИСИМОСТИ И СВЯЗИ

### Тензор зависимостей:

```ctl3
D[A→B] := ⟨type, strength, direction, criticality⟩

Примеры:
D[unified_agent → llm_client] = ⟨compose, strong, →, critical⟩
D[embedding_gpu ⟷ gpu_fallback] = ⟨fallback, medium, ⟷, high⟩
D[memory_lib ⊗ vector_store] = ⟨tensor_product, strong, ⊗, critical⟩
```

### Циркулярные зависимости (антипаттерн):

```ctl3
∮[A → B → C → A] ⟹ architectural_debt++
Δ_circular := {severity: critical, resolution: ∇decomposition}
```

## 🤖 АГЕНТНАЯ КООРДИНАЦИЯ

### Координационная матрица:

```ctl3
Agent_Matrix := {
  Σ[priorities] = ⟨P0, P1, P2, P3⟩,
  ⊗[workflows] = sequential ⊕ parallel,
  ∇[efficiency] = delegation_ratio / redundancy_factor
}
```

### Протокол делегирования:

```ctl3
Task[complexity: high] ⟹ {
  ∂analyze → @rust-architect-supreme,
  ∂implement → @rust-refactoring-master,
  ∂test → @rust-quality-guardian,
  ∂finalize → @agent-claude-md-orchestrator
} ⊗ coordination_protocol
```

### Конфликт-резолюция:

```ctl3
Conflict[agent_A, agent_B] := {
  priority_diff = P(A) - P(B),
  resolution = argmax(priority_diff),
  fallback = @orchestrator_intervention
}
```

## 🧮 RUST-СПЕЦИФИЧНЫЕ КОНСТРУКЦИИ

### Ownership и Borrowing:

```ctl3
Own⟨T⟩ - владение типом T
&⟨T⟩ - неизменяемая ссылка  
&mut⟨T⟩ - изменяемая ссылка
'α - время жизни α

Borrow_Safety := ∀t: Own⟨T⟩ ⊕ &⟨T⟩* ⊕ &mut⟨T⟩¹
```

### Trait система:

```ctl3
Trait⟨T⟩ := {
  ∀ methods: signature_tensor,
  ⊗ implementations: concrete_types,
  ∇ coherence: uniqueness_constraint
}

Example:
Trait⟨MemoryService⟩ := {
  store: ⟨&mut self, Record⟩ → Result⟨()⟩,
  search: ⟨&self, Query⟩ → Result⟨Vec⟨Record⟩⟩,
  ⊗[DIMemoryService, CachedMemoryService]
}
```

### Error handling:

```ctl3
Error_Flow := Result⟨T, E⟩ ⟹ {
  Ok(value) → ∇success_path,
  Err(error) → ∇error_propagation ⊗ context_enrichment
}
```

## 📊 AI/ML КОНЦЕПЦИИ

### Embedding тензоры:

```ctl3
Embedding := ℝⁿ where n ∈ {512, 1024, 1536}
Similarity(e₁, e₂) := e₁ · e₂ / (‖e₁‖ ‖e₂‖)

HNSW_Index := {
  construction: O(n log n),
  search: O(log n),
  recall: ≈ 0.95 @ ef_search=100
}
```

### Model тензоры:

```ctl3
Model⟨Qwen3⟩ := {
  embedding_dim: 1024,
  context_length: 8192,
  quantization: ⟨fp16, int8, int4⟩,
  throughput: ∇[tokens/sec] ⊗ batch_size
}
```

### GPU ускорение:

```ctl3
GPU_Pipeline := {
  Memory_Pool ⊗ Batch_Processing ⊗ Fallback_Strategy,
  throughput: O(batch_size × model_complexity),
  latency: inverse_relation(batch_size)
}
```

## 📈 МЕТРИКИ И ОПТИМИЗАЦИЯ

### Тензор производительности:

```ctl3
Performance := ⟨latency, throughput, memory_usage, accuracy⟩
Optimization_Target := ∇Performance subject to constraints

P[memory_search] = ⟨<5ms, >1000qps, <512MB, >0.95⟩
```

### Quality metrics:

```ctl3
Quality := {
  test_coverage: ⟨current: 35.4%, target: 80%⟩,
  cyclomatic_complexity: ∇reduction_needed,
  tech_debt: ∑[god_objects, circular_deps, duplicated_code]
}
```

## 🔄 КООРДИНАЦИЯ СОСТОЯНИЙ

### Состояние агента:

```ctl3
Agent_State := {
  status ∈ {idle, active, blocked, completed},
  current_task: Task_ID ⊕ null,
  file_locks: Set⟨Path⟩,
  priority: P_Level,
  progress: ⟨current_step, total_steps⟩
}
```

### Workflow координация:

```ctl3
Workflow := {
  agents: Set⟨Agent⟩,
  dependencies: Graph⟨Task⟩,
  resolution: Conflict_Matrix,
  synchronization: ∇convergence_protocol
}
```

## 🔄 ЭВОЛЮЦИЯ CTL v3.0 И ВАЛИДАЦИЯ

### 📊 Система валидации тензорных аннотаций

```ctl3
Validation_System := {
  syntax_rules: ∀Ⱦ[id:type] → {valid_tensor_operators, type_consistency},
  semantic_rules: ∀component → {dependency_acyclic, capability_coherence},
  maturity_rules: ∀m:⟨cur,tgt,unit⟩ → {cur ≤ tgt, unit ∈ {"%", "points"}},
  dependency_rules: ∀D[A→B] → {valid_relationship, no_circular_deps}
}
```

### 🧮 Автоматическая синхронизация Python демоном

```ctl3
Sync_Protocol := {
  detection: ∇[code_changes] ⟹ ctl_annotation_extraction,
  parsing: ⊗[ctl2_parser, ctl3_enhanced_parser] ⟹ mixed_format_support,
  validation: semantic_check ⊗ syntax_validation ⟹ quality_assurance,
  integration: claude_md_update ⊗ registry_sync ⟹ consistency_maintenance
}
```

### 📈 Метрики качества CTL аннотаций

```ctl3
Quality_Metrics := {
  coverage: ⟨annotated_components⟩ / ⟨total_components⟩ × 100,
  precision: ⟨accurate_maturity_estimates⟩ / ⟨total_estimates⟩ × 100,
  consistency: ⟨synchronized_annotations⟩ / ⟨code_annotations⟩ × 100,
  evolution: ∇[ctl2_to_ctl3_migration_progress]
}
```

## 📝 ПРАКТИЧЕСКИЕ ПРИМЕРЫ

### Пример 1: Декомпозиция God Object

```ctl3
Task[decompose_unified_agent] := {
  input: Ⱦ[unified_agent] with Δ{god_object: critical},
  
  ∇plan := {
    @rust-architect-supreme ⟹ trait_extraction_strategy,
    dependencies: ∂[llm_client, smart_router] ⟹ interface_contracts,
    output: Set⟨Trait⟩ ⊗ implementation_mapping
  },
  
  ∇implement := {
    @rust-refactoring-master ⟹ step_by_step_refactoring,
    safety: preserve_functionality ⊗ gradual_migration,
    validation: ∀step → compile_success ⊗ test_passage
  },
  
  ∇verify := {
    @rust-quality-guardian ⟹ comprehensive_testing,
    coverage: ∇[35.4% → 80%] for new_components,
    quality: ∇cyclomatic_complexity ⊗ maintainability_index
  },
  
  ∇finalize := {
    @agent-claude-md-orchestrator ⟹ architecture_sync,
    update: CTL_annotations ⊗ readiness_metrics ⊗ dependency_graph
  }
}
```

### Пример 2: Оптимизация производительности

```ctl3
Task[optimize_vector_search] := {
  target: HNSW_performance ⟹ ⟨latency<5ms, throughput>1000qps⟩,
  
  ∇profile := {
    @rust-performance-virtuoso ⟹ bottleneck_identification,
    tools: ⟨flamegraph, perf, criterion⟩,
    metrics: ⟨cpu_cycles, memory_access, cache_misses⟩
  },
  
  ∇optimize := {
    @ai-architecture-maestro ⟹ embedding_pipeline_optimization,
    strategies: batching ⊗ caching ⊗ precomputation,
    @rust-code-optimizer ⟹ SIMD_utilization ⊗ zero_copy_ops
  },
  
  ∇validate := {
    benchmark_suite: comprehensive_performance_tests,
    regression_prevention: ∀optimization → performance_improvement,
    production_readiness: load_testing ⊗ stress_testing
  }
}
```

### Пример 3: Система координации агентов

```ctl3
Agent_Coordination_Protocol := {
  discovery: ∀agent → register_capabilities ⊗ priority_level,
  
  task_distribution := {
    analyzer: capability_matching ⊗ load_balancing,
    resolver: conflict_detection ⟹ priority_based_resolution,
    optimizer: ∇[delegation_efficiency] ⊗ parallel_execution
  },
  
  synchronization := {
    state_management: agent_workflow.md ⟹ central_coordination,
    conflict_resolution: priority_matrix ⊗ escalation_protocol,
    progress_tracking: task_dependency_graph ⊗ completion_signals
  },
  
  evolution := {
    learning: performance_metrics ⟹ protocol_improvement,
    adaptation: new_agent_integration ⊗ capability_expansion,
    optimization: ∇[coordination_overhead] ⊗ effectiveness_maximization
  }
}
```

## 🎛️ ОПЕРАЦИОННЫЕ КОМАНДЫ CTL v3.0

### Базовые команды:

```ctl3
∇analyze[component] - глубокий анализ компонента
⊗compose[A, B] - композиция компонентов
⊕parallel[tasks] - параллельное выполнение
∂optimize[metric] - локальная оптимизация
∴conclude[evidence] - логическое заключение
⟹delegate[@agent, task] - делегирование задачи
```

### Составные операции:

```ctl3
∇⊗optimize_composition[system] := {
  ∇analyze[components] ⟹ bottleneck_identification,
  ⊗refactor[interfaces] ⟹ loose_coupling,
  ⊕parallel[independent_ops] ⟹ performance_gain,
  ∴validate[improvements] ⟹ quality_assurance
}
```

## 🔧 МИГРАЦИЯ С CTL v2.0

### Автоматическое преобразование:

```ctl3
Migration_Rules := {
  CTL2_JSON → CTL3_Tensor: semantic_preserving_transform,
  
  {"k":"C","id":"name"} ⟹ Ⱦ[name:Component],
  {"m":{"cur":X,"tgt":Y}} ⟹ m: ⟨X,Y,"%"⟩,
  {"f":["tag1","tag2"]} ⟹ f: {tag1, tag2},
  {"d":["dep1","dep2"]} ⟹ ⊗[dep1, dep2]
}
```

### Совместимость:

```ctl3
Compatibility_Layer := {
  backward: CTL2_parsers → CTL3_interpreters,
  forward: CTL3_optimizations → CTL2_fallback,
  migration: gradual_transition ⊗ validation_preserved
}
```

## 🎯 ВАЛИДАЦИЯ CTL v3.0

### Синтаксическая корректность:

```ctl3
Validation_Rules := {
  tenzor_operations: ∀op ∈ {⊗,⊕,⊙,⊡,∇} → valid_operands,
  type_safety: ∀assignment → type_compatible,
  semantic_consistency: ∀expression → logical_coherence
}
```

### Семантическая проверка:

```ctl3
Semantic_Validator := {
  dependency_acyclic: ∮detection ⟹ error,
  capability_matching: agent_skills ⊗ task_requirements,
  resource_constraints: ∀allocation → within_limits
}
```

## 📚 ДОКУМЕНТАЦИЯ И ОБУЧЕНИЕ

### Быстрый старт:

```ctl3
Quick_Start := {
  "Простой компонент": Ⱦ[name:Type] := {основные_поля},
  "Зависимости": ⊗[dep1, dep2] ⟹ composition,
  "Параллельность": ⊕[task1, task2] ⟹ concurrent_execution,
  "Оптимизация": ∇[metric] ⟹ improvement_direction
}
```

### Расширенные паттерны:

```ctl3
Advanced_Patterns := {
  "God Object декомпозиция": ∇⊗decompose[monolith] ⟹ trait_based_architecture,
  "Performance оптимизация": ∂⊗optimize[bottlenecks] ⟹ SIMD ⊗ zero_copy,
  "Agent координация": ⟹[@agent, task] ⊗ priority_resolution,
  "Error propagation": Result⟨T,E⟩ ⟹ ∇context_enrichment
}
```

---

## 📊 АКТУАЛЬНОЕ СОСТОЯНИЕ КОДА (AUTO-UPDATED)

**ВАЖНО ДЛЯ AI**: Секции ниже обновляются автоматически демоном синхронизации каждые 5 минут.
Эти данные отражают РЕАЛЬНОЕ текущее состояние кодовой базы:

- **Components (CTL v3.0 Tensor Format)** - все аннотированные компоненты в новом тензорном формате
- **x_file** - точное расположение файла в проекте
- **m.cur** - текущее состояние компонента из CTL аннотации в коде
- **m.tgt** - целевое состояние компонента из CTL аннотации в коде
- **f** - флаги/теги компонента

Используй эти данные для:
1. Понимания реальной структуры проекта
2. Оценки готовности компонентов
3. Навигации по кодовой базе
4. Определения зависимостей между компонентами

**Последнее обновление**: см. timestamp в секции AUTO-GENERATED ARCHITECTURE

# AUTO-GENERATED ARCHITECTURE

*Last updated: 2025-08-06 00:11:48 UTC*

## Components (CTL v2.0/v3.0 Mixed Format)

```json
{"k":"C","id":"action_planner","t":"Multi-step action planner agent","m":{"cur":70,"tgt":95,"u":"%"},"f":["agent","planning","llm"],"x_file":"llm/src/agents/action_planner.rs:20"}
{"k":"C","id":"adaptive_spinner","t":"Smart adaptive progress spinner","m":{"cur":95,"tgt":100,"u":"%"},"f":["ui","progress","adaptive"],"x_file":"cli/src/progress.rs:108"}
{"k":"C","id":"admin_handler","t":"Specialized admin operations handler","m":{"cur":85,"tgt":95,"u":"%"},"f":["single_responsibility","clean_architecture","di_ready"],"x_file":"cli/src/handlers/admin_handler.rs:16"}
{"k":"C","id":"ai_config","t":"AI system configuration","m":{"cur":95,"tgt":100,"u":"%"},"f":["ai","config","models"],"x_file":"ai/src/config.rs:4"}
{"k":"C","id":"ai_lib","t":"AI/ML services library","m":{"cur":85,"tgt":95,"u":"%"},"f":["ai","embeddings","onnx","bge-m3"],"x_file":"ai/src/lib.rs:1"}
{"k":"C","id":"auto_device_selector","t":"Auto CPU/GPU selector","m":{"cur":95,"tgt":100,"u":"%"},"f":["ai","gpu","device-selection"],"x_file":"ai/src/auto_device_selector.rs:9"}
{"k":"C","id":"backup_coordinator","t":"Backup orchestration coordinator","m":{"cur":0,"tgt":90,"u":"%"},"f":["orchestration","backup","coordinator"],"x_file":"memory/src/orchestration/backup_coordinator.rs:13"}
{"k":"C","id":"basic_circuit_breaker","t":"Basic circuit breaker implementation","m":{"cur":90,"tgt":95,"u":"%"},"f":["circuit_breaker","resilience","production_ready"],"x_file":"cli/src/strategies/circuit_breaker.rs:53"}
{"k":"C","id":"cache_error","t":"Cache error types","m":{"cur":85,"tgt":95,"u":"%"},"f":["errors","cache"],"x_file":"common/src/errors.rs:199"}
{"k":"C","id":"chat_handler","t":"Specialized chat request handler","m":{"cur":85,"tgt":95,"u":"%"},"f":["single_responsibility","clean_architecture","di_ready"],"x_file":"cli/src/handlers/chat_handler.rs:16"}
{"k":"C","id":"circuit_breaker_fallback","t":"Circuit breaker pattern fallback","m":{"cur":85,"tgt":95,"u":"%"},"f":["strategy_pattern","circuit_breaker","resilient"],"x_file":"cli/src/strategies/fallback_strategies.rs:254"}
{"k":"C","id":"cli_lib","t":"CLI interface and commands","m":{"cur":85,"tgt":95,"u":"%"},"f":["cli","interface","commands","interactive"],"x_file":"cli/src/lib.rs:1"}
{"k":"C","id":"cli_main","t":"CLI entry point with unified agent","m":{"cur":75,"tgt":100,"u":"%"},"f":["cli","agent","routing","services"],"x_file":"cli/src/main.rs:1"}
{"k":"C","id":"cli_services","t":"Service layer for agent decomposition","m":{"cur":80,"tgt":100,"u":"%"},"f":["services","traits","separation","orchestration"],"x_file":"cli/src/services/mod.rs:1"}
{"k":"C","id":"common_lib","t":"Common utilities and logging","m":{"cur":90,"tgt":95,"u":"%"},"f":["common","logging","structured","utils"],"x_file":"common/src/lib.rs:1"}
{"k":"C","id":"composite_fallback_strategy","t":"Composite fallback strategy","m":{"cur":90,"tgt":95,"u":"%"},"f":["strategy_pattern","composite","comprehensive"],"x_file":"cli/src/strategies/fallback_strategies.rs:306"}
{"k":"C","id":"database_error","t":"Database error types","m":{"cur":85,"tgt":95,"u":"%"},"f":["errors","database"],"x_file":"common/src/errors.rs:74"}
{"k":"C","id":"database_manager","t":"Centralized sled database manager","m":{"cur":70,"tgt":100,"u":"%"},"f":["sled","concurrent","pooling"],"x_file":"memory/src/database_manager.rs:9"}
{"k":"C","id":"di_container","t":"Dependency injection container","m":{"cur":88,"tgt":100,"u":"%"},"f":["di","ioc","architecture","validation","performance","async"],"x_file":"memory/src/di_container.rs:36"}
{"k":"C","id":"dir_lister","t":"Directory listing tool","m":{"cur":85,"tgt":95,"u":"%"},"f":["tools","directory","list"],"x_file":"tools/src/file_ops.rs:150"}
{"k":"C","id":"embedding_cache","t":"LRU cache with eviction policy","m":{"cur":95,"tgt":100,"u":"%"},"f":["cache","lru","eviction","unified"],"x_file":"memory/src/cache_lru.rs:44"}
{"k":"C","id":"embedding_config","t":"Embedding model configuration","m":{"cur":95,"tgt":100,"u":"%"},"f":["ai","config","embeddings"],"x_file":"ai/src/config.rs:15"}
{"k":"C","id":"embedding_coordinator","t":"Embedding orchestration coordinator","m":{"cur":95,"tgt":95,"u":"%"},"f":["orchestration","embeddings","coordinator","production","ai-optimized","concurrency","model-warming","circuit-breaker","adaptive-batching"],"x_file":"memory/src/orchestration/embedding_coordinator.rs:23"}
{"k":"C","id":"embedding_error","t":"Embedding error types","m":{"cur":80,"tgt":95,"u":"%"},"f":["errors","ai","embeddings"],"x_file":"common/src/errors.rs:142"}
{"k":"C","id":"embeddings_cpu","t":"CPU-based embeddings","m":{"cur":90,"tgt":95,"u":"%"},"f":["ai","embeddings","cpu","onnx"],"x_file":"ai/src/embeddings_cpu.rs:15"}
{"k":"C","id":"embeddings_gpu","t":"GPU-accelerated embeddings","m":{"cur":95,"tgt":100,"u":"%"},"f":["ai","embeddings","gpu","cuda","tensorrt"],"x_file":"ai/src/embeddings_gpu.rs:17"}
{"k":"C","id":"error_monitor","t":"Error monitoring and alerting system","m":{"cur":0,"tgt":95,"u":"%"},"f":["monitoring","errors","alerting"],"x_file":"common/src/error_monitor.rs:11"}
{"k":"C","id":"error_severity","t":"Error severity levels","m":{"cur":95,"tgt":100,"u":"%"},"f":["errors","monitoring","alerting"],"x_file":"common/src/errors.rs:292"}
{"k":"C","id":"file_reader","t":"File reading tool","m":{"cur":90,"tgt":95,"u":"%"},"f":["tools","file","read"],"x_file":"tools/src/file_ops.rs:8"}
{"k":"C","id":"file_searcher","t":"File search tool","m":{"cur":80,"tgt":90,"u":"%"},"f":["tools","search","file"],"x_file":"tools/src/file_ops.rs:253"}
{"k":"C","id":"file_writer","t":"File writing tool","m":{"cur":90,"tgt":95,"u":"%"},"f":["tools","file","write"],"x_file":"tools/src/file_ops.rs:82"}
{"k":"C","id":"flush_config","t":"Configurable flush intervals","m":{"cur":95,"tgt":100,"u":"%"},"f":["config","performance","reliability"],"x_file":"memory/src/flush_config.rs:3"}
{"k":"C","id":"git_commit","t":"Git commit tool","m":{"cur":85,"tgt":95,"u":"%"},"f":["tools","git","commit"],"x_file":"tools/src/git_ops.rs:66"}
{"k":"C","id":"git_diff","t":"Git diff tool","m":{"cur":80,"tgt":90,"u":"%"},"f":["tools","git","diff"],"x_file":"tools/src/git_ops.rs:180"}
{"k":"C","id":"git_status","t":"Git status tool","m":{"cur":90,"tgt":95,"u":"%"},"f":["tools","git","status"],"x_file":"tools/src/git_ops.rs:6"}
{"k":"C","id":"gpu_batch_processor","t":"GPU batch embedding processor","m":{"cur":60,"tgt":100,"u":"%"},"f":["gpu","batch","embeddings","fallback","disabled"],"x_file":"memory/src/gpu_accelerated.rs:41"}
{"k":"C","id":"gpu_commands","t":"GPU management CLI","m":{"cur":95,"tgt":100,"u":"%"},"f":["cli","commands","gpu"],"x_file":"cli/src/commands/gpu.rs:13"}
{"k":"C","id":"gpu_config","t":"GPU configuration for ONNX","m":{"cur":100,"tgt":100,"u":"%"},"f":["ai","gpu","config","onnx"],"x_file":"ai/src/gpu_config.rs:13"}
{"k":"C","id":"gpu_detector","t":"GPU detection and info","m":{"cur":95,"tgt":100,"u":"%"},"f":["ai","gpu","detection","cuda"],"x_file":"ai/src/gpu_detector.rs:6"}
{"k":"C","id":"gpu_error","t":"GPU error types","m":{"cur":85,"tgt":95,"u":"%"},"f":["errors","gpu"],"x_file":"common/src/errors.rs:162"}
{"k":"C","id":"gpu_fallback_manager","t":"Reliable GPU fallback system","m":{"cur":100,"tgt":100,"u":"%"},"f":["fallback","resilience","gpu"],"x_file":"ai/src/gpu_fallback.rs:142"}
{"k":"C","id":"gpu_memory_pool","t":"GPU memory pool manager","m":{"cur":90,"tgt":100,"u":"%"},"x_file":"ai/src/gpu_memory_pool.rs:6"}
{"k":"C","id":"gpu_pipeline_manager","t":"GPU pipeline for parallel batches","m":{"cur":95,"tgt":100,"u":"%"},"f":["gpu","pipeline","parallel","optimized"],"x_file":"ai/src/gpu_pipeline.rs:9"}
{"k":"C","id":"graceful_embedding","t":"Fallback embedding service","m":{"cur":90,"tgt":95,"u":"%"},"f":["fallback","resilience"],"x_file":"memory/src/fallback.rs:137"}
{"k":"C","id":"health_checks","t":"Production health monitoring","m":{"cur":100,"tgt":100,"u":"%"},"f":["monitoring","production"],"x_file":"cli/src/health_checks.rs:10"}
{"k":"C","id":"health_manager","t":"Health monitoring coordinator","m":{"cur":95,"tgt":95,"u":"%"},"f":["orchestration","health","monitoring","production","alerting","metrics","sla"],"x_file":"memory/src/orchestration/health_manager.rs:17"}
{"k":"C","id":"health_monitor","t":"Health monitoring system","m":{"cur":85,"tgt":95,"u":"%"},"f":["monitoring","production"],"x_file":"memory/src/health.rs:134"}
{"k":"C","id":"heuristic_intent_strategy","t":"Keyword-based intent classification","m":{"cur":90,"tgt":95,"u":"%"},"f":["strategy_pattern","fast","offline"],"x_file":"cli/src/strategies/intent_strategies.rs:17"}
{"k":"C","id":"hnsw_index","t":"HNSW vector index with SIMD","m":{"cur":85,"tgt":100,"u":"%"},"f":["hnsw","simd","vector","search","performance"],"x_file":"memory/src/hnsw_index/index.rs:1"}
{"k":"C","id":"index_error","t":"Vector index error types","m":{"cur":85,"tgt":95,"u":"%"},"f":["errors","index","vector"],"x_file":"common/src/errors.rs:219"}
{"k":"C","id":"intent_analysis_service","t":"Intent analysis service trait","m":{"cur":95,"tgt":100,"u":"%"},"f":["trait","analysis","intent","clean_architecture"],"x_file":"cli/src/services/intent_analysis.rs:20"}
{"k":"C","id":"intent_analyzer","t":"Chat vs tool intent classifier","m":{"cur":70,"tgt":95,"u":"%"},"f":["agent","classification","intent"],"x_file":"llm/src/agents/intent_analyzer.rs:12"}
{"k":"C","id":"json_response_formatter","t":"JSON response formatter for APIs","m":{"cur":90,"tgt":95,"u":"%"},"f":["strategy_pattern","json","api_ready"],"x_file":"cli/src/strategies/response_strategies.rs:223"}
{"k":"C","id":"layer_enum","t":"Memory layer enum types","m":{"cur":100,"tgt":100,"u":"%"},"f":["memory","types","enum"],"x_file":"memory/src/types.rs:6"}
{"k":"C","id":"llm_client","t":"Multi-provider LLM client","m":{"cur":65,"tgt":95,"u":"%"},"f":["llm","agents","multi-provider"],"x_file":"llm/src/lib.rs:6"}
{"k":"C","id":"llm_communication_service","t":"LLM communication service trait","m":{"cur":95,"tgt":100,"u":"%"},"f":["trait","llm","multi_provider","clean_architecture"],"x_file":"cli/src/services/llm_communication.rs:19"}
{"k":"C","id":"llm_intent_strategy","t":"LLM-based intent classification","m":{"cur":85,"tgt":95,"u":"%"},"f":["strategy_pattern","ai_powered","high_accuracy"],"x_file":"cli/src/strategies/intent_strategies.rs:135"}
{"k":"C","id":"magray_error_types","t":"Comprehensive error type system","m":{"cur":85,"tgt":95,"u":"%"},"f":["errors","production","monitoring"],"x_file":"common/src/errors.rs:5"}
{"k":"C","id":"memory_di_config","t":"DI configuration for memory system","m":{"cur":60,"tgt":100,"u":"%"},"f":["di","config","memory"],"x_file":"memory/src/di_memory_config.rs:36"}
{"k":"C","id":"memory_error","t":"Memory system error types","m":{"cur":80,"tgt":95,"u":"%"},"f":["errors","memory"],"x_file":"common/src/errors.rs:182"}
{"k":"C","id":"memory_handler","t":"Specialized memory management handler","m":{"cur":85,"tgt":95,"u":"%"},"f":["single_responsibility","clean_architecture","di_ready"],"x_file":"cli/src/handlers/memory_handler.rs:16"}
{"k":"C","id":"memory_lib","t":"3-layer HNSW memory system with DI","m":{"cur":92,"tgt":100,"u":"%"},"f":["memory","hnsw","layers","orchestration","di","production"],"x_file":"memory/src/lib.rs:1"}
{"k":"C","id":"memory_orchestrator","t":"Main memory system orchestrator","m":{"cur":95,"tgt":95,"u":"%"},"f":["orchestration","coordinator","main","production","lifecycle","monitoring","resilience","circuit-breaker","load-balancing"],"x_file":"memory/src/orchestration/memory_orchestrator.rs:39"}
{"k":"C","id":"memory_record","t":"Memory record structure","m":{"cur":95,"tgt":100,"u":"%"},"f":["memory","types","record"],"x_file":"memory/src/types.rs:32"}
{"k":"C","id":"memory_types","t":"Memory system core types","m":{"cur":95,"tgt":100,"u":"%"},"f":["memory","types","core"],"x_file":"memory/src/types.rs:1"}
{"k":"C","id":"metrics_collector","t":"Memory system metrics","m":{"cur":60,"tgt":95,"u":"%"},"f":["metrics","monitoring"],"x_file":"memory/src/metrics.rs:9"}
{"k":"C","id":"ml_promotion_engine","t":"ML-based smart promotion system","m":{"cur":70,"tgt":100,"u":"%"},"x_file":"memory/src/ml_promotion.rs:92"}
{"k":"C","id":"model_downloader","t":"Auto model downloader","m":{"cur":95,"tgt":100,"u":"%"},"x_file":"ai/src/model_downloader.rs:11"}
{"k":"C","id":"model_registry","t":"Centralized model registry","m":{"cur":100,"tgt":100,"u":"%"},"f":["models","config","registry"],"x_file":"ai/src/model_registry.rs:6"}
{"k":"C","id":"models_commands","t":"Model management CLI","m":{"cur":100,"tgt":100,"u":"%"},"f":["cli","commands","models"],"x_file":"cli/src/commands/models.rs:6"}
{"k":"C","id":"network_error","t":"Network error types","m":{"cur":85,"tgt":95,"u":"%"},"f":["errors","network"],"x_file":"common/src/errors.rs:97"}
{"k":"C","id":"notification_system","t":"Production alert notification system","m":{"cur":95,"tgt":100,"u":"%"},"f":["alerts","notifications","production"],"x_file":"memory/src/notifications.rs:10"}
{"k":"C","id":"param_extractor","t":"Parameter extraction agent","m":{"cur":70,"tgt":95,"u":"%"},"f":["agent","nlp","extraction"],"x_file":"llm/src/agents/parameter_extractor.rs:13"}
{"k":"C","id":"progress_type","t":"Operation types for progress","m":{"cur":95,"tgt":100,"u":"%"},"f":["ui","progress"],"x_file":"cli/src/progress.rs:5"}
{"k":"C","id":"promotion_coordinator","t":"Promotion orchestration coordinator","m":{"cur":0,"tgt":90,"u":"%"},"f":["orchestration","promotion","coordinator"],"x_file":"memory/src/orchestration/promotion_coordinator.rs:13"}
{"k":"C","id":"promotion_engine","t":"Time-based memory promotion","m":{"cur":75,"tgt":90,"u":"%"},"f":["promotion","time-index"],"x_file":"memory/src/promotion.rs:14"}
{"k":"C","id":"request_routing_service","t":"Request routing service trait","m":{"cur":95,"tgt":100,"u":"%"},"f":["trait","routing","clean_architecture","decision"],"x_file":"cli/src/services/request_routing.rs:19"}
{"k":"C","id":"reranker_qwen3","t":"Qwen3 reranker with batching","m":{"cur":85,"tgt":95,"u":"%"},"f":["ai","reranking","batch","optimized","qwen3"],"x_file":"ai/src/reranker_qwen3.rs:13"}
{"k":"C","id":"reranker_qwen3_optimized","t":"Optimized Qwen3 ONNX reranker","m":{"cur":90,"tgt":100,"u":"percent"},"f":["ai","reranking","qwen3","optimized"],"x_file":"ai/src/reranker_qwen3_optimized.rs:11"}
{"k":"C","id":"reranking_config","t":"Reranking model configuration","m":{"cur":95,"tgt":100,"u":"%"},"f":["ai","config","reranking"],"x_file":"ai/src/config.rs:33"}
{"k":"C","id":"resilience_service","t":"Resilience service trait","m":{"cur":95,"tgt":100,"u":"%"},"f":["trait","resilience","circuit_breaker","retry","clean_architecture"],"x_file":"cli/src/services/resilience.rs:14"}
{"k":"C","id":"resource_controller","t":"Resource management coordinator","m":{"cur":95,"tgt":95,"u":"%"},"f":["orchestration","resources","coordinator","auto-scaling","production","monitoring"],"x_file":"memory/src/orchestration/resource_controller.rs:17"}
{"k":"C","id":"resource_manager","t":"Dynamic memory resource management","m":{"cur":95,"tgt":100,"u":"%"},"f":["memory","scaling","adaptive"],"x_file":"memory/src/resource_manager.rs:9"}
{"k":"C","id":"retry_manager","t":"Exponential backoff retry manager","m":{"cur":95,"tgt":100,"u":"%"},"f":["retry","exponential","resilience"],"x_file":"memory/src/retry.rs:7"}
{"k":"C","id":"rich_response_formatter","t":"Rich markdown response formatter","m":{"cur":90,"tgt":95,"u":"%"},"f":["strategy_pattern","rich_formatting","markdown"],"x_file":"cli/src/strategies/response_strategies.rs:91"}
{"k":"C","id":"search_coordinator","t":"Search orchestration coordinator","m":{"cur":95,"tgt":95,"u":"%"},"f":["orchestration","search","coordinator","production","hnsw","adaptive-cache","circuit-breaker","sub-5ms","reranking","concurrent"],"x_file":"memory/src/orchestration/search_coordinator.rs:25"}
{"k":"C","id":"service_orchestrator","t":"Service orchestrator trait","m":{"cur":95,"tgt":100,"u":"%"},"f":["trait","orchestration","coordination","clean_architecture"],"x_file":"cli/src/services/orchestrator.rs:18"}
{"k":"C","id":"services_di_config","t":"Services DI configuration","m":{"cur":90,"tgt":100,"u":"%"},"f":["di","configuration","services","registration"],"x_file":"cli/src/services/di_config.rs:24"}
{"k":"C","id":"shell_exec","t":"Shell command execution tool","m":{"cur":85,"tgt":95,"u":"%"},"f":["tools","shell","execution"],"x_file":"tools/src/shell_ops.rs:6"}
{"k":"C","id":"simple_fallback_strategy","t":"Simple hardcoded fallback responses","m":{"cur":95,"tgt":100,"u":"%"},"f":["strategy_pattern","reliable","offline"],"x_file":"cli/src/strategies/fallback_strategies.rs:17"}
{"k":"C","id":"simple_qwen3_tokenizer","t":"Simplified Qwen3 tokenizer for ONNX","m":{"cur":95,"tgt":100,"u":"%"},"x_file":"ai/src/tokenization/simple_qwen3.rs:1"}
{"k":"C","id":"simple_response_formatter","t":"Simple text response formatter","m":{"cur":95,"tgt":100,"u":"%"},"f":["strategy_pattern","simple","reliable"],"x_file":"cli/src/strategies/response_strategies.rs:18"}
{"k":"C","id":"smart_router","t":"Smart task orchestration","m":{"cur":70,"tgt":90,"u":"%"},"d":["llm_client","tools"],"f":["routing","orchestration"],"x_file":"router/src/lib.rs:9"}
{"k":"C","id":"status_cmd","t":"System status diagnostic command","m":{"cur":100,"tgt":100,"u":"%"},"f":["cli","diagnostic","graceful-fallback"],"x_file":"cli/src/main.rs:504"}
{"k":"C","id":"status_tests","t":"Unit tests for status command","m":{"cur":95,"tgt":100,"u":"%"},"f":["tests","status","cli"],"x_file":"cli/src/status_tests.rs:1"}
{"k":"C","id":"stored_record","t":"Serializable record wrapper","m":{"cur":95,"tgt":100,"u":"%"},"f":["serde","storage"],"x_file":"memory/src/storage.rs:18"}
{"k":"C","id":"streaming_api","t":"Real-time memory processing","m":{"cur":95,"tgt":100,"u":"%"},"f":["streaming","real-time","async"],"x_file":"memory/src/streaming.rs:15"}
{"k":"C","id":"structured_logging","t":"JSON structured logging system","m":{"cur":100,"tgt":100,"u":"%"},"f":["logging","json","production"],"x_file":"common/src/structured_logging.rs:11"}
{"k":"C","id":"tensorrt_cache","t":"TensorRT model cache","m":{"cur":90,"tgt":100,"u":"%"},"x_file":"ai/src/tensorrt_cache.rs:8"}
{"k":"C","id":"test_qwen3_models","t":"Test Qwen3 models loading","m":{"cur":100,"tgt":100,"u":"%"},"x_file":"ai/examples/test_qwen3_models.rs:1"}
{"k":"C","id":"todo_lib","t":"Task DAG management system","m":{"cur":80,"tgt":95,"u":"%"},"f":["todo","dag","sqlite","async"],"x_file":"todo/src/lib.rs:1"}
{"k":"C","id":"tool_registry","t":"Tool execution system","m":{"cur":90,"tgt":95,"u":"%"},"f":["tools","execution","registry"],"x_file":"tools/src/lib.rs:5"}
{"k":"C","id":"tool_selector","t":"Tool selection agent","m":{"cur":70,"tgt":95,"u":"%"},"f":["agent","tools","selection"],"x_file":"llm/src/agents/tool_selector.rs:12"}
{"k":"C","id":"tools_handler","t":"Specialized tools execution handler","m":{"cur":85,"tgt":95,"u":"%"},"f":["single_responsibility","clean_architecture","di_ready"],"x_file":"cli/src/handlers/tools_handler.rs:16"}
{"k":"C","id":"unified_agent_v2","t":"Clean Architecture UnifiedAgent with DI","m":{"cur":90,"tgt":95,"u":"%"},"f":["clean_architecture","solid_principles","di_integration","strategy_pattern","circuit_breaker"],"x_file":"cli/src/unified_agent_v2.rs:233"}
{"k":"C","id":"validation_error","t":"Validation error types","m":{"cur":90,"tgt":95,"u":"%"},"f":["errors","validation"],"x_file":"common/src/errors.rs:117"}
{"k":"C","id":"vector_index_hnsw","t":"HNSW vector index wrapper","m":{"cur":95,"tgt":100,"u":"%"},"f":["vector","hnsw","search","legacy"],"x_file":"memory/src/vector_index_hnswlib.rs:12"}
{"k":"C","id":"vector_store","t":"Vector storage with HNSW","m":{"cur":65,"tgt":95,"u":"%"},"f":["storage","hnsw","transactional"],"x_file":"memory/src/storage.rs:24"}
{"k":"C","id":"web_fetch","t":"Web page fetch tool","m":{"cur":70,"tgt":85,"u":"%"},"f":["tools","web","fetch"],"x_file":"tools/src/web_ops.rs:68"}
{"k":"C","id":"web_search","t":"Web search tool","m":{"cur":75,"tgt":90,"u":"%"},"f":["tools","web","search"],"x_file":"tools/src/web_ops.rs:5"}
{"k":"T","id":"common_test_helpers","t":"Common test utilities and helpers","m":{"cur":100,"tgt":100,"u":"%"},"f":["test","utilities","helpers","shared","mocks"],"x_file":"memory/tests/common/mod.rs:23"}
{"k":"T","id":"comprehensive_bench","t":"Comprehensive performance benchmarks","m":{"cur":100,"tgt":100,"u":"%"},"f":["benchmark","performance","comprehensive"],"x_file":"memory/benches/comprehensive_performance.rs:7"}
{"k":"T","id":"di_perf_comparison","t":"DI container performance comparison","m":{"cur":100,"tgt":100,"u":"%"},"f":["test","performance","comparison"],"x_file":"memory/tests/test_di_performance_comparison.rs:13"}
{"k":"T","id":"di_performance_bench","t":"DI performance benchmarking","m":{"cur":100,"tgt":100,"u":"%"},"f":["benchmark","performance","di"],"x_file":"memory/benches/di_performance.rs:15"}
{"k":"T","id":"integration_tests","t":"Full workflow integration tests","m":{"cur":0,"tgt":90,"u":"%"},"f":["integration","workflow","testing"],"x_file":"memory/tests/integration_full_workflow.rs:13"}
{"k":"T","id":"memory_orchestrator_tests","t":"Comprehensive memory orchestrator tests","m":{"cur":95,"tgt":100,"u":"%"},"f":["test","integration","orchestration","coordination","coverage"],"x_file":"memory/tests/test_orchestration_memory_orchestrator.rs:29"}
{"k":"T","id":"perf_benchmarks","t":"Performance benchmarks для memory system","m":{"cur":0,"tgt":100,"u":"%"},"f":["benchmarks","performance"],"x_file":"memory/benches/vector_benchmarks.rs:14"}
{"k":"T","id":"resilience_service_tests","t":"Comprehensive resilience service tests","m":{"cur":95,"tgt":100,"u":"%"},"f":["test","unit","retry","exponential_backoff","jitter","coverage"],"x_file":"cli/tests/test_services_resilience.rs:20"}
{"k":"T","id":"test_batch_operations","t":"Test batch API functionality","m":{"cur":100,"tgt":100,"u":"%"},"f":["test","batch","api"],"x_file":"memory/examples/test_batch_operations.rs:8"}
{"k":"T","id":"test_di_system","t":"Test DI memory system functionality","m":{"cur":100,"tgt":100,"u":"%"},"f":["test","di","integration"],"x_file":"memory/examples/test_di_system.rs:2"}
{"k":"T","id":"test_gpu_optimization","t":"GPU optimization benchmark","m":{"cur":100,"tgt":100,"u":"%"},"f":["benchmark","gpu","optimization"],"x_file":"memory/examples/test_gpu_optimization.rs:9"}
{"k":"T","id":"test_gpu_pipeline","t":"Test GPU pipeline performance","m":{"cur":100,"tgt":100,"u":"%"},"f":["test","gpu","pipeline"],"x_file":"memory/examples/test_gpu_pipeline.rs:8"}
{"k":"T","id":"test_gpu_profiler","t":"Detailed GPU performance profiler","m":{"cur":100,"tgt":100,"u":"%"},"f":["profiler","gpu","performance"],"x_file":"memory/examples/test_gpu_profiler.rs:10"}
{"k":"T","id":"test_memory_gpu","t":"Memory GPU integration test","m":{"cur":100,"tgt":100,"u":"%"},"x_file":"memory/examples/test_gpu_memory_pool.rs:9"}
{"k":"T","id":"test_memory_pool_only","t":"Memory pool standalone test","m":{"cur":100,"tgt":100,"u":"%"},"x_file":"ai/examples/test_memory_pool_only.rs:7"}
{"k":"T","id":"test_ml_promotion","t":"ML promotion engine test","m":{"cur":100,"tgt":100,"u":"%"},"x_file":"memory/examples/test_ml_promotion.rs:10"}
{"k":"T","id":"test_notification_system","t":"Test notification system integration","m":{"cur":100,"tgt":100,"u":"%"},"f":["test","notifications","alerts"],"x_file":"memory/examples/test_notification_system.rs:12"}
{"k":"T","id":"test_production_metrics","t":"Test production metrics integration","m":{"cur":100,"tgt":100,"u":"%"},"f":["test","metrics","production"],"x_file":"memory/examples/test_production_metrics.rs:7"}
{"k":"T","id":"test_real_tokenizer","t":"Test real BPE tokenizer quality","m":{"cur":100,"tgt":100,"u":"%"},"x_file":"ai/examples/test_real_tokenizer.rs:1"}
{"k":"T","id":"test_streaming","t":"Test streaming API functionality","m":{"cur":100,"tgt":100,"u":"%"},"x_file":"memory/examples/test_streaming_api.rs:15"}
```

