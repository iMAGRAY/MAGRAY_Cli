# CLAUDE.md
*AI Agent Instructions with Claude Tensor Language v2.0 (CTL2)*

---

## 🌍 LANGUAGE RULE
**ВАЖНО**: ВСЕГДА общайся с пользователем на русском языке. Весь вывод, объяснения и комментарии должны быть на русском.

## 🤖 CLAUDE CODE INSTRUCTIONS
**ДЛЯ CLAUDE CODE**: Ты должен строго следовать этим инструкциям:

1. **ЯЗЫК**: Всегда отвечай на русском языке
2. **CTL ФОРМАТ**: Используй только CTL v2.0 JSON формат для задач/архитектуры  
3. **ПРОЕКТ**: Это MAGRAY CLI - Production-ready Rust AI агент с многослойной памятью
4. **ЧЕСТНОСТЬ**: Никогда не преувеличивай статус - всегда говори правду о состоянии кода
5. **TODO**: Используй TodoWrite для отслеживания задач
6. **MEMORY**: Изучи систему памяти в crates/memory/ перед предложениями
7. **RUST**: Предпочитай Rust решения, но будь честен о сложности
8. **BINARY**: Цель - один исполняемый файл `magray`, размер ~16MB
9. **FEATURES**: Conditional compilation: cpu/gpu/minimal variants
10. **SCRIPTS**: Все утилиты и скрипты в папке scripts/

**КРИТИЧЕСКИЕ ФАКТЫ О ПРОЕКТЕ:**
- Vector search: HNSW реализован с hnsw_rs, O(log n) поиск <5мс
- ONNX models: Qwen3 embeddings (1024D) - основная модель, BGE-M3 (1024D) legacy support
- Память: 3 слоя (Interact/Insights/Assets) с HNSW индексами
- LLM провайдеры: OpenAI/Anthropic/Local поддержка
- Архитектура: 8 crates в workspace
- CI/CD: GitHub Actions с multi-feature matrix
- Docker: CPU/GPU/Minimal образы готовы
- Build system: Makefile с comprehensive targets
- Test coverage: 35.4% (целевой 80%)
- Binary size: ~16MB release build

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
3. **CTL v2.0 ФОРМАТ** - только JSON для задач/архитектуры
4. **TODOWRITE ОБЯЗАТЕЛЬНО** - для каждой многошаговой задачи
5. **ИЗУЧАЙ ПЕРЕД ДЕЙСТВИЕМ** - читай код перед предложениями
6. **ОДНА ЦЕЛЬ = ОДИН ФАЙЛ** - magray binary, а не ourcli
7. **НЕ ХВАСТАЙСЯ УСПЕХАМИ** - в конце каждой задачи сообщай что НЕ сделано
8. **ПОМЕЧАЙ КОД CTL АННОТАЦИЯМИ** - добавляй @component к каждому новому компоненту

## 📝 Memory Section

### Additional Memory Guidelines
- **Старайся чаще выступать в роли оркестратора, Всегда используя соответствующих агентов для выполнения подходящих задач, если есть подходящие агенты**

---

## 📊 АКТУАЛЬНОЕ СОСТОЯНИЕ КОДА (AUTO-UPDATED)

**ВАЖНО ДЛЯ AI**: Секции ниже обновляются автоматически демоном синхронизации каждые 5 минут.
Эти данные отражают РЕАЛЬНОЕ текущее состояние кодовой базы:

- **Components (CTL v2.0 Format)** - все аннотированные компоненты с их готовностью
- **x_file** - точное расположение файла в проекте
- **m.cur** - текущий процент готовности компонента (честная оценка)
- **m.tgt** - целевой процент готовности
- **f** - флаги/теги компонента

Используй эти данные для:
1. Понимания реальной структуры проекта
2. Оценки готовности компонентов
3. Навигации по кодовой базе
4. Определения зависимостей между компонентами

**Последнее обновление**: см. timestamp в секции AUTO-GENERATED ARCHITECTURE

---

# AUTO-GENERATED ARCHITECTURE

*Last updated: 2025-08-05 18:41:29 UTC*

## Components (CTL v2.0 Format)

