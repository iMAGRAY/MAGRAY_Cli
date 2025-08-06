# CLAUDE.md
*AI Agent Instructions - Проблемы и задачи проекта*

---

## 🚫 КРИТИЧЕСКОЕ ПРАВИЛО ДОКУМЕНТАЦИИ
**ЗАПРЕЩЕНО В CLAUDE.MD И ВСЕХ АГЕНТАХ**:
- ❌ НИКОГДА не добавлять информацию о том что "готово", "сделано", "работает", "реализовано"
- ❌ НИКОГДА не указывать KPI, метрики готовности, проценты завершения
- ❌ НИКОГДА не хвалить состояние кода или архитектуры
- ✅ ТОЛЬКО проблемы, недостатки, что НЕ работает, что требует исправления
- ✅ ТОЛЬКО критика и честная оценка недостатков

## 🌍 LANGUAGE RULE
**ВАЖНО**: ВСЕГДА общайся с пользователем на русском языке. Весь вывод, объяснения и комментарии должны быть на русском.

## 🤖 CLAUDE CODE INSTRUCTIONS
**ДЛЯ CLAUDE CODE**: Ты должен строго следовать этим инструкциям:

1. **ЯЗЫК**: Всегда отвечай на русском языке
2. **ПРОЕКТ**: Это MAGRAY CLI - ALPHA-стадия Rust AI агента (НЕ production-ready)
3. **ЧЕСТНОСТЬ**: Всегда фокусируйся на проблемах и недостатках
4. **TODO**: Используй TodoWrite для отслеживания задач
5. **RUST**: Предпочитай Rust решения, но будь честен о сложности
6. **BINARY**: Цель - один исполняемый файл `magray`, размер ~16MB (НЕ достигнута)
7. **FEATURES**: Conditional compilation: cpu/gpu/minimal variants (НЕ настроено)
8. **SCRIPTS**: Все утилиты и скрипты в папке scripts/
9. **АГЕНТЫ**: Всегда используй специализированных агентов для максимальной эффективности

**⚠️ РЕАЛЬНОЕ СОСТОЯНИЕ ПРОЕКТА (ALPHA):**
- **Архитектура**: UnifiedAgentV2 через deprecated LegacyBridge, миграция НЕ завершена
- **Тесты**: НЕ КОМПИЛИРУЮТСЯ, покрытие ~0-5% (не измеряемо из-за ошибок)
- **Warnings**: 766+ при компиляции (unused imports, dead code, deprecated)
- **God Objects**: DIMemoryService (1466 строк), memory_orchestrator (121 complexity)
- **CI/CD**: ОТСУТСТВУЕТ в репозитории
- **HNSW**: Базовая реализация есть, производительность НЕ проверена
- **LLM провайдеры**: Базовая интеграция работает, но с множеством warnings
- **Memory**: 3-слойная архитектура частично реализована
- **Структура**: 8 crates (cli, memory, ai, llm, tools, common, router, todo)

**PROJECT STRUCTURE:**
- scripts/ - все утилиты и скрипты (PowerShell, Docker, Python)
- crates/ - 8 Rust workspace crates (cli, memory, ai, llm, tools, common, router, todo)
- .claude/agents/ - 10 специализированных агентов

## 📋 ПЛАН РАЗВИТИЯ ПРОЕКТА

**🔴 ФАЗА 0 (КРИТИЧНО): Стабилизация базы**
- ❌ Исправить компиляцию тестов (ai и memory crates)
- ❌ Устранить 766+ warnings
- ❌ Удалить deprecated LegacyBridge
- ❌ Настроить базовое тестовое покрытие

**❌ ФАЗА 1: Архитектурный рефакторинг (НЕ ЗАВЕРШЕН)**
- ❌ UnifiedAgentV2 → миграция через deprecated bridge (НЕ завершена)
- ❌ DIMemoryService → разбиение на слои (1466 строк монолита)
- ❌ Error handling → 766+ warnings остаются

