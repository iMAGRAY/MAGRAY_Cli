# CLAUDE.md
*AI Agent Instructions - Тензорная архитектура для ИИ агентов*

---

## 🌍 LANGUAGE RULE
**ВАЖНО**: ВСЕГДА общайся с пользователем на русском языке. Весь вывод, объяснения и комментарии должны быть на русском.

## 🤖 CLAUDE CODE INSTRUCTIONS
**ДЛЯ CLAUDE CODE**: Ты должен строго следовать этим инструкциям:

1. **ЯЗЫК**: Всегда отвечай на русском языке
2. **ПРОЕКТ**: Это MAGRAY CLI - Production-ready Rust AI агент с многослойной памятью
3. **ЧЕСТНОСТЬ**: Никогда не преувеличивай статус - всегда говори правду о состоянии кода
4. **TODO**: Используй TodoWrite для отслеживания задач
5. **RUST**: Предпочитай Rust решения, но будь честен о сложности
6. **BINARY**: Цель - один исполняемый файл `magray`, размер ~16MB
7. **FEATURES**: Conditional compilation: cpu/gpu/minimal variants
8. **SCRIPTS**: Все утилиты и скрипты в папке scripts/
9. **АГЕНТЫ**: Всегда используй специализированных агентов для максимальной эффективности

**КРИТИЧЕСКИЕ ФАКТЫ О ПРОЕКТЕ:**
- Vector search: HNSW реализован с hnsw_rs, O(log n) поиск
- ONNX models: Qwen3 embeddings (1024D) - основная модель, BGE-M3 (1024D) legacy support
- Память: 3 слоя (Interact/Insights/Assets) с HNSW индексами
- LLM провайдеры: OpenAI/Anthropic/Local поддержка
- Архитектура: 8 crates в workspace

**PROJECT STRUCTURE:**
- scripts/ - все утилиты и скрипты (PowerShell, Docker, Python)
- crates/ - 8 Rust workspace crates (cli, memory, ai, llm, tools, common, router, todo)
- .claude/agents/ - 10 специализированных агентов

## 📋 ПЛАН РАЗВИТИЯ ПРОЕКТА

**ФАЗА 1 (2-3 месяца): Архитектурный рефакторинг**
- UnifiedAgent → trait-based сервисы (IntentAnalyzer, TaskPlanner, ToolOrchestrator)
- DIMemoryService (1466 строк) → слои Storage/Index/Query/Cache
- Устранение 766 .unwrap() вызовов с comprehensive error handling

**ФАЗА 2 (1-2 месяца): LLM Integration**
- Multi-Provider система (OpenAI/Anthropic/Local) с fallback
- Adaptive Task Orchestrator с AI-driven декомпозицией
- Dynamic Tool Discovery с Natural Language Interface

**ФАЗА 3 (1 месяц): Memory Optimization**
- HNSW SIMD + GPU acceleration, Memory Mapping
- Semantic clustering, ML-based ranking, Context windows

**ФАЗА 4 (1-2 месяца): Production Readiness**
- 80%+ test coverage, Comprehensive observability
- Security hardening, Performance benchmarking

**ФАЗА 5 (1 месяц): Desktop Distribution**
- Native integration, Single binary (~16MB), Auto-updater

**ИТОГОВАЯ ЦЕЛЬ:** Единый исполняемый файл MAGRAY CLI (~16MB) - интеллектуальный помощник программиста с многослойной памятью, поддержкой множества LLM провайдеров и продвинутыми возможностями автоматизации разработки

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

## 📊 АКТУАЛЬНОЕ СОСТОЯНИЕ КОДА

**ВАЖНОСТЬ**: Эти данные отражают РЕАЛЬНОЕ состояние кодовой базы для принятия решений:

**Ключевые метрики:**
- **Всего crates**: 8 (cli, memory, ai, llm, tools, common, router, todo)
- **God Objects**: UnifiedAgent (17 dependencies), DIMemoryService (1466 lines)  
- **Critical issues**: 766 .unwrap() calls требуют устранения
- **Memory система**: 3-layer HNSW (Interact/Insights/Assets)

**Архитектурные приоритеты:**
1. Декомпозиция UnifiedAgent на trait-based сервисы
2. Рефакторинг DIMemoryService на слои
3. Comprehensive error handling вместо .unwrap()
4. HNSW performance optimization с SIMD
5. Multi-provider LLM system с fallback

**Что должно получиться:**
- Стабильный production-ready AI помощник программиста
- Интеллектуальная автоматизация рутинных задач разработки
- Единый инструмент для работы с кодом, документацией и задачами
- Seamless интеграция с различными LLM провайдерами

