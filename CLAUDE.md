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

---

# AUTO-GENERATED ARCHITECTURE

*Last updated: 2025-08-06 критический анализ*
*Status: PRE-ALPHA - катастрофическое состояние кода*

**⚠️ КРИТИЧЕСКОЕ ПРЕДУПРЕЖДЕНИЕ:**
- 1054 потенциальных panic через .unwrap()
- 300+ warnings при каждой сборке
- Unsafe code без документации
- Массивные неиспользуемые импорты
- God Objects не декомпозированы

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
        AI_config[config<br/>S:AiConfig,EmbeddingConfig<br/>fn:default,production<br/>m:Default::default,AiConfig::production]
        AI_embeddings_bge_m3[embeddings_bge_m3<br/>S:BgeM3EmbeddingService,EmbeddingResult<br/>fn:new,embed<br/>m:BgeM3EmbeddingService::new,BgeM3EmbeddingService::embed]
        AI_embeddings_cpu[embeddings_cpu<br/>S:CpuEmbeddingService,OptimizedEmbeddingResult<br/>fn:new,embed<br/>m:CpuEmbeddingService::new,CpuEmbeddingService::embed]
        AI_embeddings_gpu[embeddings_gpu<br/>S:GpuEmbeddingService,PerformanceMetrics<br/>fn:tokens_per_second,cache_hit_rate<br/>m:PerformanceMetrics::tokens_per_second,PerformanceMetrics::cache_hit_rate]
        AI_errors[errors<br/>E:AiError<br/>fn:fmt,from<br/>m:AiError::fmt,AiError::from]
        AI_gpu_config[gpu_config<br/>S:GpuConfig,GpuInfo<br/>E:GpuProviderType<br/>fn:default,auto_optimized<br/>...+1]
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
        CLI_agent[agent]
        CLI_agent_tests[agent_tests<br/>TEST<br/>fn:create_test_message,test_unified_agent_initialization]:::testFile
        CLI_agent_traits[agent_traits<br/>S:IntentDecision,RequestContext<br/>T:IntentDecisionStrategy,FallbackStrategy<br/>E:AgentResponse,AdminResponse<br/>...+1]
        CLI_container_traits[container_traits<br/>S:ContainerStats,DependencyValidationReport<br/>T:ContainerTrait,ContainerResolver<br/>E:ServiceLifetime,DependencySeverity<br/>...+3]
        CLI_health_checks[health_checks<br/>MOCK<br/>S:HealthCheckResult,HealthCheckSystem<br/>T:HealthCheck<br/>...+5]:::mockFile
        CLI_lib[lib]
        CLI_main[main<br/>S:AnimatedIcon,Cli<br/>E:Commands<br/>fn:new,get_frame<br/>...+1]
        CLI_progress[progress<br/>S:ProgressConfig,AdaptiveSpinner<br/>E:ProgressType<br/>fn:config,create_spinner<br/>...+1]
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
        COMMON_config_base[config_base<br/>S:BatchConfigBase,TimeoutConfigBase<br/>T:ConfigComposition<br/>fn:default,small<br/>...+2]
        COMMON_errors[errors<br/>T:IsRetriable,IsRecoverable<br/>E:MagrayError,DatabaseError<br/>fn:is_retriable,is_recoverable<br/>...+2]
        COMMON_error_monitor[error_monitor<br/>S:ErrorMonitor,ErrorMonitorConfig<br/>fn:default,new<br/>m:Default::default,RateLimiter::new<br/>...+1]
        COMMON_lib[lib]
        COMMON_macros[macros<br/>MOCK<br/>S:TestConfig<br/>fn:test_config_default_macro<br/>...+2]:::mockFile
        COMMON_service_traits[service_traits<br/>MOCK<br/>S:RetryConfig,PoolStats<br/>T:BaseService,ConfigurableService<br/>...+4]:::mockFile
        COMMON_structured_logging[structured_logging<br/>S:StructuredLogEntry,ExecutionContext<br/>fn:default,on_event<br/>m:Default::default,JsonFormatter::on_event<br/>...+1]
        COMMON_test_logging_advanced[test_logging_advanced<br/>TEST<br/>fn:test_execution_context_default,test_execution_context_with_all_fields]:::testFile
        COMMON_test_structured_logging[test_structured_logging<br/>TEST<br/>fn:test_structured_log_entry_creation,test_execution_context]:::testFile
        COMMON_test_structured_logging_extended[test_structured_logging_extended<br/>TEST<br/>fn:test_structured_log_entry_full,test_structured_log_entry_minimal]:::testFile
    end

    subgraph DOMAIN[Domain Crate]
        DOMAIN_errors[errors<br/>E:DomainError,ErrorCategory<br/>fn:is_validation_error,is_business_rule_error<br/>m:DomainError::is_validation_error,DomainError::is_business_rule_error]
        DOMAIN_lib[lib]
        DOMAIN_memory_record[memory_record<br/>S:MemoryRecord<br/>fn:new,with_id<br/>m:MemoryRecord::new,MemoryRecord::with_id]
        DOMAIN_mod[mod]
        DOMAIN_record_id[record_id<br/>S:RecordId<br/>fn:new,from_string<br/>m:RecordId::new,RecordId::from_string]
        DOMAIN_access_pattern[access_pattern<br/>S:AccessPattern<br/>fn:new,with_first_access<br/>m:AccessPattern::new,AccessPattern::with_first_access]
        DOMAIN_layer_type[layer_type<br/>E:LayerType<br/>fn:as_str,description<br/>m:LayerType::as_str,LayerType::description]
        DOMAIN_mod[mod]
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
        LLM_health_monitor[health_monitor<br/>S:HealthMonitor,ProviderHealthStatus<br/>fn:default,new<br/>m:Default::default,HealthMonitor::new]
        LLM_mod[mod<br/>S:SmartOrchestrationEngine,OrchestrationConfig<br/>fn:default,default<br/>m:Default::default,Default::default]
        LLM_request_analyzer[request_analyzer<br/>S:RequestAnalyzer,AnalysisReport<br/>E:RequestComplexity,TaskPriority<br/>fn:new,analyze_complexity<br/>...+1]
        LLM_anthropic_provider[anthropic_provider<br/>S:AnthropicProvider,AnthropicRequest<br/>fn:new,with_timeout<br/>m:AnthropicProvider::new,AnthropicProvider::with_timeout]
        LLM_azure_provider[azure_provider<br/>S:AzureProvider,AzureRequest<br/>fn:new,get_model_capabilities<br/>m:AzureProvider::new,AzureProvider::get_model_capabilities]
        LLM_groq_provider[groq_provider<br/>S:GroqProvider,GroqRequest<br/>fn:new,get_model_capabilities<br/>m:GroqProvider::new,GroqProvider::get_model_capabilities]
        LLM_local_provider[local_provider<br/>S:LocalProvider,LocalRequest<br/>fn:new,with_timeout<br/>m:LocalProvider::new,LocalProvider::with_timeout]
        LLM_mod[mod<br/>S:LlmRequest,LlmResponse<br/>T:LlmProvider<br/>E:MessageRole,LatencyClass<br/>...+2]
        LLM_openai_provider[openai_provider<br/>MOCK<br/>S:OpenAIProvider,OpenAIRequest<br/>fn:new,with_timeout<br/>...+2]:::mockFile
    end

    subgraph MEMORY[3-Layer HNSW Memory]
        MEMORY_comprehensive_performance[comprehensive_performance<br/>BENCH<br/>fn:create_test_record,bench_vector_store_operations]:::benchFile
        MEMORY_di_performance[di_performance<br/>BENCH<br/>S:LightweightService,HeavyService<br/>fn:new,new<br/>...+1]:::benchFile
        MEMORY_hnsw_performance_baseline[hnsw_performance_baseline<br/>BENCH<br/>S:HnswConfig,SimpleHnsw<br/>fn:baseline,optimized<br/>...+2]:::benchFile
        MEMORY_scalability_benchmarks[scalability_benchmarks<br/>BENCH<br/>fn:generate_embedding,bench_search_scalability]:::benchFile
        MEMORY_simple_test[simple_test<br/>TEST<br/>BENCH<br/>fn:simple_benchmark]:::testFile
        MEMORY_vector_benchmarks[vector_benchmarks<br/>BENCH<br/>fn:generate_random_vectors,create_test_records<br/>unsafe:2]:::benchFile
        MEMORY_benchmark_hnsw_vs_linear[benchmark_hnsw_vs_linear<br/>EXAMPLE<br/>fn:main,cosine_distance]:::exampleFile
        MEMORY_check_ort_version[check_ort_version<br/>EXAMPLE<br/>fn:main]:::exampleFile
        MEMORY_comprehensive_performance_validation[comprehensive_performance_validation<br/>EXAMPLE<br/>S:PerformanceResults<br/>fn:print_comprehensive_report,generate_test_vectors<br/>...+2]:::exampleFile
        MEMORY_debug_simd_performance[debug_simd_performance<br/>EXAMPLE<br/>fn:main]:::exampleFile
        MEMORY_direct_hnsw_benchmark[direct_hnsw_benchmark<br/>EXAMPLE<br/>fn:generate_vectors,cosine_distance_avx2<br/>unsafe:1]:::exampleFile
        MEMORY_di_best_practices[di_best_practices<br/>EXAMPLE<br/>fn:main,create_optimized_config]:::exampleFile
        MEMORY_final_simd_integration_test[final_simd_integration_test<br/>TEST<br/>EXAMPLE<br/>fn:main,simulate_hnsw_workload<br/>...+1]:::testFile
        MEMORY_full_pipeline_test[full_pipeline_test<br/>TEST<br/>EXAMPLE<br/>fn:main]:::testFile
        MEMORY_api[api<br/>S:UnifiedMemoryAPI,MemoryContext<br/>T:MemoryServiceTrait<br/>fn:search_sync,run_promotion_sync<br/>...+1]
        MEMORY_backup[backup<br/>S:BackupMetadata,LayerInfo<br/>fn:new,create_backup<br/>m:BackupManager::new,BackupManager::create_backup]
        MEMORY_batch_manager[batch_manager<br/>S:BatchConfig,BatchStats<br/>fn:default,production<br/>m:Default::default,BatchConfig::production]
        MEMORY_batch_optimized[batch_optimized<br/>S:BatchOptimizedConfig,BatchOptimizedStats<br/>E:BatchRequest<br/>fn:default,throughput_qps<br/>...+2]
        MEMORY_cache_interface[cache_interface<br/>T:EmbeddingCacheInterface<br/>fn:is_null_check,get<br/>m:EmbeddingCacheInterface::get,EmbeddingCacheInterface::insert]
        MEMORY_cache_lru[cache_lru<br/>S:CachedEmbedding,CacheConfig<br/>fn:default,production<br/>m:Default::default,ConfigTrait::production]
        MEMORY_cache_migration[cache_migration<br/>fn:migrate_cache_to_lru,recommend_cache_config]
        MEMORY_database_manager[database_manager<br/>S:DatabaseManager,DatabaseStats<br/>fn:new,global<br/>m:DatabaseManager::new,DatabaseManager::global]
        MEMORY_integration_full_workflow[integration_full_workflow<br/>TEST<br/>fn:test_complete_memory_system_workflow,test_performance_under_load]:::testFile
        MEMORY_integration_test[integration_test<br/>TEST<br/>fn:test_memory_service_basic_operations,test_memory_layers]:::testFile
        MEMORY_performance_test[performance_test<br/>TEST<br/>fn:generate_embedding,test_vector_search_performance]:::testFile
        MEMORY_test_basic_isolated[test_basic_isolated<br/>TEST<br/>fn:test_uuid_creation,test_datetime_creation]:::testFile
        MEMORY_test_batch_optimized[test_batch_optimized<br/>TEST<br/>fn:create_test_processor,create_test_record]:::testFile
        MEMORY_test_cache_migration[test_cache_migration<br/>TEST<br/>fn:test_cache_config_recommendations,test_recommended_config_cache_creation]:::testFile
        MEMORY_test_cache_service[test_cache_service<br/>TEST<br/>MOCK<br/>fn:create_test_container,create_mock_coordinator_service<br/>...+1]:::testFile
        MEMORY_test_comprehensive_core[test_comprehensive_core<br/>TEST<br/>MOCK<br/>S:MockVectorStore<br/>...+4]:::testFile
        MEMORY_container_builder[container_builder<br/>MOCK<br/>S:DIContainerBuilder,DIContainer<br/>fn:new,with_validation<br/>...+2]:::mockFile
        MEMORY_container_core[container_core<br/>S:ContainerCore<br/>fn:new,get_type_name<br/>m:ContainerCore::new,ContainerCore::get_type_name<br/>...+1]
        MEMORY_dependency_validator[dependency_validator<br/>S:DependencyGraph,DependencyGraphStats<br/>fn:new,add_dependency<br/>m:DependencyGraph::new,DependencyGraph::add_dependency]
        MEMORY_lifetime_manager[lifetime_manager<br/>MOCK<br/>S:LifetimeManagerImpl,CacheStats<br/>T:LifetimeStrategy<br/>...+3]:::mockFile
        MEMORY_metrics_collector[metrics_collector<br/>S:AtomicCounters,MetricsReporterImpl<br/>fn:new,reset<br/>m:AtomicCounters::new,AtomicCounters::reset]
        MEMORY_mod[mod<br/>MOCK<br/>S:TestService,DependentService<br/>fn:create_default_container,create_minimal_container<br/>...+2]:::mockFile
        MEMORY_traits[traits<br/>S:DIContainerStats,DIPerformanceMetrics<br/>T:DIResolver,DIRegistrar<br/>E:Lifetime<br/>...+2]
        MEMORY_config[config<br/>S:HnswConfig<br/>fn:default,high_quality<br/>m:Default::default,HnswConfig::high_quality]
        MEMORY_index[index<br/>S:VectorIndex<br/>fn:cosine_distance_avx2,horizontal_sum_avx2<br/>m:VectorIndex::new,VectorIndex::detect_simd_capabilities<br/>...+1]
        MEMORY_mod[mod]
        MEMORY_stats[stats<br/>S:HnswStats,HnswStatsSnapshot<br/>fn:new,record_search<br/>m:HnswStats::new,HnswStats::record_search]
        MEMORY_cache[cache<br/>S:LRUCacheLayer,CacheStorage<br/>fn:new,create_cache_key<br/>m:LRUCacheLayer::new,LRUCacheLayer::create_cache_key]
        MEMORY_index[index<br/>S:HNSWIndexLayer,InternalIndexStats<br/>E:IndexStatType<br/>fn:new,create_index_for_layer<br/>...+1]
        MEMORY_mod[mod<br/>S:LayeredMemoryBuilder,StorageConfig<br/>fn:default,default<br/>m:Default::default,Default::default]
        MEMORY_orchestrator[orchestrator<br/>S:LayeredDIContainer,LayerOrchestrator<br/>fn:new,storage<br/>m:LayeredDIContainer::new,LayeredDIContainer::storage]
        MEMORY_query[query<br/>S:SemanticQueryLayer<br/>fn:new,semantic_search<br/>m:SemanticQueryLayer::new,QueryLayer::semantic_search]
        MEMORY_storage[storage<br/>S:SqliteStorageLayer,InternalStorageStats<br/>E:StatType<br/>fn:new,run_migrations<br/>...+2]
        MEMORY_traits[traits<br/>S:VectorSearchResult,StorageStats<br/>T:StorageLayer,IndexLayer<br/>fn:default,test_ranking_criteria_default<br/>...+1]
        MEMORY_algorithms[algorithms<br/>S:FrequencyAlgorithm,FrequencyWeights<br/>fn:default,predict_score<br/>m:Default::default,PromotionAlgorithm::predict_score]
        MEMORY_coordinator[coordinator<br/>MOCK<br/>S:PromotionCoordinator,PromotionCoordinatorBuilder<br/>fn:new,production<br/>...+3]:::mockFile
        MEMORY_data_processor[data_processor<br/>S:MLDataProcessor,DataProcessorConfig<br/>fn:default,default<br/>m:Default::default,Default::default]
        MEMORY_legacy_facade[legacy_facade<br/>S:MLPromotionEngine,UsageTracker<br/>fn:new,run_ml_promotion_cycle<br/>m:MLPromotionEngine::new,MLPromotionEngine::run_ml_promotion_cycle]
        MEMORY_metrics[metrics<br/>S:MLPromotionMetricsCollector,MetricsConfig<br/>fn:default,production<br/>m:Default::default,MetricsConfig::production]
        MEMORY_mod[mod<br/>S:MigrationHelper<br/>fn:create_production_coordinator,create_development_coordinator<br/>m:MigrationHelper::create_compatible_coordinator]
        MEMORY_rules_engine[rules_engine<br/>S:ConfigurableRulesEngine,RulesConfig<br/>E:SpecialCondition,BusinessRule<br/>fn:default,production<br/>...+1]
        MEMORY_traits[traits<br/>S:TrainingExample,InferenceResult<br/>T:PromotionAlgorithm,PromotionMetrics<br/>fn:default<br/>...+1]
        MEMORY_backup_coordinator[backup_coordinator<br/>S:BackupCoordinator<br/>fn:new,initialize<br/>m:BackupCoordinator::new,Coordinator::initialize]
        MEMORY_circuit_breaker_manager[circuit_breaker_manager<br/>S:CircuitBreakerManager,CircuitBreakerConfig<br/>T:CircuitBreakerManagerTrait<br/>E:CircuitBreakerStatus<br/>...+2]
        MEMORY_coordinator_registry[coordinator_registry<br/>MOCK<br/>S:CoordinatorRegistry,ReadinessStatus<br/>T:CoordinatorRegistryTrait<br/>...+3]:::mockFile
        MEMORY_embedding_coordinator[embedding_coordinator<br/>S:EmbeddingCoordinator,CircuitBreaker<br/>E:CircuitState<br/>fn:new,with_retry_policy<br/>...+1]
        MEMORY_health_manager[health_manager<br/>S:HealthManager,HealthMetrics<br/>E:AlertLevel<br/>fn:new,setup_production_monitoring<br/>...+1]
        MEMORY_memory_orchestrator[memory_orchestrator<br/>S:MemoryOrchestrator,CircuitBreakerState<br/>E:CircuitBreakerStatus<br/>fn:clone,new<br/>...+1]
        MEMORY_metrics_collector[metrics_collector<br/>S:MetricsCollector,MetricsCoordinatorRegistry<br/>T:MetricsCollectorTrait<br/>E:ThroughputTrend<br/>...+2]
        MEMORY_mod[mod]
        MEMORY_cache_service[cache_service<br/>S:CacheService,CacheDetailedStats<br/>fn:new,new_with_coordinator<br/>m:CacheService::new,CacheService::new_with_coordinator]
        MEMORY_coordinator_service[coordinator_service<br/>S:CoordinatorRefs,CoordinatorService<br/>fn:default,new<br/>m:Default::default,CoordinatorService::new]
        MEMORY_core_memory_service[core_memory_service<br/>S:CoreMemoryService<br/>fn:new,new_minimal<br/>m:CoreMemoryService::new,CoreMemoryService::new_minimal]
        MEMORY_mod[mod]
        MEMORY_monitoring_service[monitoring_service<br/>S:MonitoringService<br/>fn:new,new_with_coordinator<br/>m:MonitoringService::new,MonitoringService::new_with_coordinator]
        MEMORY_refactored_di_memory_service[refactored_di_memory_service<br/>S:RefactoredDIMemoryService,LifecycleManager<br/>fn:default,new<br/>m:Default::default,RefactoredDIMemoryService::new]
        MEMORY_resilience_service[resilience_service<br/>S:CircuitBreakerState,ResilienceService<br/>fn:default,new<br/>m:Default::default,ResilienceService::new]
        MEMORY_service_factory[service_factory<br/>S:ServiceFactory,ServiceCollection<br/>fn:default,production<br/>m:Default::default,ServiceFactoryConfig::production]
        MEMORY_circuit_breaker[circuit_breaker<br/>S:CircuitBreakerConfig,CircuitBreakerInternalState<br/>E:CircuitBreakerState<br/>fn:default,default<br/>...+1]
        MEMORY_coordinator_factory[coordinator_factory<br/>MOCK<br/>S:OrchestrationCoordinators,ProductionCoordinatorFactory<br/>T:CoordinatorFactory<br/>...+3]:::mockFile
        MEMORY_facade[facade<br/>S:DIMemoryServiceFacade,DIMemoryServiceBuilder<br/>fn:new,new_minimal<br/>m:DIMemoryServiceFacade::new,DIMemoryServiceFacade::new_minimal]
        MEMORY_lifecycle_manager[lifecycle_manager<br/>S:LifecycleConfig,OperationStats<br/>E:LifecycleState<br/>fn:default,production<br/>...+1]
        MEMORY_mod[mod<br/>S:MemorySystemStats<br/>fn:default<br/>m:Default::default]
        MEMORY_operation_executor[operation_executor<br/>S:BatchInsertResult,BatchSearchResult<br/>T:OperationExecutor<br/>fn:default,production<br/>...+1]
        MEMORY_production_monitoring[production_monitoring<br/>S:ProductionMetrics,ProductionMetricsCollector<br/>T:MetricsCollector<br/>fn:success_rate,failure_rate<br/>...+1]
        MEMORY_service_config[service_config<br/>S:MemoryServiceConfig,MemoryServiceConfigBuilder<br/>T:ServiceConfigFactory<br/>E:ServiceConfigType<br/>...+2]
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
        TOOLS_mod[mod]
        TOOLS_pipeline[pipeline<br/>S:ExecutionContext,ExecutionResult<br/>E:SecurityEventType,SecuritySeverity<br/>fn:default,default<br/>...+1]
        TOOLS_resource_manager[resource_manager<br/>S:ResourceAllocation,ResourceLimits<br/>fn:default,is_within_limits<br/>m:Default::default,ResourceUsage::is_within_limits]
        TOOLS_security_enforcer[security_enforcer<br/>S:SecurityConfig,ExecutionPermission<br/>E:SecurityRestriction,FileSystemMode<br/>fn:default,new<br/>...+1]
        TOOLS_external_process[external_process<br/>S:ProcessConfig,ProcessResourceLimits<br/>E:StdinMode,StdoutMode<br/>fn:default,new<br/>...+1]
        TOOLS_hot_reload[hot_reload<br/>S:FileWatcher,HotReloadManager<br/>T:ReloadHandler<br/>E:ReloadEvent,ReloadPolicy<br/>...+2]
        TOOLS_mod[mod]
        TOOLS_plugin_manager[plugin_manager<br/>S:PluginMetadata,PluginVersion<br/>T:PluginInstance,PluginLoader<br/>E:PluginType,PluginState<br/>...+2]
        TOOLS_wasm_plugin[wasm_plugin<br/>S:WasmConfig,WasmResourceLimits<br/>T:HostFunction<br/>E:WasmPluginError<br/>...+2]
        TOOLS_mod[mod]
        TOOLS_secure_registry[secure_registry<br/>S:SecurityContext,UserPermissions<br/>E:UserTrustLevel,AuditEventType<br/>fn:default,validate_and_sanitize<br/>...+1]
        TOOLS_tool_metadata[tool_metadata<br/>S:ToolMetadata,SemanticVersion<br/>E:ToolCategory,FileSystemPermissions<br/>fn:new,is_compatible<br/>...+1]
    end

    %% Зависимости между крейтами
    CLI -.->|uses| ROUTER
    CLI -.->|uses| AI
    CLI -.->|uses| MEMORY
    CLI -.->|uses| LLM
    CLI -.->|uses| COMMON
    CLI -.->|uses| TOOLS
    MEMORY -.->|uses| COMMON
    MEMORY -.->|uses| AI
    ROUTER -.->|uses| LLM
    ROUTER -.->|uses| TOOLS
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