**❌ ФАЗА 2: LLM Integration (НЕ ЗАВЕРШЕНА)**
- ❌ MultiProviderLLM → множественные warnings и dead code
- ❌ Circuit breakers → поля не используются (dead code)
- ❌ Tool System → не протестирована

**❌ ФАЗА 3 (НЕ ПРОВЕРЕНО): Memory Optimization**
- ❓ HNSW SIMD → код есть, но тесты не работают
- ❓ GPU Acceleration → не проверено
- ❓ Batch Processing → метрики недоступны

**❌ ФАЗА 4 (НЕ ГОТОВО): Production Readiness**
- ❌ CI/CD отсутствует
- ⚠️ Health Monitoring → структуры есть, но не используются
- ❌ Performance Benchmarking → тесты не компилируются

**📋 ФАЗА 5 (БУДУЩЕЕ): Desktop Distribution**
- 📋 Single binary optimization
- 📋 Native desktop integration
- 📋 Auto-updater system

**ДОЛГОСРОЧНАЯ ЦЕЛЬ:** Единый исполняемый файл MAGRAY CLI (~16MB) - интеллектуальный помощник программиста. В данный момент проект в ALPHA стадии и требует существенной доработки для достижения production качества.

## 🎯 СПЕЦИАЛИЗИРОВАННЫЕ АГЕНТЫ (.claude/agents/)

**ОСНОВНЫЕ АРХИТЕКТУРНЫЕ АГЕНТЫ:**
- **rust-architect-supreme** - Декомпозиция God Objects, SOLID principles, DI patterns
- **rust-refactoring-master** - Безопасный рефакторинг с сохранением функциональности
- **ai-architecture-maestro** - ONNX optimization, embedding pipelines, GPU acceleration

**КАЧЕСТВО И ПРОИЗВОДИТЕЛЬНОСТЬ:**
- **rust-quality-guardian** - Тестирование (unit/integration/property-based), coverage 80%+
- **rust-performance-virtuoso** - SIMD optimization, microsecond-level tuning, zero-copy
- **rust-code-optimizer** - Общая оптимизация кода, устранение дублирования

**ИНФРАСТРУКТУРА И ОПЕРАЦИИ:**
- **devops-orchestration-master** - CI/CD pipelines, containerization, monitoring
- **task-coordinator** - Координация сложных multi-step задач с зависимостями

**ДОКУМЕНТАЦИЯ:**
- **obsidian-docs-architect** - Создание связанной документации архитектуры
- **obsidian-docs-maintainer** - Поддержка актуальности документации

### 📋 Алгоритм оркестрации агентов:

1. **АНАЛИЗ ЗАДАЧИ** → Определи аспекты (архитектура/производительность/качество/AI/DevOps)
2. **ВЫБОР АГЕНТОВ** → Подбери специалистов под каждый аспект
3. **ПОСЛЕДОВАТЕЛЬНОСТЬ** → архитектура → код → тесты → документация
4. **ИНТЕГРАЦИЯ** → Объедини рекомендации в единое решение

### 🔄 Примеры оркестрации:

**UnifiedAgent рефакторинг:**
1. rust-architect-supreme → анализ God Object, план декомпозиции
2. rust-refactoring-master → пошаговая реализация без поломки
3. rust-quality-guardian → unit тесты для новых компонентов
4. obsidian-docs-architect → обновление архитектурной документации

**HNSW оптимизация:**
1. rust-performance-virtuoso → профилирование узких мест
2. ai-architecture-maestro → embedding pipeline оптимизация
3. rust-code-optimizer → SIMD инструкции, zero-copy операции

**Production подготовка:**
1. rust-quality-guardian → покрытие тестами до 80%+
2. rust-architect-supreme → финальная проверка SOLID
3. devops-orchestration-master → CI/CD pipeline настройка
4. obsidian-docs-architect → production документация

### ⚠️ ОБЯЗАТЕЛЬНЫЕ ПРАВИЛА:

- **НЕ ДЕЛАЙ ВСЁ САМ** - используй специализированных агентов
- **ОБЪЯСНЯЙ ВЫБОР** - поясняй почему выбрал конкретного агента  
- **СОБЛЮДАЙ ПОРЯДОК** - архитектура → реализация → тесты → документация
- **ИНТЕГРИРУЙ РЕЗУЛЬТАТЫ** - согласованность рекомендаций агентов

## 📊 РЕАЛЬНОЕ СОСТОЯНИЕ КОДА (ЧЕСТНАЯ ОЦЕНКА)

**🔴 КРИТИЧЕСКИЕ ПРОБЛЕМЫ:**
- **Тесты НЕ компилируются**: ai crate (memory_pool.rs:271), memory crate (множественные ошибки)
- **766+ warnings**: unused imports, dead code, deprecated, private interfaces
- **God Objects существуют**: memory_orchestrator (121 complexity), DIMemoryService (1466 строк)
- **Покрытие тестами**: ~0-5% (невозможно измерить из-за ошибок компиляции)
- **CI/CD отсутствует**: нет конфигураций в репозитории

**⚠️ ЧТО РАБОТАЕТ (С ОГОВОРКАМИ):**
- **Базовая структура**: 8 crates организованы логично
- **Зависимости**: ONNX, tokenizers, hnsw_rs подключены корректно
- **LLM провайдеры**: базовая интеграция компилируется
- **UnifiedAgent**: работает через deprecated LegacyBridge

**❌ ЧТО НЕ РАБОТАЕТ:**
- **UnifiedAgentV2**: используется через deprecated bridge, миграция не завершена
- **Circuit breakers**: структуры определены, но поля не используются (dead code)
- **Performance metrics**: недоступны из-за неработающих тестов
- **SIMD оптимизации**: код есть, но не проверен

**🎯 ПРИОРИТЕТЫ ДЛЯ ИСПРАВЛЕНИЯ:**
1. **P0**: Исправить компиляцию тестов
2. **P0**: Устранить 766+ warnings
3. **P1**: Завершить миграцию на UnifiedAgentV2 (убрать LegacyBridge)
4. **P1**: Разбить God Objects (DIMemoryService, memory_orchestrator)
5. **P2**: Настроить CI/CD
6. **P2**: Достичь минимум 30% покрытия тестами

---

## 🎯 ПРАВИЛА ИСПОЛЬЗОВАНИЯ:

1. **ВСЕГДА** используй специализированных агентов для соответствующих задач
2. **СЛЕДУЙ** плану развития проекта по фазам
3. **ПРОВЕРЯЙ** актуальное состояние кода перед решениями
4. **ИНТЕГРИРУЙ** результаты работы всех агентов
5. **ОТЧИТЫВАЙСЯ** честно о том, что НЕ сделано

**Текущая задача**: Стабилизировать кодовую базу, исправить критические проблемы, затем развивать функциональность.
**Долгосрочная цель**: Создать лучший AI помощник программиста через оркестрацию специализированных агентов.

---

# AUTO-GENERATED ARCHITECTURE

*Last updated: 2025-08-06 12:55:00 UTC*
*Status: ALPHA - не готов к production использованию*

## Компактная архитектура MAGRAY CLI