---

## 🎯 ПРАВИЛА ИСПОЛЬЗОВАНИЯ:

1. **ВСЕГДА** используй специализированных агентов для соответствующих задач
2. **СЛЕДУЙ** плану развития проекта по фазам
3. **ПРОВЕРЯЙ** актуальное состояние кода перед решениями
4. **ИНТЕГРИРУЙ** результаты работы всех агентов
5. **ОТЧИТЫВАЙСЯ** честно о том, что НЕ сделано

**Цель**: Создать лучший AI помощник программиста через оркестрацию специализированных агентов.

---

# AUTO-GENERATED ARCHITECTURE

*Last updated: 2025-08-06 07:32:40 UTC*

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

    subgraph CLI[CLI Agent & Commands]
        CLI_agent[agent<br/>S:UnifiedAgent<br/>E:AgentResponse<br/>fn:new,process_message<br/>...+1]
        CLI_agent_tests[agent_tests<br/>TEST<br/>fn:create_test_message,test_unified_agent_initialization]:::testFile
        CLI_agent_traits[agent_traits<br/>S:IntentDecision,RequestContext<br/>T:IntentDecisionStrategy,FallbackStrategy<br/>E:AgentResponse,AdminResponse<br/>...+1]
        CLI_health_checks[health_checks<br/>MOCK<br/>S:HealthCheckResult,HealthCheckSystem<br/>T:HealthCheck<br/>...+5]:::mockFile
        CLI_lib[lib]
        CLI_main[main<br/>S:AnimatedIcon,Cli<br/>E:Commands<br/>fn:new,get_frame<br/>...+1]
        CLI_progress[progress<br/>S:ProgressConfig,AdaptiveSpinner<br/>E:ProgressType<br/>fn:config,create_spinner<br/>...+1]
        CLI_status_tests[status_tests<br/>TEST<br/>fn:test_show_system_status_no_panic,test_show_system_status_with_llm]:::testFile
        CLI_test_agent[test_agent<br/>TEST<br/>fn:test_unified_agent_creation,test_agent_simple_message]:::testFile
        CLI_test_cli[test_cli<br/>TEST<br/>MOCK<br/>S:TestFuture,CustomError<br/>...+3]:::testFile
        CLI_test_commands_gpu[test_commands_gpu<br/>TEST<br/>MOCK<br/>S:TestCli<br/>...+3]:::testFile
        CLI_test_commands_memory[test_commands_memory<br/>TEST<br/>fn:test_memory_command_args_trait,check_args_trait<br/>unsafe:1]:::testFile
        CLI_test_commands_models[test_commands_models<br/>TEST<br/>MOCK<br/>S:TestCli<br/>...+3]:::testFile
        CLI_test_health_checks[test_health_checks<br/>TEST<br/>fn:test_health_status_display,test_health_check_result_creation]:::testFile
        CLI_test_memory_integration[test_memory_integration<br/>TEST<br/>fn:test_memory_command_structure,_type_check]:::testFile
        CLI_test_progress[test_progress<br/>TEST<br/>fn:test_progress_type_configs,test_adaptive_spinner_creation]:::testFile
        CLI_gpu[gpu<br/>S:GpuCommand<br/>T:DecisionExt<br/>E:GpuSubcommand,CacheAction<br/>...+2]
        CLI_memory[memory<br/>S:MemoryCommand<br/>E:MemorySubcommand<br/>fn:execute,handle_memory_subcommand<br/>...+1]
        CLI_mod[mod]
        CLI_models[models<br/>S:ModelsCommand<br/>E:ModelsSubcommand<br/>fn:execute,list_models<br/>...+1]
        CLI_admin_handler[admin_handler<br/>MOCK<br/>S:AdminHandler,MockAdminService<br/>fn:new,handle_admin_request<br/>...+3]:::mockFile
        CLI_chat_handler[chat_handler<br/>MOCK<br/>S:ChatHandler,MockLlmService<br/>fn:new,handle_chat<br/>...+3]:::mockFile
        CLI_memory_handler[memory_handler<br/>MOCK<br/>S:MemoryHandler,MockMemoryService<br/>fn:new,store_user_message<br/>...+3]:::mockFile
        CLI_mod[mod]
        CLI_performance_monitor[performance_monitor<br/>S:PerformanceMonitor,OperationMetrics<br/>fn:default,new<br/>m:Default::default,PerformanceMonitor::new]
        CLI_tools_handler[tools_handler<br/>MOCK<br/>S:ToolsHandler,MockRoutingService<br/>fn:new,handle_tools<br/>...+3]:::mockFile
        CLI_adaptive_orchestrator[adaptive_orchestrator<br/>S:ResourceRequirements,OrchestrationTask<br/>T:TaskOrchestrator<br/>E:TaskPriority,TaskComplexity<br/>...+2]
        CLI_mod[mod]
        CLI_resource_manager[resource_manager<br/>S:SystemResourceUsage,ResourceLimits<br/>fn:default,default<br/>m:Default::default,Default::default]
        CLI_strategy_selector[strategy_selector<br/>S:SelectionCriteria,StrategyMetrics<br/>E:ExecutionStrategy<br/>fn:new,select_strategy<br/>...+1]
        CLI_task_analyzer[task_analyzer<br/>S:TaskAnalyzer<br/>fn:new,analyze_task<br/>m:TaskAnalyzer::new,TaskAnalyzer::analyze_task]
        CLI_tool_orchestrator[tool_orchestrator<br/>S:ToolOrchestratorConfig,ToolOrchestrator<br/>T:IntegratedOrchestrator<br/>fn:default,new<br/>...+1]
        CLI_di_config[di_config<br/>fn:register_services,create_services_container]
        CLI_intent_analysis[intent_analysis<br/>MOCK<br/>S:IntentAnalysisStats,DefaultIntentAnalysisService<br/>T:IntentAnalysisService<br/>...+4]:::mockFile
        CLI_llm_communication[llm_communication<br/>S:ChatOptions,LlmHealthStatus<br/>T:LlmCommunicationService<br/>fn:default,default<br/>...+1]
        CLI_mod[mod<br/>S:IntentDecision,RequestContext<br/>E:AgentResponse]
        CLI_orchestrator[orchestrator<br/>S:OrchestratorStats,ServicePerformance<br/>T:ServiceOrchestrator<br/>fn:new,create_request_context<br/>...+1]
        CLI_request_routing[request_routing<br/>S:RoutingRecommendation,ResourceRequirements<br/>T:RequestRoutingService<br/>E:RouteType<br/>...+2]
        CLI_resilience[resilience<br/>S:RetryConfig,ResilienceStats<br/>T:ResilienceService<br/>fn:default,new<br/>...+1]
        CLI_circuit_breaker[circuit_breaker<br/>S:CircuitBreakerMetrics,BasicCircuitBreaker<br/>E:CircuitBreakerState<br/>fn:default,new<br/>...+1]
        CLI_fallback_strategies[fallback_strategies<br/>S:SimpleFallbackStrategy,SmartFallbackStrategy<br/>fn:new,classify_error<br/>m:SimpleFallbackStrategy::new,SimpleFallbackStrategy::classify_error]
        CLI_intent_strategies[intent_strategies<br/>MOCK<br/>S:HeuristicIntentStrategy,LlmIntentStrategy<br/>fn:new,analyze_keywords<br/>...+3]:::mockFile
        CLI_mod[mod]
        CLI_response_strategies[response_strategies<br/>S:SimpleResponseFormatter,RichResponseFormatter<br/>fn:new,with_defaults<br/>m:SimpleResponseFormatter::new,SimpleResponseFormatter::with_defaults]
    end

    subgraph COMMON[Common Utilities]
        COMMON_comprehensive_errors[comprehensive_errors<br/>S:ErrorContext<br/>T:SafeUnwrap<br/>E:MagrayCoreError,MemoryError<br/>...+3]
        COMMON_errors[errors<br/>T:IsRetriable,IsRecoverable<br/>E:MagrayError,DatabaseError<br/>fn:is_retriable,is_recoverable<br/>...+2]
        COMMON_error_monitor[error_monitor<br/>S:ErrorMonitor,ErrorMonitorConfig<br/>fn:default,new<br/>m:Default::default,RateLimiter::new<br/>...+1]
        COMMON_lib[lib]
        COMMON_structured_logging[structured_logging<br/>S:StructuredLogEntry,ExecutionContext<br/>fn:default,on_event<br/>m:Default::default,JsonFormatter::on_event<br/>...+1]
        COMMON_test_logging_advanced[test_logging_advanced<br/>TEST<br/>fn:test_execution_context_default,test_execution_context_with_all_fields]:::testFile
        COMMON_test_structured_logging[test_structured_logging<br/>TEST<br/>fn:test_structured_log_entry_creation,test_execution_context]:::testFile
        COMMON_test_structured_logging_extended[test_structured_logging_extended<br/>TEST<br/>fn:test_structured_log_entry_full,test_structured_log_entry_minimal]:::testFile
    end

    subgraph LLM[Multi-Provider LLM]
        LLM_circuit_breaker[circuit_breaker<br/>S:CircuitBreaker<br/>E:CircuitBreakerState<br/>fn:default,new<br/>...+1]
        LLM_cost_optimizer[cost_optimizer<br/>S:CostTable,CostOptimizer<br/>fn:default,get_cost<br/>m:Default::default,CostTable::get_cost]
        LLM_integration_test[integration_test<br/>TEST<br/>fn:test_single_provider_mode,test_multi_provider_creation]:::testFile
        LLM_lib[lib<br/>S:ProviderStats,TaskComplexity<br/>E:LlmProvider,ProviderType<br/>fn:default,user<br/>...+1]
        LLM_multi_provider[multi_provider<br/>S:MultiProviderLlmOrchestrator,RetryConfig<br/>fn:default,new<br/>m:Default::default,MultiProviderLlmOrchestrator::new]
        LLM_test_agents[test_agents<br/>TEST<br/>MOCK<br/>fn:test_tool_selector_agent_creation,test_tool_selector_simple_commands<br/>...+1]:::testFile
        LLM_test_llm_advanced[test_llm_advanced<br/>TEST<br/>fn:test_llm_client_configuration,test_llm_client_from_env_variations]:::testFile
        LLM_test_llm_client[test_llm_client<br/>TEST<br/>MOCK<br/>fn:create_mock_openai_response,create_mock_anthropic_response<br/>...+1]:::testFile
        LLM_test_llm_integration[test_llm_integration<br/>TEST<br/>fn:test_end_to_end_chat_workflow,test_multi_step_planning_workflow]:::testFile
        LLM_action_planner[action_planner<br/>S:ActionPlan,PlanStep<br/>fn:new,create_plan<br/>m:ActionPlannerAgent::new,ActionPlannerAgent::create_plan]
        LLM_intent_analyzer[intent_analyzer<br/>S:IntentDecision,IntentAnalyzerAgent<br/>fn:new,analyze_intent<br/>m:IntentAnalyzerAgent::new,IntentAnalyzerAgent::analyze_intent]
        LLM_mod[mod]
        LLM_parameter_extractor[parameter_extractor<br/>S:ParameterExtraction,ParameterExtractorAgent<br/>fn:new,extract_parameters<br/>m:ParameterExtractorAgent::new,ParameterExtractorAgent::extract_parameters]
        LLM_tool_selector[tool_selector<br/>S:ToolSelection,ToolSelectorAgent<br/>fn:new,select_tool<br/>m:ToolSelectorAgent::new,ToolSelectorAgent::select_tool]
    end

    subgraph MEMORY[3-Layer HNSW Memory]
        MEMORY_comprehensive_performance[comprehensive_performance<br/>BENCH<br/>fn:create_test_record,bench_vector_store_operations]:::benchFile
        MEMORY_di_performance[di_performance<br/>BENCH<br/>S:LightweightService,HeavyService<br/>fn:new,new<br/>...+1]:::benchFile
        MEMORY_scalability_benchmarks[scalability_benchmarks<br/>BENCH<br/>fn:generate_embedding,bench_search_scalability]:::benchFile
        MEMORY_simple_test[simple_test<br/>TEST<br/>BENCH<br/>fn:simple_benchmark]:::testFile
        MEMORY_vector_benchmarks[vector_benchmarks<br/>BENCH<br/>fn:generate_random_vectors,create_test_records<br/>unsafe:2]:::benchFile
        MEMORY_benchmark_hnsw_vs_linear[benchmark_hnsw_vs_linear<br/>EXAMPLE<br/>fn:main,cosine_distance]:::exampleFile
        MEMORY_check_ort_version[check_ort_version<br/>EXAMPLE<br/>fn:main]:::exampleFile
        MEMORY_debug_simd_performance[debug_simd_performance<br/>EXAMPLE<br/>fn:main]:::exampleFile
        MEMORY_di_best_practices[di_best_practices<br/>EXAMPLE<br/>fn:main,create_optimized_config]:::exampleFile
        MEMORY_final_simd_integration_test[final_simd_integration_test<br/>TEST<br/>EXAMPLE<br/>fn:main,simulate_hnsw_workload<br/>...+1]:::testFile
        MEMORY_full_pipeline_test[full_pipeline_test<br/>TEST<br/>EXAMPLE<br/>fn:main]:::testFile
        MEMORY_memory_demo[memory_demo<br/>EXAMPLE<br/>fn:main]:::exampleFile
        MEMORY_perf_test[perf_test<br/>TEST<br/>EXAMPLE<br/>fn:create_test_record,main]:::testFile
        MEMORY_api[api<br/>S:UnifiedMemoryAPI,MemoryContext<br/>T:MemoryServiceTrait<br/>fn:search_sync,run_promotion_sync<br/>...+1]
        MEMORY_backup[backup<br/>S:BackupMetadata,LayerInfo<br/>fn:new,create_backup<br/>m:BackupManager::new,BackupManager::create_backup]
        MEMORY_batch_manager[batch_manager<br/>S:BatchConfig,BatchStats<br/>fn:default,new<br/>m:Default::default,BatchOperationManager::new]
        MEMORY_batch_optimized[batch_optimized<br/>S:BatchOptimizedConfig,BatchOptimizedStats<br/>E:BatchRequest<br/>fn:default,throughput_qps<br/>...+2]
        MEMORY_cache_interface[cache_interface<br/>T:EmbeddingCacheInterface<br/>fn:is_null_check,get<br/>m:EmbeddingCacheInterface::get,EmbeddingCacheInterface::insert]
        MEMORY_cache_lru[cache_lru<br/>S:CachedEmbedding,CacheConfig<br/>fn:default,new<br/>m:Default::default,LruIndex::new]
        MEMORY_cache_migration[cache_migration<br/>fn:migrate_cache_to_lru,recommend_cache_config]
        MEMORY_database_manager[database_manager<br/>S:DatabaseManager,DatabaseStats<br/>fn:new,global<br/>m:DatabaseManager::new,DatabaseManager::global]
        MEMORY_integration_full_workflow[integration_full_workflow<br/>TEST<br/>fn:test_complete_memory_system_workflow,test_performance_under_load]:::testFile
        MEMORY_integration_test[integration_test<br/>TEST<br/>fn:test_memory_service_basic_operations,test_memory_layers]:::testFile
        MEMORY_performance_test[performance_test<br/>TEST<br/>fn:generate_embedding,test_vector_search_performance]:::testFile
        MEMORY_test_cache_migration[test_cache_migration<br/>TEST<br/>fn:test_cache_config_recommendations,test_recommended_config_cache_creation]:::testFile
        MEMORY_test_database_manager[test_database_manager<br/>TEST<br/>fn:test_concurrent_database_access,test_memory_service_concurrent_creation]:::testFile
        MEMORY_test_di_async[test_di_async<br/>TEST<br/>fn:test_async_di_creation,test_async_factory_pattern]:::testFile
        MEMORY_test_di_async_simple[test_di_async_simple<br/>TEST<br/>fn:test_basic_async_di_creation,test_di_container_factory]:::testFile
        MEMORY_test_di_integration[test_di_integration<br/>TEST<br/>fn:main]:::testFile
        MEMORY_config[config<br/>S:HnswConfig<br/>fn:default,high_quality<br/>m:Default::default,HnswConfig::high_quality]
        MEMORY_index[index<br/>S:VectorIndex<br/>fn:cosine_distance_avx2,horizontal_sum_avx2<br/>m:VectorIndex::new,VectorIndex::detect_simd_capabilities<br/>...+1]
        MEMORY_mod[mod]
        MEMORY_stats[stats<br/>S:HnswStats,HnswStatsSnapshot<br/>fn:new,record_search<br/>m:HnswStats::new,HnswStats::record_search]
        MEMORY_backup_coordinator[backup_coordinator<br/>S:BackupCoordinator<br/>fn:new,initialize<br/>m:BackupCoordinator::new,Coordinator::initialize]
        MEMORY_embedding_coordinator[embedding_coordinator<br/>S:EmbeddingCoordinator,CircuitBreaker<br/>E:CircuitState<br/>fn:new,with_retry_policy<br/>...+1]
        MEMORY_health_manager[health_manager<br/>S:HealthManager,HealthMetrics<br/>E:AlertLevel<br/>fn:new,setup_production_monitoring<br/>...+1]
        MEMORY_memory_orchestrator[memory_orchestrator<br/>S:MemoryOrchestrator,CircuitBreakerState<br/>E:CircuitBreakerStatus<br/>fn:clone,new<br/>...+1]
        MEMORY_mod[mod]
        MEMORY_promotion_coordinator[promotion_coordinator<br/>S:PromotionCoordinator<br/>fn:new,initialize<br/>m:PromotionCoordinator::new,Coordinator::initialize]
        MEMORY_resource_controller[resource_controller<br/>S:ResourceController,ResourceMetrics<br/>E:ResourceAlertType<br/>fn:new,new_production<br/>...+1]
        MEMORY_retry_handler[retry_handler<br/>S:RetryPolicy,RetryHandler<br/>E:RetryResult<br/>fn:default,fast<br/>...+1]
        MEMORY_mod[mod<br/>TEST<br/>MOCK<br/>S:TestConfigBuilder,TestRecordBuilder<br/>...+4]:::testFile
        MEMORY_di_container_test[di_container_test<br/>TEST<br/>fn:create_di_test_service,create_di_test_record]:::testFile
        MEMORY_full_system_test[full_system_test<br/>TEST<br/>fn:create_test_record,create_production_test_service]:::testFile
        MEMORY_mod[mod<br/>TEST]:::testFile
        MEMORY_orchestration_test[orchestration_test<br/>TEST<br/>fn:create_orchestration_test_service,create_test_record]:::testFile
        MEMORY_performance_test[performance_test<br/>TEST<br/>fn:create_performance_test_service,create_perf_test_record]:::testFile
        MEMORY_resilience_test[resilience_test<br/>TEST<br/>fn:create_resilience_test_service,create_resilience_test_record]:::testFile
    end

    subgraph ROUTER[Smart Task Router]
        ROUTER_lib[lib<br/>S:ActionPlan,PlannedAction<br/>fn:new,analyze_and_plan<br/>m:SmartRouter::new,SmartRouter::analyze_and_plan]
        ROUTER_test_router[test_router<br/>TEST<br/>fn:create_test_llm_client,test_action_plan_creation]:::testFile
        ROUTER_test_router_async[test_router_async<br/>TEST<br/>MOCK<br/>S:MockTool<br/>...+4]:::testFile
        ROUTER_test_smart_router[test_smart_router<br/>TEST<br/>fn:test_smart_router_structure,test_planned_action_args]:::testFile
    end

    subgraph TODO[Task DAG System]
        TODO_graph[graph<br/>S:TaskNode,DependencyGraphV2<br/>fn:default,new<br/>m:Default::default,DependencyGraphV2::new]
        TODO_lib[lib<br/>fn:create_service,create_default_service]
        TODO_service_v2[service_v2<br/>S:TodoServiceV2,TodoEventStream<br/>fn:new,create_task<br/>m:TodoServiceV2::new,TodoServiceV2::create_task]
        TODO_store[store<br/>S:TodoStore<br/>fn:new,create<br/>m:TodoStore::new,TodoStore::create]
        TODO_store_v2[store_v2<br/>S:TodoStoreV2<br/>fn:new,init_schema<br/>m:TodoStoreV2::new,TodoStoreV2::init_schema]
        TODO_types[types<br/>S:MemoryReference,TodoItem<br/>E:TaskState,Priority<br/>fn:from_record,fmt<br/>...+1]
        TODO_test_extended[test_extended<br/>TEST<br/>fn:test_service_creation_variants,test_create_multiple_tasks]:::testFile
        TODO_test_graph[test_graph<br/>TEST<br/>fn:create_test_task,test_graph_creation]:::testFile
        TODO_test_service[test_service<br/>TEST<br/>fn:test_service_creation,test_create_task]:::testFile
        TODO_test_types[test_types<br/>TEST<br/>fn:test_task_state_display,test_task_state_from_str]:::testFile
    end

    subgraph TOOLS[Tools Registry]
        TOOLS_enhanced_tool_system[enhanced_tool_system<br/>S:EnhancedToolSystemConfig,EnhancedToolResult<br/>fn:default,new<br/>m:Default::default,EnhancedToolSystem::new]
        TOOLS_execution_pipeline[execution_pipeline<br/>S:ExecutionResult,CircuitBreakerConfig<br/>E:ExecutionStrategy,CircuitBreakerState<br/>fn:default,default<br/>...+1]
        TOOLS_file_ops[file_ops<br/>S:FileReader,FileWriter<br/>fn:new,default<br/>m:FileReader::new,Default::default]
        TOOLS_git_ops[git_ops<br/>S:GitStatus,GitCommit<br/>fn:new,default<br/>m:GitStatus::new,Default::default]
        TOOLS_intelligent_selector[intelligent_selector<br/>S:ToolConfidence,ToolSelectionContext<br/>E:TaskComplexity,UrgencyLevel<br/>fn:default,new<br/>...+1]
        TOOLS_lib[lib<br/>S:ToolInput,ToolOutput<br/>T:Tool<br/>fn:supports_natural_language,new<br/>...+1]
        TOOLS_performance_monitor[performance_monitor<br/>S:ToolPerformanceMetrics,ToolError<br/>E:PerformanceTrend,AlertLevel<br/>fn:default,new<br/>...+1]
        TOOLS_shell_ops[shell_ops<br/>S:ShellExec<br/>fn:new,default<br/>m:ShellExec::new,Default::default]
        TOOLS_test_file_ops[test_file_ops<br/>TEST<br/>fn:test_file_reader,test_file_reader_nonexistent]:::testFile
        TOOLS_test_git_ops[test_git_ops<br/>TEST<br/>fn:test_git_status_spec,test_git_status_natural_language_parsing]:::testFile
        TOOLS_test_registry[test_registry<br/>TEST<br/>MOCK<br/>S:MockTool<br/>...+4]:::testFile
        TOOLS_test_shell_ops[test_shell_ops<br/>TEST<br/>fn:test_shell_exec_spec,test_shell_exec_natural_language_parsing]:::testFile
        TOOLS_test_tool_types[test_tool_types<br/>TEST<br/>fn:test_tool_input_creation,test_tool_input_clone]:::testFile
        TOOLS_test_web_ops[test_web_ops<br/>TEST<br/>fn:test_web_search_spec,test_web_search_natural_language_parsing]:::testFile
    end

    %% Зависимости между крейтами
    CLI -.->|uses| AI
    CLI -.->|uses| TOOLS
    CLI -.->|uses| COMMON
    CLI -.->|uses| LLM
    CLI -.->|uses| ROUTER
    CLI -.->|uses| MEMORY
    MEMORY -.->|uses| AI
    MEMORY -.->|uses| COMMON
    ROUTER -.->|uses| TOOLS
    ROUTER -.->|uses| LLM
    TODO -.->|uses| LLM
    TODO -.->|uses| MEMORY
    TOOLS -.->|uses| LLM

    classDef crate fill:#e3f2fd,stroke:#1976d2,stroke-width:2px
    classDef file fill:#fff9c4,stroke:#f57c00,stroke-width:1px
    classDef testFile fill:#ffebee,stroke:#c62828,stroke-width:1px,stroke-dasharray: 5 5
    classDef mockFile fill:#fce4ec,stroke:#ad1457,stroke-width:1px,stroke-dasharray: 3 3
    classDef exampleFile fill:#e8f5e9,stroke:#2e7d32,stroke-width:1px
    classDef benchFile fill:#fff3e0,stroke:#e65100,stroke-width:1px
    classDef trait fill:#f3e5f5,stroke:#7b1fa2,stroke-width:1px
    classDef struct fill:#e8f5e9,stroke:#388e3c,stroke-width:1px