```json
{"f":["agent","planning","llm"],"id":"action_planner","k":"C","m":{"cur":70,"tgt":95,"u":"%"},"t":"Multi-step action planner agent","x_file":"llm/src/agents/action_planner.rs:20"}
{"f":["ui","progress","adaptive"],"id":"adaptive_spinner","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Smart adaptive progress spinner","x_file":"cli/src/progress.rs:108"}
{"f":["ai","config","models"],"id":"ai_config","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"AI system configuration","x_file":"ai/src/config.rs:4"}
{"f":["ai","embeddings","onnx","bge-m3"],"id":"ai_lib","k":"C","m":{"cur":85,"tgt":95,"u":"%"},"t":"AI/ML services library","x_file":"ai/src/lib.rs:1"}
{"f":["ai","gpu","device-selection"],"id":"auto_device_selector","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Auto CPU/GPU selector","x_file":"ai/src/auto_device_selector.rs:9"}
{"f":["orchestration","backup","coordinator"],"id":"backup_coordinator","k":"C","m":{"cur":0,"tgt":90,"u":"%"},"t":"Backup orchestration coordinator","x_file":"memory/src/orchestration/backup_coordinator.rs:13"}
{"f":["errors","cache"],"id":"cache_error","k":"C","m":{"cur":85,"tgt":95,"u":"%"},"t":"Cache error types","x_file":"common/src/errors.rs:199"}
{"f":["cli","interface","commands","interactive"],"id":"cli_lib","k":"C","m":{"cur":85,"tgt":95,"u":"%"},"t":"CLI interface and commands","x_file":"cli/src/lib.rs:1"}
{"f":["common","logging","structured","utils"],"id":"common_lib","k":"C","m":{"cur":90,"tgt":95,"u":"%"},"t":"Common utilities and logging","x_file":"common/src/lib.rs:1"}
{"f":["errors","database"],"id":"database_error","k":"C","m":{"cur":85,"tgt":95,"u":"%"},"t":"Database error types","x_file":"common/src/errors.rs:74"}
{"f":["sled","concurrent","pooling"],"id":"database_manager","k":"C","m":{"cur":70,"tgt":100,"u":"%"},"t":"Centralized sled database manager","x_file":"memory/src/database_manager.rs:9"}
{"f":["di","ioc","architecture","validation","performance","async"],"id":"di_container","k":"C","m":{"cur":90,"tgt":95,"u":"%"},"t":"Dependency injection container","x_file":"memory/src/di_container.rs:36"}
{"f":["di","memory","clean_architecture","stub"],"id":"di_memory_service","k":"C","m":{"cur":5,"tgt":95,"u":"%"},"t":"DI-based memory service orchestrator","x_file":"memory/src/service_di.rs:86"}
{"f":["tools","directory","list"],"id":"dir_lister","k":"C","m":{"cur":85,"tgt":95,"u":"%"},"t":"Directory listing tool","x_file":"tools/src/file_ops.rs:150"}
{"f":["cache","lru","eviction","unified"],"id":"embedding_cache","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"LRU cache with eviction policy","x_file":"memory/src/cache_lru.rs:44"}
{"f":["ai","config","embeddings"],"id":"embedding_config","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Embedding model configuration","x_file":"ai/src/config.rs:15"}
{"f":["orchestration","embeddings","coordinator"],"id":"embedding_coordinator","k":"C","m":{"cur":0,"tgt":90,"u":"%"},"t":"Embedding orchestration coordinator","x_file":"memory/src/orchestration/embedding_coordinator.rs:16"}
{"f":["errors","ai","embeddings"],"id":"embedding_error","k":"C","m":{"cur":80,"tgt":95,"u":"%"},"t":"Embedding error types","x_file":"common/src/errors.rs:142"}
{"f":["ai","embeddings","cpu","onnx"],"id":"embeddings_cpu","k":"C","m":{"cur":90,"tgt":95,"u":"%"},"t":"CPU-based embeddings","x_file":"ai/src/embeddings_cpu.rs:15"}
{"f":["ai","embeddings","gpu","cuda","tensorrt"],"id":"embeddings_gpu","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"GPU-accelerated embeddings","x_file":"ai/src/embeddings_gpu.rs:17"}
{"f":["monitoring","errors","alerting"],"id":"error_monitor","k":"C","m":{"cur":0,"tgt":95,"u":"%"},"t":"Error monitoring and alerting system","x_file":"common/src/error_monitor.rs:11"}
{"f":["errors","monitoring","alerting"],"id":"error_severity","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Error severity levels","x_file":"common/src/errors.rs:292"}
{"f":["tools","file","read"],"id":"file_reader","k":"C","m":{"cur":90,"tgt":95,"u":"%"},"t":"File reading tool","x_file":"tools/src/file_ops.rs:8"}
{"f":["tools","search","file"],"id":"file_searcher","k":"C","m":{"cur":80,"tgt":90,"u":"%"},"t":"File search tool","x_file":"tools/src/file_ops.rs:253"}
{"f":["tools","file","write"],"id":"file_writer","k":"C","m":{"cur":90,"tgt":95,"u":"%"},"t":"File writing tool","x_file":"tools/src/file_ops.rs:82"}
{"f":["config","performance","reliability"],"id":"flush_config","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Configurable flush intervals","x_file":"memory/src/flush_config.rs:3"}
{"f":["tools","git","commit"],"id":"git_commit","k":"C","m":{"cur":85,"tgt":95,"u":"%"},"t":"Git commit tool","x_file":"tools/src/git_ops.rs:66"}
{"f":["tools","git","diff"],"id":"git_diff","k":"C","m":{"cur":80,"tgt":90,"u":"%"},"t":"Git diff tool","x_file":"tools/src/git_ops.rs:180"}
{"f":["tools","git","status"],"id":"git_status","k":"C","m":{"cur":90,"tgt":95,"u":"%"},"t":"Git status tool","x_file":"tools/src/git_ops.rs:6"}
{"f":["gpu","batch","embeddings","fallback","disabled"],"id":"gpu_batch_processor","k":"C","m":{"cur":60,"tgt":100,"u":"%"},"t":"GPU batch embedding processor","x_file":"memory/src/gpu_accelerated.rs:41"}
{"f":["cli","commands","gpu"],"id":"gpu_commands","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"GPU management CLI","x_file":"cli/src/commands/gpu.rs:13"}
{"f":["ai","gpu","config","onnx"],"id":"gpu_config","k":"C","m":{"cur":100,"tgt":100,"u":"%"},"t":"GPU configuration for ONNX","x_file":"ai/src/gpu_config.rs:13"}
{"f":["ai","gpu","detection","cuda"],"id":"gpu_detector","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"GPU detection and info","x_file":"ai/src/gpu_detector.rs:6"}
{"f":["errors","gpu"],"id":"gpu_error","k":"C","m":{"cur":85,"tgt":95,"u":"%"},"t":"GPU error types","x_file":"common/src/errors.rs:162"}
{"f":["fallback","resilience","gpu"],"id":"gpu_fallback_manager","k":"C","m":{"cur":100,"tgt":100,"u":"%"},"t":"Reliable GPU fallback system","x_file":"ai/src/gpu_fallback.rs:142"}
{"id":"gpu_memory_pool","k":"C","m":{"cur":90,"tgt":100,"u":"%"},"t":"GPU memory pool manager","x_file":"ai/src/gpu_memory_pool.rs:6"}
{"f":["gpu","pipeline","parallel","optimized"],"id":"gpu_pipeline_manager","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"GPU pipeline for parallel batches","x_file":"ai/src/gpu_pipeline.rs:9"}
{"f":["fallback","resilience"],"id":"graceful_embedding","k":"C","m":{"cur":90,"tgt":95,"u":"%"},"t":"Fallback embedding service","x_file":"memory/src/fallback.rs:137"}
{"f":["monitoring","production"],"id":"health_checks","k":"C","m":{"cur":100,"tgt":100,"u":"%"},"t":"Production health monitoring","x_file":"cli/src/health_checks.rs:10"}
{"f":["orchestration","health","monitoring"],"id":"health_manager","k":"C","m":{"cur":0,"tgt":90,"u":"%"},"t":"Health monitoring coordinator","x_file":"memory/src/orchestration/health_manager.rs:12"}
{"f":["monitoring","production"],"id":"health_monitor","k":"C","m":{"cur":85,"tgt":95,"u":"%"},"t":"Health monitoring system","x_file":"memory/src/health.rs:134"}
{"f":["errors","index","vector"],"id":"index_error","k":"C","m":{"cur":85,"tgt":95,"u":"%"},"t":"Vector index error types","x_file":"common/src/errors.rs:219"}
{"f":["agent","classification","intent"],"id":"intent_analyzer","k":"C","m":{"cur":70,"tgt":95,"u":"%"},"t":"Chat vs tool intent classifier","x_file":"llm/src/agents/intent_analyzer.rs:12"}
{"f":["memory","types","enum"],"id":"layer_enum","k":"C","m":{"cur":100,"tgt":100,"u":"%"},"t":"Memory layer enum types","x_file":"memory/src/types.rs:6"}
{"f":["llm","agents","multi-provider"],"id":"llm_client","k":"C","m":{"cur":65,"tgt":95,"u":"%"},"t":"Multi-provider LLM client","x_file":"llm/src/lib.rs:6"}
{"f":["errors","production","monitoring"],"id":"magray_error_types","k":"C","m":{"cur":85,"tgt":95,"u":"%"},"t":"Comprehensive error type system","x_file":"common/src/errors.rs:5"}
{"f":["di","config","memory"],"id":"memory_di_config","k":"C","m":{"cur":60,"tgt":100,"u":"%"},"t":"DI configuration for memory system","x_file":"memory/src/di_memory_config.rs:27"}
{"f":["errors","memory"],"id":"memory_error","k":"C","m":{"cur":80,"tgt":95,"u":"%"},"t":"Memory system error types","x_file":"common/src/errors.rs:182"}
{"f":["memory","hnsw","layers","orchestration"],"id":"memory_lib","k":"C","m":{"cur":75,"tgt":95,"u":"%"},"t":"3-layer HNSW memory system","x_file":"memory/src/lib.rs:1"}
{"f":["orchestration","coordinator","main"],"id":"memory_orchestrator","k":"C","m":{"cur":0,"tgt":95,"u":"%"},"t":"Main memory system orchestrator","x_file":"memory/src/orchestration/memory_orchestrator.rs:24"}
{"f":["memory","types","record"],"id":"memory_record","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Memory record structure","x_file":"memory/src/types.rs:32"}
{"f":["memory","types","core"],"id":"memory_types","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Memory system core types","x_file":"memory/src/types.rs:1"}
{"f":["metrics","monitoring"],"id":"metrics_collector","k":"C","m":{"cur":60,"tgt":95,"u":"%"},"t":"Memory system metrics","x_file":"memory/src/metrics.rs:9"}
{"id":"ml_promotion_engine","k":"C","m":{"cur":70,"tgt":100,"u":"%"},"t":"ML-based smart promotion system","x_file":"memory/src/ml_promotion.rs:92"}
{"id":"model_downloader","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Auto model downloader","x_file":"ai/src/model_downloader.rs:11"}
{"f":["models","config","registry"],"id":"model_registry","k":"C","m":{"cur":100,"tgt":100,"u":"%"},"t":"Centralized model registry","x_file":"ai/src/model_registry.rs:6"}
{"f":["cli","commands","models"],"id":"models_commands","k":"C","m":{"cur":100,"tgt":100,"u":"%"},"t":"Model management CLI","x_file":"cli/src/commands/models.rs:6"}
{"f":["errors","network"],"id":"network_error","k":"C","m":{"cur":85,"tgt":95,"u":"%"},"t":"Network error types","x_file":"common/src/errors.rs:97"}
{"f":["alerts","notifications","production"],"id":"notification_system","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Production alert notification system","x_file":"memory/src/notifications.rs:10"}
{"f":["agent","nlp","extraction"],"id":"param_extractor","k":"C","m":{"cur":70,"tgt":95,"u":"%"},"t":"Parameter extraction agent","x_file":"llm/src/agents/parameter_extractor.rs:13"}
{"f":["ui","progress"],"id":"progress_type","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Operation types for progress","x_file":"cli/src/progress.rs:5"}
{"f":["orchestration","promotion","coordinator"],"id":"promotion_coordinator","k":"C","m":{"cur":0,"tgt":90,"u":"%"},"t":"Promotion orchestration coordinator","x_file":"memory/src/orchestration/promotion_coordinator.rs:13"}
{"f":["promotion","time-index"],"id":"promotion_engine","k":"C","m":{"cur":75,"tgt":90,"u":"%"},"t":"Time-based memory promotion","x_file":"memory/src/promotion.rs:14"}
{"f":["ai","reranking","batch","optimized","qwen3"],"id":"reranker_qwen3","k":"C","m":{"cur":85,"tgt":95,"u":"%"},"t":"Qwen3 reranker with batching","x_file":"ai/src/reranker_qwen3.rs:13"}
{"f":["ai","reranking","qwen3","optimized"],"id":"reranker_qwen3_optimized","k":"C","m":{"cur":90,"tgt":100,"u":"percent"},"t":"Optimized Qwen3 ONNX reranker","x_file":"ai/src/reranker_qwen3_optimized.rs:11"}
{"f":["ai","config","reranking"],"id":"reranking_config","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Reranking model configuration","x_file":"ai/src/config.rs:33"}
{"f":["orchestration","resources","coordinator"],"id":"resource_controller","k":"C","m":{"cur":0,"tgt":90,"u":"%"},"t":"Resource management coordinator","x_file":"memory/src/orchestration/resource_controller.rs:12"}
{"f":["memory","scaling","adaptive"],"id":"resource_manager","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Dynamic memory resource management","x_file":"memory/src/resource_manager.rs:9"}
{"f":["retry","exponential","resilience"],"id":"retry_manager","k":"C","m":{"cur":90,"tgt":100,"u":"%"},"t":"Exponential backoff retry manager","x_file":"memory/src/retry.rs:7"}
{"f":["orchestration","search","coordinator"],"id":"search_coordinator","k":"C","m":{"cur":0,"tgt":90,"u":"%"},"t":"Search orchestration coordinator","x_file":"memory/src/orchestration/search_coordinator.rs:17"}
{"f":["tools","shell","execution"],"id":"shell_exec","k":"C","m":{"cur":85,"tgt":95,"u":"%"},"t":"Shell command execution tool","x_file":"tools/src/shell_ops.rs:6"}
{"id":"simple_qwen3_tokenizer","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Simplified Qwen3 tokenizer for ONNX","x_file":"ai/src/tokenization/simple_qwen3.rs:1"}
{"d":["llm_client","tools"],"f":["routing","orchestration"],"id":"smart_router","k":"C","m":{"cur":70,"tgt":90,"u":"%"},"t":"Smart task orchestration","x_file":"router/src/lib.rs:9"}
{"f":["cli","diagnostic","graceful-fallback"],"id":"status_cmd","k":"C","m":{"cur":100,"tgt":100,"u":"%"},"t":"System status diagnostic command","x_file":"cli/src/main.rs:420"}
{"f":["tests","status","cli"],"id":"status_tests","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Unit tests for status command","x_file":"cli/src/status_tests.rs:1"}
{"f":["serde","storage"],"id":"stored_record","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Serializable record wrapper","x_file":"memory/src/storage.rs:18"}
{"f":["streaming","real-time","async"],"id":"streaming_api","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"Real-time memory processing","x_file":"memory/src/streaming.rs:15"}
{"f":["logging","json","production"],"id":"structured_logging","k":"C","m":{"cur":100,"tgt":100,"u":"%"},"t":"JSON structured logging system","x_file":"common/src/structured_logging.rs:11"}
{"id":"tensorrt_cache","k":"C","m":{"cur":90,"tgt":100,"u":"%"},"t":"TensorRT model cache","x_file":"ai/src/tensorrt_cache.rs:8"}
{"id":"test_qwen3_models","k":"C","m":{"cur":100,"tgt":100,"u":"%"},"t":"Test Qwen3 models loading","x_file":"ai/examples/test_qwen3_models.rs:1"}
{"f":["todo","dag","sqlite","async"],"id":"todo_lib","k":"C","m":{"cur":80,"tgt":95,"u":"%"},"t":"Task DAG management system","x_file":"todo/src/lib.rs:1"}
{"f":["tools","execution","registry"],"id":"tool_registry","k":"C","m":{"cur":90,"tgt":95,"u":"%"},"t":"Tool execution system","x_file":"tools/src/lib.rs:5"}
{"f":["agent","tools","selection"],"id":"tool_selector","k":"C","m":{"cur":70,"tgt":95,"u":"%"},"t":"Tool selection agent","x_file":"llm/src/agents/tool_selector.rs:12"}
{"d":["llm_client","smart_router"],"f":["agents","routing","memory"],"id":"unified_agent","k":"C","m":{"cur":60,"tgt":90,"u":"%"},"t":"Main agent orchestrator","x_file":"cli/src/agent.rs:8"}
{"f":["errors","validation"],"id":"validation_error","k":"C","m":{"cur":90,"tgt":95,"u":"%"},"t":"Validation error types","x_file":"common/src/errors.rs:117"}
{"f":["vector","hnsw","search","legacy"],"id":"vector_index_hnsw","k":"C","m":{"cur":95,"tgt":100,"u":"%"},"t":"HNSW vector index wrapper","x_file":"memory/src/vector_index_hnswlib.rs:12"}
{"f":["storage","hnsw","transactional"],"id":"vector_store","k":"C","m":{"cur":65,"tgt":95,"u":"%"},"t":"Vector storage with HNSW","x_file":"memory/src/storage.rs:24"}
{"f":["tools","web","fetch"],"id":"web_fetch","k":"C","m":{"cur":70,"tgt":85,"u":"%"},"t":"Web page fetch tool","x_file":"tools/src/web_ops.rs:68"}
{"f":["tools","web","search"],"id":"web_search","k":"C","m":{"cur":75,"tgt":90,"u":"%"},"t":"Web search tool","x_file":"tools/src/web_ops.rs:5"}
{"f":["benchmark","performance","comprehensive"],"id":"comprehensive_bench","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"Comprehensive performance benchmarks","x_file":"memory/benches/comprehensive_performance.rs:7"}
{"f":["test","performance","comparison"],"id":"di_perf_comparison","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"DI container performance comparison","x_file":"memory/tests/test_di_performance_comparison.rs:14"}
{"f":["benchmark","performance","di"],"id":"di_performance_bench","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"DI performance benchmarking","x_file":"memory/benches/di_performance.rs:15"}
{"f":["integration","workflow","testing"],"id":"integration_tests","k":"T","m":{"cur":0,"tgt":90,"u":"%"},"t":"Full workflow integration tests","x_file":"memory/tests/integration_full_workflow.rs:13"}
{"f":["benchmarks","performance"],"id":"perf_benchmarks","k":"T","m":{"cur":0,"tgt":100,"u":"%"},"t":"Performance benchmarks для memory system","x_file":"memory/benches/vector_benchmarks.rs:10"}
{"f":["test","batch","api"],"id":"test_batch_operations","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"Test batch API functionality","x_file":"memory/examples/test_batch_operations.rs:8"}
{"f":["benchmark","gpu","optimization"],"id":"test_gpu_optimization","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"GPU optimization benchmark","x_file":"memory/examples/test_gpu_optimization.rs:9"}
{"f":["test","gpu","pipeline"],"id":"test_gpu_pipeline","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"Test GPU pipeline performance","x_file":"memory/examples/test_gpu_pipeline.rs:8"}
{"f":["profiler","gpu","performance"],"id":"test_gpu_profiler","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"Detailed GPU performance profiler","x_file":"memory/examples/test_gpu_profiler.rs:10"}
{"id":"test_memory_gpu","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"Memory GPU integration test","x_file":"memory/examples/test_gpu_memory_pool.rs:9"}
{"id":"test_memory_pool_only","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"Memory pool standalone test","x_file":"ai/examples/test_memory_pool_only.rs:7"}
{"id":"test_ml_promotion","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"ML promotion engine test","x_file":"memory/examples/test_ml_promotion.rs:10"}
{"f":["test","notifications","alerts"],"id":"test_notification_system","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"Test notification system integration","x_file":"memory/examples/test_notification_system.rs:12"}
{"f":["test","metrics","production"],"id":"test_production_metrics","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"Test production metrics integration","x_file":"memory/examples/test_production_metrics.rs:7"}
{"id":"test_real_tokenizer","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"Test real BPE tokenizer quality","x_file":"ai/examples/test_real_tokenizer.rs:1"}
{"id":"test_streaming","k":"T","m":{"cur":100,"tgt":100,"u":"%"},"t":"Test streaming API functionality","x_file":"memory/examples/test_streaming_api.rs:15"}
```