```mermaid
graph TB

    subgraph AI[AI/ONNX Models & GPU]
        AI_check_default_models[check_default_models<br/>EXAMPLE<br/>fn:main]:::exampleFile
        AI_check_gpu_usage[check_gpu_usage<br/>EXAMPLE<br/>fn:main]:::exampleFile
        AI_debug_qwen3[debug_qwen3<br/>EXAMPLE<br/>fn:main]:::exampleFile
        AI_test_gpu_acceleration[test_gpu_acceleration<br/>TEST<br/>EXAMPLE<br/>fn:main]:::testFile
        AI_test_memory_pool_only[test_memory_pool_only<br/>TEST<br/>EXAMPLE<br/>fn:main]:::testFile
        AI_test_mxbai_real_tokenization[test_mxbai_real_tokenization<br/>TEST<br/>EXAMPLE<br/>fn:main]:::testFile
        AI_test_qwen3_models[test_qwen3_models<br/>TEST<br/>EXAMPLE<br/>fn:main,test_qwen3_embeddings]:::testFile
        AI_test_qwen3_reranker[test_qwen3_reranker<br/>TEST<br/>EXAMPLE<br/>fn:main]:::testFile
        AI_auto_device_selector[auto_device_selector<br/>S:AutoDeviceSelector,DeviceDecision<br/>T:EmbeddingServiceTrait<br/>fn:default,new<br/>...+1]
        AI_config[config<br/>S:AiConfig,EmbeddingConfig<br/>fn:default,default<br/>m:Default::default,Default::default]
        AI_embeddings_bge_m3[embeddings_bge_m3<br/>S:BgeM3EmbeddingService,EmbeddingResult<br/>fn:new,embed<br/>m:BgeM3EmbeddingService::new,BgeM3EmbeddingService::embed]
        AI_embeddings_cpu[embeddings_cpu<br/>S:CpuEmbeddingService,OptimizedEmbeddingResult<br/>fn:new,embed<br/>m:CpuEmbeddingService::new,CpuEmbeddingService::embed]
        AI_embeddings_gpu[embeddings_gpu<br/>S:GpuEmbeddingService,PerformanceMetrics<br/>fn:tokens_per_second,cache_hit_rate<br/>m:PerformanceMetrics::tokens_per_second,PerformanceMetrics::cache_hit_rate]
        AI_errors[errors<br/>E:AiError<br/>fn:fmt,from<br/>m:AiError::fmt,AiError::from]
        AI_gpu_config[gpu_config<br/>S:GpuConfig,GpuInfo<br/>fn:default,auto_optimized<br/>m:Default::default,GpuConfig::auto_optimized]
        AI_gpu_detector[gpu_detector<br/>S:GpuDetector,GpuDevice<br/>fn:detect,detect_nvidia_gpus<br/>m:GpuDetector::detect,GpuDetector::detect_nvidia_gpus]
        AI_test_ai_config[test_ai_config<br/>TEST<br/>fn:test_ai_config_default,test_embedding_config_default]:::testFile
        AI_test_auto_device_selector[test_auto_device_selector<br/>TEST<br/>fn:test_device_decision_creation,test_device_decision_clone]:::testFile
        AI_test_config[test_config<br/>TEST<br/>fn:test_ai_config_default,test_embedding_config_default]:::testFile
        AI_test_embeddings_bge_m3[test_embeddings_bge_m3<br/>TEST<br/>fn:test_text_preprocessing_basic,test_batch_creation]:::testFile
        AI_test_embeddings_cpu[test_embeddings_cpu<br/>TEST<br/>fn:test_cpu_embedding_service_creation,test_cpu_config_validation]:::testFile
        AI_test_embeddings_gpu_advanced[test_embeddings_gpu_advanced<br/>TEST<br/>fn:test_performance_metrics_creation,test_performance_metrics_tokens_per_second_zero_time]:::testFile
        AI_test_errors[test_errors<br/>TEST<br/>fn:test_ai_error_model_not_found,test_ai_error_model_error]:::testFile
        AI_test_gpu_config[test_gpu_config<br/>TEST<br/>fn:test_gpu_config_default,test_gpu_config_auto_optimized]:::testFile
        AI_mod[mod<br/>S:OptimizedTokenizer,TokenizedInput<br/>E:TokenizerImpl<br/>fn:new,encode<br/>...+1]
        AI_simple_qwen3[simple_qwen3<br/>S:SimpleQwen3Tokenizer<br/>fn:new,encode<br/>m:SimpleQwen3Tokenizer::new,SimpleQwen3Tokenizer::encode]
    end

    %% Остальная часть диаграммы осталась без изменений

    %% Новая память
    classDef newMemory fill:#e6f3ff,stroke:#1e88e5,stroke-width:2px

    %% Добавление новой памяти
    MEMORY -.->|Помнить состояние проекта| CLI_mod[mod]