```

## Статистика проекта

- **Всего crates**: 8
- **Всего файлов**: 285
- **Активные зависимости**: 13
- **Основные компоненты**: CLI, Memory (3-Layer HNSW), AI/ONNX, LLM Multi-Provider
- **GPU поддержка**: CUDA + TensorRT через feature flags
- **Производительность**: HNSW O(log n) search, SIMD оптимизации

## Ключевые особенности

- **Единый исполняемый файл**: `magray` (target ~16MB)
- **Conditional compilation**: cpu/gpu/minimal variants
- **Memory система**: 3 слоя (Interact/Insights/Assets) с HNSW индексами  
- **AI модели**: Qwen3 embeddings (1024D), BGE-M3 legacy support
- **LLM провайдеры**: OpenAI/Anthropic/Local
- **Production готовность**: Circuit breakers, health checks, metrics

## 💸 Технический долг

**Общий долг**: 3513.0 часов (439.1 дней)
**Критических проблем**: 102
**Высокий приоритет**: 194

- [CRITICAL] Цикломатическая сложность 31 (должна быть < 10)
  - Файл: `ai/src/embeddings_bge_m3.rs`
  - Оценка: 5.5 часов
- [CRITICAL] Цикломатическая сложность 68 (должна быть < 10)
  - Файл: `ai/src/embeddings_cpu.rs`
  - Оценка: 16.0 часов
- [CRITICAL] God Object вероятность 80%
  - Файл: `ai/src/embeddings_cpu.rs`
  - Оценка: 16.0 часов
- [CRITICAL] Цикломатическая сложность 42 (должна быть < 10)
  - Файл: `ai/src/embeddings_gpu.rs`
  - Оценка: 11.0 часов
- [CRITICAL] God Object вероятность 90%
  - Файл: `ai/src/gpu_fallback.rs`
  - Оценка: 16.0 часов

## 📊 Метрики сложности

### Самые сложные файлы:
- `memory/src/orchestration/memory_orchestrator.rs`:
  - Цикломатическая: 121
  - Когнитивная: 3910
  - ⚠️ God Object: 100%
- `memory/src/gpu_accelerated.rs`:
  - Цикломатическая: 119
  - Когнитивная: 3050
  - ⚠️ God Object: 80%
- `memory/src/storage.rs`:
  - Цикломатическая: 105
  - Когнитивная: 2362
  - ⚠️ God Object: 80%
- `cli/src/main.rs`:
  - Цикломатическая: 98
  - Когнитивная: 2853
- `memory/src/cache_lru.rs`:
  - Цикломатическая: 98
  - Когнитивная: 2016
  - ⚠️ God Object: 80%

## 🔍 Обнаруженные дубликаты

- **impl AsRef** встречается 22 раз:
  - `ai/src/models.rs` (AsRef)
  - `ai/src/model_downloader.rs` (AsRef)
  - `ai/src/tensorrt_cache.rs` (AsRef)
  - ...и еще 19 мест
- **impl CircuitBreaker** встречается 2 раз:
  - `ai/src/gpu_fallback.rs` (CircuitBreaker)
  - `llm/src/circuit_breaker.rs` (CircuitBreaker)
- **impl CircuitBreakerTrait for MockCircuitBreaker** встречается 4 раз:
  - `cli/src/handlers/admin_handler.rs` (MockCircuitBreaker)
  - `cli/src/handlers/chat_handler.rs` (MockCircuitBreaker)
  - `cli/src/handlers/memory_handler.rs` (MockCircuitBreaker)
  - ...и еще 1 мест
- **impl DIContainer** встречается 2 раз:
  - `memory/examples/test_performance_metrics.rs` (DIContainer)
  - `memory/src/di_container.rs` (DIContainer)
- **impl DIPerformanceMetrics** встречается 2 раз:
  - `memory/examples/test_performance_metrics.rs` (DIPerformanceMetrics)
  - `memory/src/di_container.rs` (DIPerformanceMetrics)
- **impl Default for CircuitBreakerMetrics** встречается 2 раз:
  - `cli/src/strategies/circuit_breaker.rs` (CircuitBreakerMetrics)
  - `tools/src/execution_pipeline.rs` (CircuitBreakerMetrics)
- **impl Default for DIPerformanceMetrics** встречается 2 раз:
  - `memory/examples/test_performance_metrics.rs` (DIPerformanceMetrics)
  - `memory/src/di_container.rs` (DIPerformanceMetrics)
- **impl Default for FlushConfig** встречается 2 раз:
  - `memory/src/flush_config.rs` (FlushConfig)
  - `memory/tests/test_flush_config.rs` (FlushConfig)
- **impl Default for RetryConfig** встречается 4 раз:
  - `cli/src/services/resilience.rs` (RetryConfig)
  - `llm/src/multi_provider.rs` (RetryConfig)
  - `memory/src/retry.rs` (RetryConfig)
  - ...и еще 1 мест
- **impl DefaultIntentAnalysisService** встречается 3 раз:
  - `cli/tests/test_services_intent_analysis.rs` (DefaultIntentAnalysisService)
  - `cli/tests/test_services_intent_analysis.rs` (DefaultIntentAnalysisService)
  - `cli/src/services/intent_analysis.rs` (DefaultIntentAnalysisService)

## 🎭 Реестр моков и заглушек

Всего найдено моков: **60**

### cli
- `MockAdminService` в cli/src/handlers/admin_handler.rs
- `MockCircuitBreaker` в cli/src/handlers/admin_handler.rs
- `MockHealthCheck` в cli/src/health_checks.rs
- `MockIntentAnalyzer` в cli/src/services/intent_analysis.rs
- `MockIntentAnalyzerAgent` в cli/tests/test_services_intent_analysis.rs
### memory
- `DependentTestService` в memory/tests/test_di_performance_comparison.rs
- `DummyLogger` в memory/tests/test_mocks_and_stubs.rs
- `FakeMemoryService` в memory/tests/test_mocks_and_stubs.rs
- `HeavyTestService` в memory/tests/test_di_performance_comparison.rs
- `MockBackupCoordinator` в memory/tests/test_orchestration_memory_orchestrator.rs
### router
- `MockTool` в router/tests/test_router_async.rs
### tools
- `MockTool` в tools/tests/test_registry.rs
- `impl MockTool` в tools/tests/test_registry.rs

## 🛠️ Тестовые утилиты и билдеры

### memory
- `TestConfigBuilder` в memory/tests/common/mod.rs
- `TestDataGenerator` в memory/tests/common/mod.rs
- `TestRecordBuilder` в memory/tests/common/mod.rs