## 📝 MEMORY

## 🚨 КРИТИЧЕСКАЯ ПАМЯТКА ПРОЕКТА

**ПРОЕКТ В КАТАСТРОФИЧЕСКОМ СОСТОЯНИИ:**
- ❌ **1054 .unwrap() вызовов** - каждый может привести к panic
- ❌ **300+ warnings** загрязняют каждую сборку
- ❌ **Unsafe код** без documentation и justification  
- ❌ **God Objects** продолжают существовать
- ❌ **Моки вместо тестов** - тестирование отсутствует
- ❌ **Domain/Application слои** - пустые заглушки
- ❌ **Clean Architecture** - только структура директорий

**НЕМЕДЛЕННЫЕ ПРИОРИТЕТЫ:**
1. Устранить небезопасные .unwrap() паттерны
2. Исправить все compilation warnings
3. Удалить неиспользуемый код (dead code)
4. Заменить async fn в traits на proper Future returns
5. Документировать или удалить unsafe блоки

**ЗАПРЕЩЕНО:**
- ❌ Добавлять новую функциональность без исправления существующих проблем
- ❌ Игнорировать warnings и dead code  
- ❌ Использовать .unwrap() в новом коде
- ❌ Создавать новые God Objects
- ❌ Писать моки вместо реальных тестов
