graph TD
    %% Project architecture diagram
    classDef default fill:#f9f9f9,stroke:#333,stroke-width:2px
    classDef core fill:#ff6b6b,stroke:#d63031,stroke-width:2px
    classDef service fill:#4ecdc4,stroke:#00b894,stroke-width:2px
    classDef utility fill:#ffe66d,stroke:#fdcb6e,stroke-width:2px
    classDef config fill:#a29bfe,stroke:#6c5ce7,stroke-width:2px

    root_crates_cli_src_commands["root/crates/cli/src/commands"]
    class root_crates_cli_src_commands core
    root_crates_memory_examples_test_gpu_acceleration["root/crates/memory/examples/test_gpu_acceleration"]
    class root_crates_memory_examples_test_gpu_acceleration core
    root_crates_ai_src_embeddings_cpu["root/crates/ai/src/embeddings_cpu"]
    class root_crates_ai_src_embeddings_cpu core
    tokio["tokio"]
    class tokio core
    root_crates_memory_tests_integration_test["root/crates/memory/tests/integration_test"]
    class root_crates_memory_tests_integration_test core
    std::time::Instant["std::time::Instant"]
    class std::time::Instant core
    root_crates_ai_tests_test_reranker_integration["root/crates/ai/tests/test_reranker_integration"]
    class root_crates_ai_tests_test_reranker_integration core
    root_crates_memory_benches_vector_search_benchmarks["root/crates/memory/benches/vector_search_benchmarks"]
    class root_crates_memory_benches_vector_search_benchmarks core
    root_crates_memory_src_gpu_accelerated["root/crates/memory/src/gpu_accelerated"]
    class root_crates_memory_src_gpu_accelerated core
    tracing::info["tracing::info"]
    class tracing::info core
    async_trait::async_trait["async_trait::async_trait"]
    class async_trait::async_trait core
    root_crates_memory_src_service["root/crates/memory/src/service"]
    class root_crates_memory_src_service service
    reqwest["reqwest"]
    class reqwest core
    ai::{GpuEmbeddingService,_CpuEmbeddingService,_EmbeddingConfig}["ai::{GpuEmbeddingService, CpuEmbeddingService, EmbeddingConfig}"]
    class ai::{GpuEmbeddingService,_CpuEmbeddingService,_EmbeddingConfig} service
    anyhow::{Result,_Context}["anyhow::{Result, Context}"]
    class anyhow::{Result,_Context} core
    ai::embeddings_gpu::GpuEmbeddingService["ai::embeddings_gpu::GpuEmbeddingService"]
    class ai::embeddings_gpu::GpuEmbeddingService service
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer["root/crates/memory/models/bge-code-v1/quantize_with_real_tokenizer"]
    class root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer core
    root_crates_ai_examples_test_mxbai_real_tokenization["root/crates/ai/examples/test_mxbai_real_tokenization"]
    class root_crates_ai_examples_test_mxbai_real_tokenization core
    std::sync::{Arc,_RwLock}["std::sync::{Arc, RwLock}"]
    class std::sync::{Arc,_RwLock} core
    std::str["std::str"]
    class std::str core
    root_crates_memory_tests_test_promotion["root/crates/memory/tests/test_promotion"]
    class root_crates_memory_tests_test_promotion core
    root_crates_cli_src_commands_quantize["root/crates/cli/src/commands/quantize"]
    class root_crates_cli_src_commands_quantize core
    tokio::time::{sleep,_Duration}["tokio::time::{sleep, Duration}"]
    class tokio::time::{sleep,_Duration} core
    serde_json::Value["serde_json::Value"]
    class serde_json::Value core
    root_crates_ai_src_reranking["root/crates/ai/src/reranking"]
    class root_crates_ai_src_reranking core
    agent::{UnifiedAgent,_AgentResponse}["agent::{UnifiedAgent, AgentResponse}"]
    class agent::{UnifiedAgent,_AgentResponse} core
    anyhow::Result["anyhow::Result"]
    class anyhow::Result core
    ai::{["ai::{"]
    class ai::{ core
    ai::reranker_mxbai::BgeRerankerService["ai::reranker_mxbai::BgeRerankerService"]
    class ai::reranker_mxbai::BgeRerankerService service
    crate::LlmClient["crate::LlmClient"]
    class crate::LlmClient core
    tokio::sync::mpsc["tokio::sync::mpsc"]
    class tokio::sync::mpsc core
    root_crates_router_src_tests["root/crates/router/src/tests"]
    class root_crates_router_src_tests core
    std::fs::File["std::fs::File"]
    class std::fs::File core
    root_crates_todo_src_types["root/crates/todo/src/types"]
    class root_crates_todo_src_types core
    sentence_transformers["sentence_transformers"]
    class sentence_transformers core
    memory::{MemoryService,_MemoryConfig,_Layer,_Record}["memory::{MemoryService, MemoryConfig, Layer, Record}"]
    class memory::{MemoryService,_MemoryConfig,_Layer,_Record} service
    memory::MemRef["memory::MemRef"]
    class memory::MemRef core
    anyhow::Result_as_AnyhowResult["anyhow::Result as AnyhowResult"]
    class anyhow::Result_as_AnyhowResult core
    root_crates_memory_models_bge_code_v1_mixed_precision_quantize["root/crates/memory/models/bge-code-v1/mixed_precision_quantize"]
    class root_crates_memory_models_bge_code_v1_mixed_precision_quantize core
    anyhow::{anyhow,_Result}["anyhow::{anyhow, Result}"]
    class anyhow::{anyhow,_Result} core
    dashmap::DashMap["dashmap::DashMap"]
    class dashmap::DashMap core
    root_crates_ai_src_quantization_stack_dataset["root/crates/ai/src/quantization/stack_dataset"]
    class root_crates_ai_src_quantization_stack_dataset core
    root_crates_ai_src_tokenization_tests["root/crates/ai/src/tokenization/tests"]
    class root_crates_ai_src_tokenization_tests core
    json["json"]
    class json core
    root_crates_todo_src_graph_v2["root/crates/todo/src/graph_v2"]
    class root_crates_todo_src_graph_v2 core
    glob["glob"]
    class glob core
    parking_lot::RwLock["parking_lot::RwLock"]
    class parking_lot::RwLock core
    std::sync::{Arc,_Mutex}["std::sync::{Arc, Mutex}"]
    class std::sync::{Arc,_Mutex} core
    super::adaround::{AdaRoundConfig,_QuantizationResult}["super::adaround::{AdaRoundConfig, QuantizationResult}"]
    class super::adaround::{AdaRoundConfig,_QuantizationResult} config
    root_crates_llm_src_agents_parameter_extractor["root/crates/llm/src/agents/parameter_extractor"]
    class root_crates_llm_src_agents_parameter_extractor core
    ort::execution_providers::{CUDAExecutionProvider,_TensorRTExecutionProvider,_ExecutionProviderDispatch}["ort::execution_providers::{CUDAExecutionProvider, TensorRTExecutionProvider, ExecutionProviderDispatch}"]
    class ort::execution_providers::{CUDAExecutionProvider,_TensorRTExecutionProvider,_ExecutionProviderDispatch} core
    tracing::{info,_debug,_warn}["tracing::{info, debug, warn}"]
    class tracing::{info,_debug,_warn} core
    std::hash::{Hash,_Hasher}["std::hash::{Hash, Hasher}"]
    class std::hash::{Hash,_Hasher} core
    root_crates_memory_examples_tests["root/crates/memory/examples/tests"]
    class root_crates_memory_examples_tests core
    std::path::{Path,_PathBuf}["std::path::{Path, PathBuf}"]
    class std::path::{Path,_PathBuf} core
    root_crates_memory_src_api["root/crates/memory/src/api"]
    class root_crates_memory_src_api service
    crate::embeddings_gpu::GpuEmbeddingService["crate::embeddings_gpu::GpuEmbeddingService"]
    class crate::embeddings_gpu::GpuEmbeddingService service
    root_crates_memory_tests_test_metrics_integration["root/crates/memory/tests/test_metrics_integration"]
    class root_crates_memory_tests_test_metrics_integration core
    lru::LruCache["lru::LruCache"]
    class lru::LruCache core
    crate::{Result,_TokenizerService,_RerankingConfig,_models::OnnxSession}["crate::{Result, TokenizerService, RerankingConfig, models::OnnxSession}"]
    class crate::{Result,_TokenizerService,_RerankingConfig,_models::OnnxSession} service
    std::path::Path["std::path::Path"]
    class std::path::Path core
    rusqlite::{params,_Connection,_OptionalExtension}["rusqlite::{params, Connection, OptionalExtension}"]
    class rusqlite::{params,_Connection,_OptionalExtension} core
    tqdm["tqdm"]
    class tqdm core
    root_crates_memory_src_cache_migration["root/crates/memory/src/cache_migration"]
    class root_crates_memory_src_cache_migration core
    tracing::{info,_Level}["tracing::{info, Level}"]
    class tracing::{info,_Level} core
    root_crates_llm_src_agents_tool_selector["root/crates/llm/src/agents/tool_selector"]
    class root_crates_llm_src_agents_tool_selector utility
    root_crates_ai_src_embeddings_bge_m3["root/crates/ai/src/embeddings_bge_m3"]
    class root_crates_ai_src_embeddings_bge_m3 core
    criterion::{black_box,_criterion_group,_criterion_main,_Criterion,_BenchmarkId}["criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId}"]
    class criterion::{black_box,_criterion_group,_criterion_main,_Criterion,_BenchmarkId} core
    crate::graph_v2::{DependencyGraphV2,_GraphStats}["crate::graph_v2::{DependencyGraphV2, GraphStats}"]
    class crate::graph_v2::{DependencyGraphV2,_GraphStats} core
    std::fs["std::fs"]
    class std::fs core
    ai::{AiConfig,_EmbeddingConfig,_RerankingConfig,_GpuConfig}["ai::{AiConfig, EmbeddingConfig, RerankingConfig, GpuConfig}"]
    class ai::{AiConfig,_EmbeddingConfig,_RerankingConfig,_GpuConfig} config
    super::*["super::*"]
    class super::* core
    root_crates_memory_models_bge_code_v1_test_fp16_quality["root/crates/memory/models/bge-code-v1/test_fp16_quality"]
    class root_crates_memory_models_bge_code_v1_test_fp16_quality core
    root_crates_ai_src_embeddings_gpu["root/crates/ai/src/embeddings_gpu"]
    class root_crates_ai_src_embeddings_gpu core
    crate::cache_interface::EmbeddingCacheInterface["crate::cache_interface::EmbeddingCacheInterface"]
    class crate::cache_interface::EmbeddingCacheInterface core
    root_crates_memory_src_cache["root/crates/memory/src/cache"]
    class root_crates_memory_src_cache core
    hnsw_rs::hnsw::*["hnsw_rs::hnsw::*"]
    class hnsw_rs::hnsw::* core
    root_crates_memory_models_bge_code_v1_simple_best_quantize["root/crates/memory/models/bge-code-v1/simple_best_quantize"]
    class root_crates_memory_models_bge_code_v1_simple_best_quantize core
    ndarray::{Array1,_Array2,_ArrayView1,_ArrayView2,_Axis}["ndarray::{Array1, Array2, ArrayView1, ArrayView2, Axis}"]
    class ndarray::{Array1,_Array2,_ArrayView1,_ArrayView2,_Axis} core
    root_crates_todo_src_store_v2["root/crates/todo/src/store_v2"]
    class root_crates_todo_src_store_v2 core
    pandas_as_pd["pandas as pd"]
    class pandas_as_pd core
    ort::{inputs,_session::Session,_value::Tensor}["ort::{inputs, session::Session, value::Tensor}"]
    class ort::{inputs,_session::Session,_value::Tensor} core
    tracing::{info,_debug}["tracing::{info, debug}"]
    class tracing::{info,_debug} core
    root_crates_memory_models_bge_code_v1_modern_quantize_2025["root/crates/memory/models/bge-code-v1/modern_quantize_2025"]
    class root_crates_memory_models_bge_code_v1_modern_quantize_2025 core
    llm::{LlmClient,_IntentAnalyzerAgent}["llm::{LlmClient, IntentAnalyzerAgent}"]
    class llm::{LlmClient,_IntentAnalyzerAgent} core
    ai::{AiConfig,_ModelLoader,_RerankingService}["ai::{AiConfig, ModelLoader, RerankingService}"]
    class ai::{AiConfig,_ModelLoader,_RerankingService} service
    crate::memory_pool::{GLOBAL_MEMORY_POOL,_PoolStats}["crate::memory_pool::{GLOBAL_MEMORY_POOL, PoolStats}"]
    class crate::memory_pool::{GLOBAL_MEMORY_POOL,_PoolStats} core
    tokenizers::pre_tokenizers::bert::BertPreTokenizer["tokenizers::pre_tokenizers::bert::BertPreTokenizer"]
    class tokenizers::pre_tokenizers::bert::BertPreTokenizer core
    crate::metrics::MetricsCollector["crate::metrics::MetricsCollector"]
    class crate::metrics::MetricsCollector core
    tokio::runtime::Runtime["tokio::runtime::Runtime"]
    class tokio::runtime::Runtime core
    crate::{Tool,_ToolInput,_ToolOutput,_ToolSpec}["crate::{Tool, ToolInput, ToolOutput, ToolSpec}"]
    class crate::{Tool,_ToolInput,_ToolOutput,_ToolSpec} utility
    jsonschema::{Draft,_JSONSchema}["jsonschema::{Draft, JSONSchema}"]
    class jsonschema::{Draft,_JSONSchema} core
    serde_json["serde_json"]
    class serde_json core
    llm::{LlmClient,_ActionPlannerAgent,_ToolSelectorAgent,_ParameterExtractorAgent}["llm::{LlmClient, ActionPlannerAgent, ToolSelectorAgent, ParameterExtractorAgent}"]
    class llm::{LlmClient,_ActionPlannerAgent,_ToolSelectorAgent,_ParameterExtractorAgent} utility
    root_crates_ai_src_gpu_memory_pool["root/crates/ai/src/gpu_memory_pool"]
    class root_crates_ai_src_gpu_memory_pool core
    root_crates_ai_examples_quantize_with_stack["root/crates/ai/examples/quantize_with_stack"]
    class root_crates_ai_examples_quantize_with_stack core
    root_crates_ai_examples_test_real_tokenization["root/crates/ai/examples/test_real_tokenization"]
    class root_crates_ai_examples_test_real_tokenization core
    memory::{MemoryConfig,_MemoryService,_UnifiedMemoryAPI,_MemoryContext,_ApiSearchOptions,_Layer}["memory::{MemoryConfig, MemoryService, UnifiedMemoryAPI, MemoryContext, ApiSearchOptions, Layer}"]
    class memory::{MemoryConfig,_MemoryService,_UnifiedMemoryAPI,_MemoryContext,_ApiSearchOptions,_Layer} service
    onnx["onnx"]
    class onnx core
    root_crates_todo_src_tests["root/crates/todo/src/tests"]
    class root_crates_todo_src_tests core
    root_crates_memory_benches_vector_index_v3_benchmark["root/crates/memory/benches/vector_index_v3_benchmark"]
    class root_crates_memory_benches_vector_index_v3_benchmark core
    std::path::PathBuf["std::path::PathBuf"]
    class std::path::PathBuf core
    sled::Db["sled::Db"]
    class sled::Db core
    root_crates_todo_src_service_v2["root/crates/todo/src/service_v2"]
    class root_crates_todo_src_service_v2 service
    tokio_stream::StreamExt["tokio_stream::StreamExt"]
    class tokio_stream::StreamExt core
    std::cell::RefCell["std::cell::RefCell"]
    class std::cell::RefCell core
    root_crates_memory_examples_test_vector_index_clean["root/crates/memory/examples/test_vector_index_clean"]
    class root_crates_memory_examples_test_vector_index_clean core
    memory::{MemoryService,_MemoryConfig,_Record,_Layer}["memory::{MemoryService, MemoryConfig, Record, Layer}"]
    class memory::{MemoryService,_MemoryConfig,_Record,_Layer} service
    std::time::{SystemTime,_UNIX_EPOCH}["std::time::{SystemTime, UNIX_EPOCH}"]
    class std::time::{SystemTime,_UNIX_EPOCH} core
    root_crates_memory_src_vector_index_hnswlib["root/crates/memory/src/vector_index_hnswlib"]
    class root_crates_memory_src_vector_index_hnswlib core
    sha2::{Sha256,_Digest}["sha2::{Sha256, Digest}"]
    class sha2::{Sha256,_Digest} core
    root_crates_memory_tests_performance_test["root/crates/memory/tests/performance_test"]
    class root_crates_memory_tests_performance_test core
    root_crates_tools_src_shell_ops["root/crates/tools/src/shell_ops"]
    class root_crates_tools_src_shell_ops utility
    ndarray::{Array1,_Array2,_ArrayView1,_ArrayView2}["ndarray::{Array1, Array2, ArrayView1, ArrayView2}"]
    class ndarray::{Array1,_Array2,_ArrayView1,_ArrayView2} core
    root_crates_memory_examples_test_health_monitoring["root/crates/memory/examples/test_health_monitoring"]
    class root_crates_memory_examples_test_health_monitoring core
    crate::tokenization::{OptimizedTokenizer,_TokenizedInput_as_OptTokenizedInput,_BatchTokenized}["crate::tokenization::{OptimizedTokenizer, TokenizedInput as OptTokenizedInput, BatchTokenized}"]
    class crate::tokenization::{OptimizedTokenizer,_TokenizedInput_as_OptTokenizedInput,_BatchTokenized} core
    sklearn_model_selection["sklearn.model_selection"]
    class sklearn_model_selection core
    transformers["transformers"]
    class transformers core
    tracing::{info,_warn}["tracing::{info, warn}"]
    class tracing::{info,_warn} core
    root_crates_memory_models_the_stack_smol_xs_the_stack_smol_xs["root/crates/memory/models/the-stack-smol-xs/the-stack-smol-xs"]
    class root_crates_memory_models_the_stack_smol_xs_the_stack_smol_xs core
    ndarray::Array2["ndarray::Array2"]
    class ndarray::Array2 core
    crate::storage::VectorStore["crate::storage::VectorStore"]
    class crate::storage::VectorStore core
    root_crates_ai_src_tokenizer["root/crates/ai/src/tokenizer"]
    class root_crates_ai_src_tokenizer core
    time["time"]
    class time core
    root_crates_ai_src_gpu_config["root/crates/ai/src/gpu_config"]
    class root_crates_ai_src_gpu_config config
    crate::EmbeddingConfig["crate::EmbeddingConfig"]
    class crate::EmbeddingConfig config
    tokenizers::models::bpe::BPE["tokenizers::models::bpe::BPE"]
    class tokenizers::models::bpe::BPE core
    root_crates_memory_examples_vector_index_performance["root/crates/memory/examples/vector_index_performance"]
    class root_crates_memory_examples_vector_index_performance core
    qllm_nn_linear["qllm.nn.linear"]
    class qllm_nn_linear core
    logging["logging"]
    class logging core
    root_crates_ai_examples_quantize_bge_model["root/crates/ai/examples/quantize_bge_model"]
    class root_crates_ai_examples_quantize_bge_model core
    serde::{Deserialize,_Serialize}["serde::{Deserialize, Serialize}"]
    class serde::{Deserialize,_Serialize} core
    ort::{session::Session,_inputs,_value::Tensor}["ort::{session::Session, inputs, value::Tensor}"]
    class ort::{session::Session,_inputs,_value::Tensor} core
    tokio::time::sleep["tokio::time::sleep"]
    class tokio::time::sleep core
    root_crates_memory_examples_test_reranker["root/crates/memory/examples/test_reranker"]
    class root_crates_memory_examples_test_reranker core
    root_crates_ai_src_reranker_mxbai_optimized["root/crates/ai/src/reranker_mxbai_optimized"]
    class root_crates_ai_src_reranker_mxbai_optimized core
    crate::types::{TodoItem,_TaskState}["crate::types::{TodoItem, TaskState}"]
    class crate::types::{TodoItem,_TaskState} core
    root_scripts_ctl["root/scripts/ctl"]
    class root_scripts_ctl core
    root_crates_llm_src_agents_intent_analyzer["root/crates/llm/src/agents/intent_analyzer"]
    class root_crates_llm_src_agents_intent_analyzer core
    chrono::{Duration,_Utc}["chrono::{Duration, Utc}"]
    class chrono::{Duration,_Utc} core
    serde::{Serialize,_Deserialize}["serde::{Serialize, Deserialize}"]
    class serde::{Serialize,_Deserialize} core
    ort::{Session,_SessionBuilder}["ort::{Session, SessionBuilder}"]
    class ort::{Session,_SessionBuilder} core
    router::SmartRouter["router::SmartRouter"]
    class router::SmartRouter core
    std::collections::hash_map::DefaultHasher["std::collections::hash_map::DefaultHasher"]
    class std::collections::hash_map::DefaultHasher core
    std::sync::atomic::{AtomicUsize,_Ordering}["std::sync::atomic::{AtomicUsize, Ordering}"]
    class std::sync::atomic::{AtomicUsize,_Ordering} core
    root_crates_ai_examples_full_quantization_pipeline["root/crates/ai/examples/full_quantization_pipeline"]
    class root_crates_ai_examples_full_quantization_pipeline core
    std::env["std::env"]
    class std::env config
    root_crates_memory_examples_benchmark_hnsw_vs_linear["root/crates/memory/examples/benchmark_hnsw_vs_linear"]
    class root_crates_memory_examples_benchmark_hnsw_vs_linear core
    requests["requests"]
    class requests core
    chrono::{DateTime,_Utc}["chrono::{DateTime, Utc}"]
    class chrono::{DateTime,_Utc} core
    root_crates_llm_src_lib["root/crates/llm/src/lib"]
    class root_crates_llm_src_lib core
    tracing::{info,_error}["tracing::{info, error}"]
    class tracing::{info,_error} core
    r2d2::Pool["r2d2::Pool"]
    class r2d2::Pool core
    chrono::{DateTime,_Utc,_Duration}["chrono::{DateTime, Utc, Duration}"]
    class chrono::{DateTime,_Utc,_Duration} core
    tokenizers::Tokenizer["tokenizers::Tokenizer"]
    class tokenizers::Tokenizer core
    tracing::{info,_warn,_debug}["tracing::{info, warn, debug}"]
    class tracing::{info,_warn,_debug} core
    std::fmt["std::fmt"]
    class std::fmt core
    std::sync::Mutex["std::sync::Mutex"]
    class std::sync::Mutex core
    rand::seq::SliceRandom["rand::seq::SliceRandom"]
    class rand::seq::SliceRandom core
    ai::BgeM3EmbeddingService["ai::BgeM3EmbeddingService"]
    class ai::BgeM3EmbeddingService service
    crate::gpu_detector::{GpuDetector,_GpuOptimalParams}["crate::gpu_detector::{GpuDetector, GpuOptimalParams}"]
    class crate::gpu_detector::{GpuDetector,_GpuOptimalParams} core
    root_crates_ai_src_tokenization_mod["root/crates/ai/src/tokenization/mod"]
    class root_crates_ai_src_tokenization_mod core
    clap::Parser["clap::Parser"]
    class clap::Parser core
    ort::{session::Session,_value::Tensor,_inputs}["ort::{session::Session, value::Tensor, inputs}"]
    class ort::{session::Session,_value::Tensor,_inputs} core
    root_crates_ai_src_tensorrt_cache["root/crates/ai/src/tensorrt_cache"]
    class root_crates_ai_src_tensorrt_cache core
    root_crates_ai_src_gpu_detector["root/crates/ai/src/gpu_detector"]
    class root_crates_ai_src_gpu_detector core
    std::time::{Duration,_Instant}["std::time::{Duration, Instant}"]
    class std::time::{Duration,_Instant} core
    root_crates_memory_src_lib["root/crates/memory/src/lib"]
    class root_crates_memory_src_lib core
    root_crates_memory_src_types["root/crates/memory/src/types"]
    class root_crates_memory_src_types core
    crate::metrics::{MetricsCollector,_TimedOperation}["crate::metrics::{MetricsCollector, TimedOperation}"]
    class crate::metrics::{MetricsCollector,_TimedOperation} core
    tools::{ToolRegistry,_ToolInput,_ToolOutput}["tools::{ToolRegistry, ToolInput, ToolOutput}"]
    class tools::{ToolRegistry,_ToolInput,_ToolOutput} utility
    root_crates_todo_src_store["root/crates/todo/src/store"]
    class root_crates_todo_src_store core
    root_crates_ai_src_models["root/crates/ai/src/models"]
    class root_crates_ai_src_models core
    criterion::{black_box,_criterion_group,_criterion_main,_Criterion,_BenchmarkId,_Throughput}["criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput}"]
    class criterion::{black_box,_criterion_group,_criterion_main,_Criterion,_BenchmarkId,_Throughput} core
    crate::embeddings_cpu::CpuEmbeddingService["crate::embeddings_cpu::CpuEmbeddingService"]
    class crate::embeddings_cpu::CpuEmbeddingService service
    super::stack_dataset::{StackDatasetLoader,_StackSample,_QualityFilters}["super::stack_dataset::{StackDatasetLoader, StackSample, QualityFilters}"]
    class super::stack_dataset::{StackDatasetLoader,_StackSample,_QualityFilters} core
    numpy_as_np["numpy as np"]
    class numpy_as_np core
    anyhow::{Context,_Result}["anyhow::{Context, Result}"]
    class anyhow::{Context,_Result} core
    crate::reranker_mxbai_optimized::OptimizedRerankingService["crate::reranker_mxbai_optimized::OptimizedRerankingService"]
    class crate::reranker_mxbai_optimized::OptimizedRerankingService service
    tracing::{debug,_info,_warn}["tracing::{debug, info, warn}"]
    class tracing::{debug,_info,_warn} core
    commands::GpuCommand["commands::GpuCommand"]
    class commands::GpuCommand core
    std::collections::VecDeque["std::collections::VecDeque"]
    class std::collections::VecDeque core
    root_crates_memory_examples_memory_demo["root/crates/memory/examples/memory_demo"]
    class root_crates_memory_examples_memory_demo core
    memory::{VectorIndexHnswRs,_HnswRsConfig}["memory::{VectorIndexHnswRs, HnswRsConfig}"]
    class memory::{VectorIndexHnswRs,_HnswRsConfig} config
    torch["torch"]
    class torch core
    typing["typing"]
    class typing core
    root_crates_memory_src_cache_interface["root/crates/memory/src/cache_interface"]
    class root_crates_memory_src_cache_interface core
    root_crates_memory_tests_test_hnsw_comparison["root/crates/memory/tests/test_hnsw_comparison"]
    class root_crates_memory_tests_test_hnsw_comparison core
    std::io::{self,_Write}["std::io::{self, Write}"]
    class std::io::{self,_Write} core
    tracing::{info,_debug,_error}["tracing::{info, debug, error}"]
    class tracing::{info,_debug,_error} core
    root_crates_ai_src_quantization_onnx_quantizer["root/crates/ai/src/quantization/onnx_quantizer"]
    class root_crates_ai_src_quantization_onnx_quantizer core
    std::time::Duration["std::time::Duration"]
    class std::time::Duration core
    shutil["shutil"]
    class shutil utility
    crate::types::*["crate::types::*"]
    class crate::types::* core
    root_crates_cli_src_agent["root/crates/cli/src/agent"]
    class root_crates_cli_src_agent core
    qllm["qllm"]
    class qllm core
    root_crates_memory_examples_test_lru_cache["root/crates/memory/examples/test_lru_cache"]
    class root_crates_memory_examples_test_lru_cache core
    root_crates_ai_src_errors["root/crates/ai/src/errors"]
    class root_crates_ai_src_errors core
    root_scripts_ctl_converter["root/scripts/ctl_converter"]
    class root_scripts_ctl_converter core
    tempfile::TempDir["tempfile::TempDir"]
    class tempfile::TempDir core
    crate::vector_index_hnswlib::{VectorIndexHnswRs,_HnswRsConfig}["crate::vector_index_hnswlib::{VectorIndexHnswRs, HnswRsConfig}"]
    class crate::vector_index_hnswlib::{VectorIndexHnswRs,_HnswRsConfig} config
    root_crates_memory_benches_memory_benchmarks["root/crates/memory/benches/memory_benchmarks"]
    class root_crates_memory_benches_memory_benchmarks core
    crate::{["crate::{"]
    class crate::{ core
    petgraph::algo::toposort["petgraph::algo::toposort"]
    class petgraph::algo::toposort core
    thread_local::ThreadLocal["thread_local::ThreadLocal"]
    class thread_local::ThreadLocal core
    std::collections::HashMap["std::collections::HashMap"]
    class std::collections::HashMap core
    super::calibration::CalibrationDataset["super::calibration::CalibrationDataset"]
    class super::calibration::CalibrationDataset core
    root_crates_ai_src_quantization_optimizer["root/crates/ai/src/quantization/optimizer"]
    class root_crates_ai_src_quantization_optimizer core
    indicatif::{ProgressBar,_ProgressStyle}["indicatif::{ProgressBar, ProgressStyle}"]
    class indicatif::{ProgressBar,_ProgressStyle} core
    root_crates_memory_src_batch_manager["root/crates/memory/src/batch_manager"]
    class root_crates_memory_src_batch_manager core
    std::sync::RwLock["std::sync::RwLock"]
    class std::sync::RwLock core
    pathlib["pathlib"]
    class pathlib core
    notify::{Watcher,_RecursiveMode,_Result_as_NotifyResult,_RecommendedWatcher,_Event}["notify::{Watcher, RecursiveMode, Result as NotifyResult, RecommendedWatcher, Event}"]
    class notify::{Watcher,_RecursiveMode,_Result_as_NotifyResult,_RecommendedWatcher,_Event} core
    regex::Regex["regex::Regex"]
    class regex::Regex core
    tracing::{debug,_info}["tracing::{debug, info}"]
    class tracing::{debug,_info} core
    root_crates_memory_src_metrics["root/crates/memory/src/metrics"]
    class root_crates_memory_src_metrics core
    root_crates_ai_src_auto_device_selector["root/crates/ai/src/auto_device_selector"]
    class root_crates_ai_src_auto_device_selector core
    llm::LlmClient["llm::LlmClient"]
    class llm::LlmClient core
    ai::{RerankingConfig,_RerankingService}["ai::{RerankingConfig, RerankingService}"]
    class ai::{RerankingConfig,_RerankingService} service
    datetime["datetime"]
    class datetime core
    root_crates_cli_src_main["root/crates/cli/src/main"]
    class root_crates_cli_src_main core
    root_scripts_download_mxbai_tokenizer["root/scripts/download_mxbai_tokenizer"]
    class root_scripts_download_mxbai_tokenizer core
    root_scripts_generate_ctl_metrics["root/scripts/generate_ctl_metrics"]
    class root_scripts_generate_ctl_metrics core
    root_crates_ai_src_config["root/crates/ai/src/config"]
    class root_crates_ai_src_config config
    root_crates_llm_src_agents_action_planner["root/crates/llm/src/agents/action_planner"]
    class root_crates_llm_src_agents_action_planner core
    root_crates_memory_benches_scalability_benchmarks["root/crates/memory/benches/scalability_benchmarks"]
    class root_crates_memory_benches_scalability_benchmarks core
    onnxruntime_as_ort["onnxruntime as ort"]
    class onnxruntime_as_ort core
    sled::{Db,_Tree}["sled::{Db, Tree}"]
    class sled::{Db,_Tree} core
    tokio::process::Command["tokio::process::Command"]
    class tokio::process::Command core
    sys["sys"]
    class sys core
    memory::fallback::{GracefulEmbeddingService,_EmbeddingProvider}["memory::fallback::{GracefulEmbeddingService, EmbeddingProvider}"]
    class memory::fallback::{GracefulEmbeddingService,_EmbeddingProvider} service
    petgraph::graph::{DiGraph,_NodeIndex}["petgraph::graph::{DiGraph, NodeIndex}"]
    class petgraph::graph::{DiGraph,_NodeIndex} core
    chrono::{DateTime,_Duration,_Utc}["chrono::{DateTime, Duration, Utc}"]
    class chrono::{DateTime,_Duration,_Utc} core
    datasets["datasets"]
    class datasets core
    root_crates_router_src_lib["root/crates/router/src/lib"]
    class root_crates_router_src_lib core
    root_crates_memory_examples_test_unified_api["root/crates/memory/examples/test_unified_api"]
    class root_crates_memory_examples_test_unified_api service
    root_crates_memory_src_fallback["root/crates/memory/src/fallback"]
    class root_crates_memory_src_fallback core
    tracing::warn["tracing::warn"]
    class tracing::warn core
    root_crates_ai_tests_test_adaround_quantization["root/crates/ai/tests/test_adaround_quantization"]
    class root_crates_ai_tests_test_adaround_quantization core
    root_crates_memory_src_health["root/crates/memory/src/health"]
    class root_crates_memory_src_health core
    crate::{EmbeddingCache,_EmbeddingCacheLRU,_CacheConfig}["crate::{EmbeddingCache, EmbeddingCacheLRU, CacheConfig}"]
    class crate::{EmbeddingCache,_EmbeddingCacheLRU,_CacheConfig} config
    ai::AiConfig["ai::AiConfig"]
    class ai::AiConfig config
    root_crates_memory_examples_full_pipeline_test["root/crates/memory/examples/full_pipeline_test"]
    class root_crates_memory_examples_full_pipeline_test core
    rusqlite::{params,_Connection,_Row,_OptionalExtension}["rusqlite::{params, Connection, Row, OptionalExtension}"]
    class rusqlite::{params,_Connection,_Row,_OptionalExtension} core
    root_crates_todo_src_graph["root/crates/todo/src/graph"]
    class root_crates_todo_src_graph core
    root_crates_memory_src_migration["root/crates/memory/src/migration"]
    class root_crates_memory_src_migration core
    tokio::io::AsyncWriteExt["tokio::io::AsyncWriteExt"]
    class tokio::io::AsyncWriteExt core
    tokio::fs["tokio::fs"]
    class tokio::fs core
    root_crates_memory_models_bge_code_v1_test_quality["root/crates/memory/models/bge-code-v1/test_quality"]
    class root_crates_memory_models_bge_code_v1_test_quality core
    root_crates_memory_models_the_stack_smol_xs_dataset_creation["root/crates/memory/models/the-stack-smol-xs/dataset_creation"]
    class root_crates_memory_models_the_stack_smol_xs_dataset_creation core
    root_crates_ai_src_quantization_config["root/crates/ai/src/quantization/config"]
    class root_crates_ai_src_quantization_config config
    root_crates_memory_examples_test_memory_gpu_integration["root/crates/memory/examples/test_memory_gpu_integration"]
    class root_crates_memory_examples_test_memory_gpu_integration core
    hnsw_rs::prelude::*["hnsw_rs::prelude::*"]
    class hnsw_rs::prelude::* core
    parking_lot::{RwLock,_Mutex}["parking_lot::{RwLock, Mutex}"]
    class parking_lot::{RwLock,_Mutex} core
    super::config::{QuantizationConfig,_CalibrationMethod}["super::config::{QuantizationConfig, CalibrationMethod}"]
    class super::config::{QuantizationConfig,_CalibrationMethod} config
    crate::{AiError,_Result}["crate::{AiError, Result}"]
    class crate::{AiError,_Result} core
    walkdir::WalkDir["walkdir::WalkDir"]
    class walkdir::WalkDir core
    root_crates_ai_src_tests["root/crates/ai/src/tests"]
    class root_crates_ai_src_tests core
    root_crates_memory_src_storage["root/crates/memory/src/storage"]
    class root_crates_memory_src_storage core
    ai::quantization::{["ai::quantization::{"]
    class ai::quantization::{ core
    root_crates_ai_src_reranker_mxbai["root/crates/ai/src/reranker_mxbai"]
    class root_crates_ai_src_reranker_mxbai core
    root_crates_cli_src_commands_gpu["root/crates/cli/src/commands/gpu"]
    class root_crates_cli_src_commands_gpu core
    crate::store_v2::TodoStoreV2["crate::store_v2::TodoStoreV2"]
    class crate::store_v2::TodoStoreV2 core
    crate::{GpuConfig,_GpuInfo}["crate::{GpuConfig, GpuInfo}"]
    class crate::{GpuConfig,_GpuInfo} config
    rand::{Rng,_SeedableRng}["rand::{Rng, SeedableRng}"]
    class rand::{Rng,_SeedableRng} core
    memory::{MemoryService,_MemoryConfig,_Record,_Layer,_BatchProcessorStats}["memory::{MemoryService, MemoryConfig, Record, Layer, BatchProcessorStats}"]
    class memory::{MemoryService,_MemoryConfig,_Record,_Layer,_BatchProcessorStats} service
    std::process::Command["std::process::Command"]
    class std::process::Command core
    std::io::{BufReader,_BufRead}["std::io::{BufReader, BufRead}"]
    class std::io::{BufReader,_BufRead} core
    root_crates_memory_tests_test_two_stage_search["root/crates/memory/tests/test_two_stage_search"]
    class root_crates_memory_tests_test_two_stage_search core
    crate::gpu_detector::GpuDetector["crate::gpu_detector::GpuDetector"]
    class crate::gpu_detector::GpuDetector core
    root_docs_daemon_src_main["root/docs-daemon/src/main"]
    class root_docs_daemon_src_main core
    petgraph::Direction["petgraph::Direction"]
    class petgraph::Direction core
    root_crates_tools_src_lib["root/crates/tools/src/lib"]
    class root_crates_tools_src_lib utility
    re["re"]
    class re core
    crate::types::{Layer,_Record}["crate::types::{Layer, Record}"]
    class crate::types::{Layer,_Record} core
    argparse["argparse"]
    class argparse core
    std::collections::{HashMap,_VecDeque}["std::collections::{HashMap, VecDeque}"]
    class std::collections::{HashMap,_VecDeque} core
    memory::{Layer,_MemoryConfig,_MemoryService,_Record}["memory::{Layer, MemoryConfig, MemoryService, Record}"]
    class memory::{Layer,_MemoryConfig,_MemoryService,_Record} service
    root_crates_memory_models_bge_code_v1_embedding_specialized_quantize["root/crates/memory/models/bge-code-v1/embedding_specialized_quantize"]
    class root_crates_memory_models_bge_code_v1_embedding_specialized_quantize core
    tracing_subscriber["tracing_subscriber"]
    class tracing_subscriber core
    ort::{Session,_Value}["ort::{Session, Value}"]
    class ort::{Session,_Value} core
    std::sync::Arc["std::sync::Arc"]
    class std::sync::Arc core
    root_scripts_convert_annotations["root/scripts/convert_annotations"]
    class root_scripts_convert_annotations core
    tokio::sync::{Mutex,_Semaphore}["tokio::sync::{Mutex, Semaphore}"]
    class tokio::sync::{Mutex,_Semaphore} core
    tracing::{info,_warn,_error}["tracing::{info, warn, error}"]
    class tracing::{info,_warn,_error} core
    ort::{Session,_SessionBuilder,_GraphOptimizationLevel}["ort::{Session, SessionBuilder, GraphOptimizationLevel}"]
    class ort::{Session,_SessionBuilder,_GraphOptimizationLevel} core
    tokenizers::normalizers::BertNormalizer["tokenizers::normalizers::BertNormalizer"]
    class tokenizers::normalizers::BertNormalizer core
    clap::{Parser,_Subcommand}["clap::{Parser, Subcommand}"]
    class clap::{Parser,_Subcommand} core
    console::{style,_Term}["console::{style, Term}"]
    class console::{style,_Term} core
    crate::{AiError,_Result,_TokenizerService,_RerankingConfig}["crate::{AiError, Result, TokenizerService, RerankingConfig}"]
    class crate::{AiError,_Result,_TokenizerService,_RerankingConfig} service
    memory::{MemoryConfig,_MemoryService,_Layer,_Record}["memory::{MemoryConfig, MemoryService, Layer, Record}"]
    class memory::{MemoryConfig,_MemoryService,_Layer,_Record} service
    root_crates_memory_examples_test_graceful_degradation["root/crates/memory/examples/test_graceful_degradation"]
    class root_crates_memory_examples_test_graceful_degradation core
    onnxruntime_quantization["onnxruntime.quantization"]
    class onnxruntime_quantization core
    ai::gpu_detector::GpuDetector["ai::gpu_detector::GpuDetector"]
    class ai::gpu_detector::GpuDetector core
    tokio::time::interval["tokio::time::interval"]
    class tokio::time::interval core
    std::collections::BTreeMap["std::collections::BTreeMap"]
    class std::collections::BTreeMap core
    root_crates_todo_src_lib["root/crates/todo/src/lib"]
    class root_crates_todo_src_lib core
    r2d2_sqlite::SqliteConnectionManager["r2d2_sqlite::SqliteConnectionManager"]
    class r2d2_sqlite::SqliteConnectionManager core
    root_crates_ai_src_memory_pool["root/crates/ai/src/memory_pool"]
    class root_crates_ai_src_memory_pool core
    root_crates_tools_src_git_ops["root/crates/tools/src/git_ops"]
    class root_crates_tools_src_git_ops utility
    petgraph::algo::{toposort,_has_path_connecting}["petgraph::algo::{toposort, has_path_connecting}"]
    class petgraph::algo::{toposort,_has_path_connecting} core
    crate::RerankingConfig["crate::RerankingConfig"]
    class crate::RerankingConfig config
    memory::{["memory::{"]
    class memory::{ core
    parking_lot::Mutex["parking_lot::Mutex"]
    class parking_lot::Mutex core
    uuid::Uuid["uuid::Uuid"]
    class uuid::Uuid core
    std::sync::atomic::{AtomicU64,_Ordering}["std::sync::atomic::{AtomicU64, Ordering}"]
    class std::sync::atomic::{AtomicU64,_Ordering} core
    root_crates_memory_examples_test_reranker_mock["root/crates/memory/examples/test_reranker_mock"]
    class root_crates_memory_examples_test_reranker_mock core
    crate::model_downloader::ensure_model["crate::model_downloader::ensure_model"]
    class crate::model_downloader::ensure_model core
    sentence_transformers_quantization["sentence_transformers.quantization"]
    class sentence_transformers_quantization core
    parking_lot::{Mutex,_RwLock}["parking_lot::{Mutex, RwLock}"]
    class parking_lot::{Mutex,_RwLock} core
    tracing::{debug,_instrument}["tracing::{debug, instrument}"]
    class tracing::{debug,_instrument} core
    os["os"]
    class os core
    root_crates_memory_src_promotion["root/crates/memory/src/promotion"]
    class root_crates_memory_src_promotion core
    root_crates_tools_src_web_ops["root/crates/tools/src/web_ops"]
    class root_crates_tools_src_web_ops utility
    memory::{VectorIndexV3,_VectorIndexConfigV3}["memory::{VectorIndexV3, VectorIndexConfigV3}"]
    class memory::{VectorIndexV3,_VectorIndexConfigV3} config
    tracing::{info,_warn,_Level}["tracing::{info, warn, Level}"]
    class tracing::{info,_warn,_Level} core
    root_crates_memory_src_cache_lru["root/crates/memory/src/cache_lru"]
    class root_crates_memory_src_cache_lru core
    tokio::sync::Mutex["tokio::sync::Mutex"]
    class tokio::sync::Mutex core
    std::num::NonZeroUsize["std::num::NonZeroUsize"]
    class std::num::NonZeroUsize core
    root_crates_ai_src_quantization_adaround["root/crates/ai/src/quantization/adaround"]
    class root_crates_ai_src_quantization_adaround core
    rand::thread_rng["rand::thread_rng"]
    class rand::thread_rng core
    pickle["pickle"]
    class pickle core
    crate::{EmbeddingConfig,_GpuConfig}["crate::{EmbeddingConfig, GpuConfig}"]
    class crate::{EmbeddingConfig,_GpuConfig} config
    root_crates_tools_src_file_ops["root/crates/tools/src/file_ops"]
    class root_crates_tools_src_file_ops utility
    clap::{Args,_Subcommand}["clap::{Args, Subcommand}"]
    class clap::{Args,_Subcommand} core
    std::sync::mpsc::channel["std::sync::mpsc::channel"]
    class std::sync::mpsc::channel core
    subprocess["subprocess"]
    class subprocess core
    root_crates_ai_src_model_downloader["root/crates/ai/src/model_downloader"]
    class root_crates_ai_src_model_downloader core
    (["("]
    class ( core
    crate::tokenization::OptimizedTokenizer["crate::tokenization::OptimizedTokenizer"]
    class crate::tokenization::OptimizedTokenizer core
    memory::{VectorStore,_Layer,_Record,_VectorIndexHnswRs,_HnswRsConfig}["memory::{VectorStore, Layer, Record, VectorIndexHnswRs, HnswRsConfig}"]
    class memory::{VectorStore,_Layer,_Record,_VectorIndexHnswRs,_HnswRsConfig} config
    rand_chacha::ChaCha8Rng["rand_chacha::ChaCha8Rng"]
    class rand_chacha::ChaCha8Rng core
    crate::GpuConfig["crate::GpuConfig"]
    class crate::GpuConfig config
    anyhow::{Result,_anyhow}["anyhow::{Result, anyhow}"]
    class anyhow::{Result,_anyhow} core
    tokenizers::processors::template::TemplateProcessing["tokenizers::processors::template::TemplateProcessing"]
    class tokenizers::processors::template::TemplateProcessing core
    root_crates_ai_src_quantization_calibration["root/crates/ai/src/quantization/calibration"]
    class root_crates_ai_src_quantization_calibration core
    criterion::{black_box,_criterion_group,_criterion_main,_Criterion,_BenchmarkId,_PlotConfiguration}["criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, PlotConfiguration}"]
    class criterion::{black_box,_criterion_group,_criterion_main,_Criterion,_BenchmarkId,_PlotConfiguration} config
    root_crates_memory_src_tests["root/crates/memory/src/tests"]
    class root_crates_memory_src_tests core
    tracing::{debug,_info,_instrument}["tracing::{debug, info, instrument}"]
    class tracing::{debug,_info,_instrument} core
    chrono::Utc["chrono::Utc"]
    class chrono::Utc core
    root_crates_ai_tests_test_stack_dataset["root/crates/ai/tests/test_stack_dataset"]
    class root_crates_ai_tests_test_stack_dataset core
    root_crates_ai_src_quantization_export_onnx["root/crates/ai/src/quantization/export_onnx"]
    class root_crates_ai_src_quantization_export_onnx core
    std::time::SystemTime["std::time::SystemTime"]
    class std::time::SystemTime core

    root_crates_ai_examples_full_quantization_pipeline -.-> ai::quantization::{
    root_crates_ai_examples_full_quantization_pipeline -.-> anyhow::Result
    root_crates_ai_examples_full_quantization_pipeline -.-> ort::{Session,_SessionBuilder}
    root_crates_ai_examples_full_quantization_pipeline -.-> std::path::PathBuf
    root_crates_ai_examples_full_quantization_pipeline -.-> tracing::{info,_warn,_Level}
    root_crates_ai_examples_full_quantization_pipeline -.-> tracing_subscriber
    root_crates_ai_examples_quantize_bge_model -.-> ai::quantization::{
    root_crates_ai_examples_quantize_bge_model -.-> anyhow::Result
    root_crates_ai_examples_quantize_bge_model -.-> std::path::PathBuf
    root_crates_ai_examples_quantize_bge_model -.-> tracing::{info,_Level}
    root_crates_ai_examples_quantize_bge_model -.-> tracing_subscriber
    root_crates_ai_examples_quantize_with_stack -.-> ai::quantization::{
    root_crates_ai_examples_quantize_with_stack -.-> anyhow::Result
    root_crates_ai_examples_quantize_with_stack -.-> std::path::PathBuf
    root_crates_ai_examples_quantize_with_stack -.-> tracing::{info,_Level}
    root_crates_ai_examples_quantize_with_stack -.-> tracing_subscriber
    root_crates_ai_examples_test_mxbai_real_tokenization -.-> ai::reranker_mxbai::BgeRerankerService
    root_crates_ai_examples_test_mxbai_real_tokenization -.-> anyhow::Result
    root_crates_ai_examples_test_mxbai_real_tokenization -.-> std::path::PathBuf
    root_crates_ai_examples_test_real_tokenization -.-> ai::BgeM3EmbeddingService
    root_crates_ai_examples_test_real_tokenization -.-> anyhow::Result
    root_crates_ai_examples_test_real_tokenization -.-> std::path::PathBuf
    root_crates_ai_src_auto_device_selector -.-> anyhow::Result
    root_crates_ai_src_auto_device_selector -.-> async_trait::async_trait
    root_crates_ai_src_auto_device_selector -.-> crate::EmbeddingConfig
    root_crates_ai_src_auto_device_selector -.-> crate::embeddings_cpu::CpuEmbeddingService
    root_crates_ai_src_auto_device_selector -.-> crate::embeddings_gpu::GpuEmbeddingService
    root_crates_ai_src_auto_device_selector -.-> crate::gpu_detector::GpuDetector
    root_crates_ai_src_auto_device_selector ==> root_crates_ai_src_tests
    root_crates_ai_src_auto_device_selector -.-> std::time::Instant
    root_crates_ai_src_auto_device_selector -.-> super::*
    root_crates_ai_src_auto_device_selector -.-> tracing::warn
    root_crates_ai_src_auto_device_selector -.-> tracing::{info,_debug}
    root_crates_ai_src_config -.-> serde::{Deserialize,_Serialize}
    root_crates_ai_src_config -.-> std::path::PathBuf
    root_crates_ai_src_embeddings_bge_m3 -.-> anyhow::Result
    root_crates_ai_src_embeddings_bge_m3 -.-> crate::tokenization::OptimizedTokenizer
    root_crates_ai_src_embeddings_bge_m3 -.-> ort::{session::Session,_value::Tensor,_inputs}
    root_crates_ai_src_embeddings_bge_m3 ==> root_crates_ai_src_tests
    root_crates_ai_src_embeddings_bge_m3 -.-> std::path::PathBuf
    root_crates_ai_src_embeddings_bge_m3 -.-> std::sync::Arc
    root_crates_ai_src_embeddings_bge_m3 -.-> super::*
    root_crates_ai_src_embeddings_bge_m3 -.-> tracing::{info,_debug,_warn}
    root_crates_ai_src_embeddings_cpu -.-> anyhow::Result_as_AnyhowResult
    root_crates_ai_src_embeddings_cpu -.-> crate::EmbeddingConfig
    root_crates_ai_src_embeddings_cpu -.-> crate::GpuConfig
    root_crates_ai_src_embeddings_cpu -.-> crate::memory_pool::{GLOBAL_MEMORY_POOL,_PoolStats}
    root_crates_ai_src_embeddings_cpu -.-> crate::tokenization::{OptimizedTokenizer,_TokenizedInput_as_OptTokenizedInput,_BatchTokenized}
    root_crates_ai_src_embeddings_cpu -.-> crate::{GpuConfig,_GpuInfo}
    root_crates_ai_src_embeddings_cpu -.-> ort::{session::Session,_value::Tensor,_inputs}
    root_crates_ai_src_embeddings_cpu ==> root_crates_ai_src_tests
    root_crates_ai_src_embeddings_cpu -.-> std::path::PathBuf
    root_crates_ai_src_embeddings_cpu -.-> std::sync::Arc
    root_crates_ai_src_embeddings_cpu -.-> std::sync::Mutex
    root_crates_ai_src_embeddings_cpu -.-> super::*
    root_crates_ai_src_embeddings_cpu -.-> tracing::warn
    root_crates_ai_src_embeddings_cpu -.-> tracing::{info,_debug}
    root_crates_ai_src_embeddings_gpu -.-> anyhow::Result
    root_crates_ai_src_embeddings_gpu -.-> crate::gpu_detector::GpuDetector
    root_crates_ai_src_embeddings_gpu -.-> crate::model_downloader::ensure_model
    root_crates_ai_src_embeddings_gpu -.-> crate::{EmbeddingConfig,_GpuConfig}
    root_crates_ai_src_embeddings_gpu -.-> ndarray::Array2
    root_crates_ai_src_embeddings_gpu -.-> ort::{session::Session,_inputs,_value::Tensor}
    root_crates_ai_src_embeddings_gpu -.-> std::path::PathBuf
    root_crates_ai_src_embeddings_gpu -.-> std::sync::Arc
    root_crates_ai_src_embeddings_gpu -.-> std::sync::Mutex
    root_crates_ai_src_embeddings_gpu -.-> std::time::Instant
    root_crates_ai_src_embeddings_gpu -.-> tokenizers::Tokenizer
    root_crates_ai_src_embeddings_gpu -.-> tracing::warn
    root_crates_ai_src_embeddings_gpu -.-> tracing::{info,_debug}
    root_crates_ai_src_errors -.-> std::fmt
    root_crates_ai_src_gpu_config -.-> anyhow::Result
    root_crates_ai_src_gpu_config -.-> crate::gpu_detector::{GpuDetector,_GpuOptimalParams}
    root_crates_ai_src_gpu_config -.-> ort::execution_providers::{CUDAExecutionProvider,_TensorRTExecutionProvider,_ExecutionProviderDispatch}
    root_crates_ai_src_gpu_config -.-> tracing::info
    root_crates_ai_src_gpu_config -.-> tracing::warn
    root_crates_ai_src_gpu_detector ==> root_crates_ai_src_tests
    root_crates_ai_src_gpu_detector -.-> serde::{Deserialize,_Serialize}
    root_crates_ai_src_gpu_detector -.-> std::process::Command
    root_crates_ai_src_gpu_detector -.-> std::str
    root_crates_ai_src_gpu_detector -.-> super::*
    root_crates_ai_src_gpu_detector -.-> tracing::{info,_debug}
    root_crates_ai_src_gpu_memory_pool -.-> anyhow::Result
    root_crates_ai_src_gpu_memory_pool ==> root_crates_ai_src_tests
    root_crates_ai_src_gpu_memory_pool -.-> std::collections::VecDeque
    root_crates_ai_src_gpu_memory_pool -.-> std::sync::{Arc,_Mutex}
    root_crates_ai_src_gpu_memory_pool -.-> super::*
    root_crates_ai_src_gpu_memory_pool -.-> tracing::{info,_debug,_warn}
    root_crates_ai_src_memory_pool ==> root_crates_ai_src_tests
    root_crates_ai_src_memory_pool -.-> std::cell::RefCell
    root_crates_ai_src_memory_pool -.-> std::collections::VecDeque
    root_crates_ai_src_memory_pool -.-> std::sync::Arc
    root_crates_ai_src_memory_pool -.-> super::*
    root_crates_ai_src_memory_pool -.-> thread_local::ThreadLocal
    root_crates_ai_src_memory_pool -.-> tracing::{debug,_info}
    root_crates_ai_src_model_downloader -.-> anyhow::{Result,_Context}
    root_crates_ai_src_model_downloader -.-> reqwest
    root_crates_ai_src_model_downloader ==> root_crates_ai_src_tests
    root_crates_ai_src_model_downloader -.-> std::path::{Path,_PathBuf}
    root_crates_ai_src_model_downloader -.-> std::sync::Arc
    root_crates_ai_src_model_downloader -.-> std::sync::atomic::{AtomicU64,_Ordering}
    root_crates_ai_src_model_downloader -.-> super::*
    root_crates_ai_src_model_downloader -.-> tempfile::TempDir
    root_crates_ai_src_model_downloader -.-> tokio::fs
    root_crates_ai_src_model_downloader -.-> tokio::io::AsyncWriteExt
    root_crates_ai_src_model_downloader -.-> tokio_stream::StreamExt
    root_crates_ai_src_model_downloader -.-> tracing::{info,_warn}
    root_crates_ai_src_models -.-> crate::{AiError,_Result}
    root_crates_ai_src_models -.-> std::path::{Path,_PathBuf}
    root_crates_ai_src_models -.-> tracing::{info,_warn,_debug}
    root_crates_ai_src_quantization_adaround -.-> anyhow::{Result,_anyhow}
    root_crates_ai_src_quantization_adaround -.-> ndarray::{Array1,_Array2,_ArrayView1,_ArrayView2,_Axis}
    root_crates_ai_src_quantization_adaround -.-> ort::{Session,_Value}
    root_crates_ai_src_quantization_adaround -.-> std::collections::HashMap
    root_crates_ai_src_quantization_adaround -.-> std::path::Path
    root_crates_ai_src_quantization_adaround -.-> super::calibration::CalibrationDataset
    root_crates_ai_src_quantization_adaround -.-> super::config::{QuantizationConfig,_CalibrationMethod}
    root_crates_ai_src_quantization_adaround -.-> tracing::{info,_debug,_warn}
    root_crates_ai_src_quantization_calibration -.-> anyhow::{Result,_anyhow}
    root_crates_ai_src_quantization_calibration -.-> std::path::{Path,_PathBuf}
    root_crates_ai_src_quantization_calibration -.-> super::stack_dataset::{StackDatasetLoader,_StackSample,_QualityFilters}
    root_crates_ai_src_quantization_calibration -.-> tracing::{info,_debug}
    root_crates_ai_src_quantization_config -.-> serde::{Deserialize,_Serialize}
    root_crates_ai_src_quantization_export_onnx -.-> anyhow::{Result,_anyhow}
    root_crates_ai_src_quantization_export_onnx -.-> std::path::{Path,_PathBuf}
    root_crates_ai_src_quantization_export_onnx -.-> std::process::Command
    root_crates_ai_src_quantization_export_onnx -.-> tracing::{info,_debug,_warn}
    root_crates_ai_src_quantization_onnx_quantizer -.-> anyhow::{Result,_anyhow}
    root_crates_ai_src_quantization_onnx_quantizer -.-> ndarray::{Array1,_Array2,_ArrayView1,_ArrayView2}
    root_crates_ai_src_quantization_onnx_quantizer -.-> ort::{Session,_SessionBuilder,_GraphOptimizationLevel}
    root_crates_ai_src_quantization_onnx_quantizer -.-> std::collections::HashMap
    root_crates_ai_src_quantization_onnx_quantizer -.-> std::path::{Path,_PathBuf}
    root_crates_ai_src_quantization_onnx_quantizer -.-> super::adaround::{AdaRoundConfig,_QuantizationResult}
    root_crates_ai_src_quantization_onnx_quantizer -.-> super::calibration::CalibrationDataset
    root_crates_ai_src_quantization_onnx_quantizer -.-> super::config::{QuantizationConfig,_CalibrationMethod}
    root_crates_ai_src_quantization_onnx_quantizer -.-> tracing::{info,_debug,_warn}
    root_crates_ai_src_quantization_optimizer -.-> anyhow::{Result,_anyhow}
    root_crates_ai_src_quantization_optimizer -.-> std::path::Path
    root_crates_ai_src_quantization_optimizer -.-> tracing::{info,_debug}
    root_crates_ai_src_quantization_stack_dataset -.-> anyhow::{Result,_anyhow}
    root_crates_ai_src_quantization_stack_dataset -.-> rand::seq::SliceRandom
    root_crates_ai_src_quantization_stack_dataset -.-> rand::thread_rng
    root_crates_ai_src_quantization_stack_dataset -.-> serde::{Deserialize,_Serialize}
    root_crates_ai_src_quantization_stack_dataset -.-> std::fs::File
    root_crates_ai_src_quantization_stack_dataset -.-> std::io::{BufReader,_BufRead}
    root_crates_ai_src_quantization_stack_dataset -.-> std::path::{Path,_PathBuf}
    root_crates_ai_src_quantization_stack_dataset -.-> tracing::{info,_debug,_warn}
    root_crates_ai_src_reranker_mxbai -.-> anyhow::Result_as_AnyhowResult
    root_crates_ai_src_reranker_mxbai -.-> crate::RerankingConfig
    root_crates_ai_src_reranker_mxbai -.-> crate::memory_pool::{GLOBAL_MEMORY_POOL,_PoolStats}
    root_crates_ai_src_reranker_mxbai -.-> crate::{GpuConfig,_GpuInfo}
    root_crates_ai_src_reranker_mxbai -.-> ort::{session::Session,_value::Tensor,_inputs}
    root_crates_ai_src_reranker_mxbai ==> root_crates_ai_src_tests
    root_crates_ai_src_reranker_mxbai -.-> std::path::PathBuf
    root_crates_ai_src_reranker_mxbai -.-> std::sync::Arc
    root_crates_ai_src_reranker_mxbai -.-> std::sync::Mutex
    root_crates_ai_src_reranker_mxbai -.-> super::*
    root_crates_ai_src_reranker_mxbai -.-> tracing::{info,_debug,_warn}
    root_crates_ai_src_reranker_mxbai_optimized -.-> crate::{AiError,_Result,_TokenizerService,_RerankingConfig}
    root_crates_ai_src_reranker_mxbai_optimized -.-> ndarray::Array2
    root_crates_ai_src_reranker_mxbai_optimized -.-> ort::{inputs,_session::Session,_value::Tensor}
    root_crates_ai_src_reranker_mxbai_optimized ==> root_crates_ai_src_tests
    root_crates_ai_src_reranker_mxbai_optimized -.-> std::path::PathBuf
    root_crates_ai_src_reranker_mxbai_optimized -.-> std::sync::Arc
    root_crates_ai_src_reranker_mxbai_optimized -.-> std::time::Instant
    root_crates_ai_src_reranker_mxbai_optimized -.-> super::*
    root_crates_ai_src_reranker_mxbai_optimized -.-> tokio::sync::Mutex
    root_crates_ai_src_reranker_mxbai_optimized -.-> tracing::{debug,_info,_warn}
    root_crates_ai_src_reranking -.-> crate::reranker_mxbai_optimized::OptimizedRerankingService
    root_crates_ai_src_reranking -.-> crate::{Result,_TokenizerService,_RerankingConfig,_models::OnnxSession}
    root_crates_ai_src_reranking -.-> std::collections::hash_map::DefaultHasher
    root_crates_ai_src_reranking -.-> std::hash::{Hash,_Hasher}
    root_crates_ai_src_reranking -.-> std::sync::Arc
    root_crates_ai_src_reranking -.-> tracing::{info,_debug,_warn}
    root_crates_ai_src_tensorrt_cache -.-> anyhow::{Result,_Context}
    root_crates_ai_src_tensorrt_cache ==> root_crates_ai_src_tests
    root_crates_ai_src_tensorrt_cache -.-> serde::{Serialize,_Deserialize}
    root_crates_ai_src_tensorrt_cache -.-> std::collections::hash_map::DefaultHasher
    root_crates_ai_src_tensorrt_cache -.-> std::fs
    root_crates_ai_src_tensorrt_cache -.-> std::hash::{Hash,_Hasher}
    root_crates_ai_src_tensorrt_cache -.-> std::path::{Path,_PathBuf}
    root_crates_ai_src_tensorrt_cache -.-> std::time::SystemTime
    root_crates_ai_src_tensorrt_cache -.-> super::*
    root_crates_ai_src_tensorrt_cache -.-> tempfile::TempDir
    root_crates_ai_src_tensorrt_cache -.-> tracing::{info,_debug}
    root_crates_ai_src_tokenization_mod -.-> anyhow::Result
    root_crates_ai_src_tokenization_mod ==> root_crates_ai_src_tokenization_tests
    root_crates_ai_src_tokenization_mod -.-> std::path::Path
    root_crates_ai_src_tokenization_mod -.-> std::path::PathBuf
    root_crates_ai_src_tokenization_mod -.-> std::sync::Arc
    root_crates_ai_src_tokenization_mod -.-> super::*
    root_crates_ai_src_tokenization_mod -.-> tokenizers::Tokenizer
    root_crates_ai_src_tokenization_mod -.-> tracing::{info,_debug}
    root_crates_ai_src_tokenizer -.-> crate::{AiError,_Result}
    root_crates_ai_src_tokenizer -.-> std::path::Path
    root_crates_ai_src_tokenizer -.-> tokenizers::Tokenizer
    root_crates_ai_src_tokenizer -.-> tokenizers::models::bpe::BPE
    root_crates_ai_src_tokenizer -.-> tokenizers::normalizers::BertNormalizer
    root_crates_ai_src_tokenizer -.-> tokenizers::pre_tokenizers::bert::BertPreTokenizer
    root_crates_ai_src_tokenizer -.-> tokenizers::processors::template::TemplateProcessing
    root_crates_ai_src_tokenizer -.-> tracing::{info,_debug}
    root_crates_ai_tests_test_adaround_quantization -.-> ai::quantization::{
    root_crates_ai_tests_test_adaround_quantization -.-> std::path::PathBuf
    root_crates_ai_tests_test_adaround_quantization -.-> tempfile::TempDir
    root_crates_ai_tests_test_reranker_integration -.-> ai::{RerankingConfig,_RerankingService}
    root_crates_ai_tests_test_stack_dataset -.-> ai::quantization::{
    root_crates_ai_tests_test_stack_dataset -.-> std::path::PathBuf
    root_crates_cli_src_agent -.-> anyhow::Result
    root_crates_cli_src_agent -.-> llm::{LlmClient,_IntentAnalyzerAgent}
    root_crates_cli_src_agent -.-> router::SmartRouter
    root_crates_cli_src_commands_gpu -.-> ai::embeddings_gpu::GpuEmbeddingService
    root_crates_cli_src_commands_gpu -.-> ai::{
    root_crates_cli_src_commands_gpu -.-> anyhow::Result
    root_crates_cli_src_commands_gpu -.-> clap::{Args,_Subcommand}
    root_crates_cli_src_commands_gpu -.-> std::time::Instant
    root_crates_cli_src_commands_gpu -.-> tracing::{info,_warn,_error}
    root_crates_cli_src_commands_quantize -.-> ai::quantization::{
    root_crates_cli_src_commands_quantize -.-> anyhow::Result
    root_crates_cli_src_commands_quantize -.-> clap::Parser
    root_crates_cli_src_commands_quantize -.-> std::path::PathBuf
    root_crates_cli_src_commands_quantize -.-> tracing::{info,_error}
    root_crates_cli_src_main -.-> agent::{UnifiedAgent,_AgentResponse}
    root_crates_cli_src_main -.-> anyhow::Result
    root_crates_cli_src_main -.-> clap::{Parser,_Subcommand}
    root_crates_cli_src_main -.-> commands::GpuCommand
    root_crates_cli_src_main -.-> console::{style,_Term}
    root_crates_cli_src_main -.-> indicatif::{ProgressBar,_ProgressStyle}
    root_crates_cli_src_main -.-> llm::LlmClient
    root_crates_cli_src_main ==> root_crates_cli_src_agent
    root_crates_cli_src_main ==> root_crates_cli_src_commands
    root_crates_cli_src_main -.-> std::io::{self,_Write}
    root_crates_cli_src_main -.-> std::time::Duration
    root_crates_cli_src_main -.-> tokio::time::sleep
    root_crates_llm_src_agents_action_planner -.-> anyhow::{Result,_anyhow}
    root_crates_llm_src_agents_action_planner -.-> crate::LlmClient
    root_crates_llm_src_agents_action_planner -.-> serde::{Deserialize,_Serialize}
    root_crates_llm_src_agents_action_planner -.-> std::collections::HashMap
    root_crates_llm_src_agents_intent_analyzer -.-> anyhow::{Result,_anyhow}
    root_crates_llm_src_agents_intent_analyzer -.-> crate::LlmClient
    root_crates_llm_src_agents_intent_analyzer -.-> serde::{Deserialize,_Serialize}
    root_crates_llm_src_agents_parameter_extractor -.-> anyhow::{Result,_anyhow}
    root_crates_llm_src_agents_parameter_extractor -.-> crate::LlmClient
    root_crates_llm_src_agents_parameter_extractor -.-> serde::{Deserialize,_Serialize}
    root_crates_llm_src_agents_parameter_extractor -.-> std::collections::HashMap
    root_crates_llm_src_agents_tool_selector -.-> anyhow::{Result,_anyhow}
    root_crates_llm_src_agents_tool_selector -.-> crate::LlmClient
    root_crates_llm_src_agents_tool_selector -.-> serde::{Deserialize,_Serialize}
    root_crates_llm_src_lib -.-> anyhow::{Result,_anyhow}
    root_crates_llm_src_lib -.-> serde::{Deserialize,_Serialize}
    root_crates_llm_src_lib -.-> std::env
    root_crates_llm_src_lib -.-> tracing::{info,_debug,_error}
    root_crates_memory_benches_memory_benchmarks -.-> criterion::{black_box,_criterion_group,_criterion_main,_Criterion,_BenchmarkId}
    root_crates_memory_benches_memory_benchmarks -.-> memory::{
    root_crates_memory_benches_memory_benchmarks -.-> std::sync::Arc
    root_crates_memory_benches_memory_benchmarks -.-> std::time::Duration
    root_crates_memory_benches_memory_benchmarks -.-> tempfile::TempDir
    root_crates_memory_benches_memory_benchmarks -.-> tokio::runtime::Runtime
    root_crates_memory_benches_scalability_benchmarks -.-> criterion::{black_box,_criterion_group,_criterion_main,_Criterion,_BenchmarkId,_PlotConfiguration}
    root_crates_memory_benches_scalability_benchmarks -.-> memory::{VectorStore,_Layer,_Record,_VectorIndexHnswRs,_HnswRsConfig}
    root_crates_memory_benches_scalability_benchmarks -.-> rand::{Rng,_SeedableRng}
    root_crates_memory_benches_scalability_benchmarks -.-> rand_chacha::ChaCha8Rng
    root_crates_memory_benches_scalability_benchmarks -.-> std::time::Duration
    root_crates_memory_benches_scalability_benchmarks -.-> tempfile::TempDir
    root_crates_memory_benches_scalability_benchmarks -.-> tokio::runtime::Runtime
    root_crates_memory_benches_vector_index_v3_benchmark -.-> criterion::{black_box,_criterion_group,_criterion_main,_Criterion,_BenchmarkId}
    root_crates_memory_benches_vector_index_v3_benchmark -.-> memory::{VectorIndexV3,_VectorIndexConfigV3}
    root_crates_memory_benches_vector_index_v3_benchmark -.-> std::time::Duration
    root_crates_memory_benches_vector_search_benchmarks -.-> criterion::{black_box,_criterion_group,_criterion_main,_Criterion,_BenchmarkId,_Throughput}
    root_crates_memory_benches_vector_search_benchmarks -.-> memory::{
    root_crates_memory_benches_vector_search_benchmarks -.-> std::sync::Arc
    root_crates_memory_benches_vector_search_benchmarks -.-> std::time::Duration
    root_crates_memory_benches_vector_search_benchmarks -.-> tempfile::TempDir
    root_crates_memory_benches_vector_search_benchmarks -.-> tokio::runtime::Runtime
    root_crates_memory_benches_vector_search_benchmarks -.-> tracing_subscriber
    root_crates_memory_examples_benchmark_hnsw_vs_linear -.-> anyhow::Result
    root_crates_memory_examples_benchmark_hnsw_vs_linear -.-> memory::{VectorIndexHnswRs,_HnswRsConfig}
    root_crates_memory_examples_benchmark_hnsw_vs_linear -.-> std::time::Instant
    root_crates_memory_examples_full_pipeline_test -.-> anyhow::Result
    root_crates_memory_examples_full_pipeline_test -.-> memory::{MemoryConfig,_MemoryService,_Layer,_Record}
    root_crates_memory_examples_full_pipeline_test -.-> std::path::PathBuf
    root_crates_memory_examples_memory_demo -.-> anyhow::Result
    root_crates_memory_examples_memory_demo -.-> memory::{Layer,_MemoryConfig,_MemoryService,_Record}
    root_crates_memory_examples_memory_demo -.-> std::path::PathBuf
    root_crates_memory_examples_memory_demo -.-> uuid::Uuid
    root_crates_memory_examples_test_gpu_acceleration -.-> ai::gpu_detector::GpuDetector
    root_crates_memory_examples_test_gpu_acceleration -.-> anyhow::Result
    root_crates_memory_examples_test_gpu_acceleration -.-> chrono::Utc
    root_crates_memory_examples_test_gpu_acceleration -.-> memory::{MemoryService,_MemoryConfig,_Record,_Layer,_BatchProcessorStats}
    root_crates_memory_examples_test_gpu_acceleration ==> root_crates_memory_examples_tests
    root_crates_memory_examples_test_gpu_acceleration -.-> std::time::Instant
    root_crates_memory_examples_test_gpu_acceleration -.-> super::*
    root_crates_memory_examples_test_gpu_acceleration -.-> tempfile::TempDir
    root_crates_memory_examples_test_gpu_acceleration -.-> tracing::info
    root_crates_memory_examples_test_gpu_acceleration -.-> uuid::Uuid
    root_crates_memory_examples_test_graceful_degradation -.-> anyhow::Result
    root_crates_memory_examples_test_graceful_degradation -.-> memory::fallback::{GracefulEmbeddingService,_EmbeddingProvider}
    root_crates_memory_examples_test_graceful_degradation -.-> std::sync::Arc
    root_crates_memory_examples_test_graceful_degradation -.-> std::sync::atomic::{AtomicUsize,_Ordering}
    root_crates_memory_examples_test_graceful_degradation -.-> tracing::info
    root_crates_memory_examples_test_health_monitoring -.-> anyhow::Result
    root_crates_memory_examples_test_health_monitoring -.-> chrono::Utc
    root_crates_memory_examples_test_health_monitoring -.-> memory::{
    root_crates_memory_examples_test_health_monitoring -.-> tokio::time::{sleep,_Duration}
    root_crates_memory_examples_test_health_monitoring -.-> tracing::info
    root_crates_memory_examples_test_health_monitoring -.-> uuid::Uuid
    root_crates_memory_examples_test_lru_cache -.-> anyhow::Result
    root_crates_memory_examples_test_lru_cache -.-> chrono::Utc
    root_crates_memory_examples_test_lru_cache -.-> memory::{
    root_crates_memory_examples_test_lru_cache -.-> std::time::Instant
    root_crates_memory_examples_test_lru_cache -.-> tracing::info
    root_crates_memory_examples_test_lru_cache -.-> uuid::Uuid
    root_crates_memory_examples_test_memory_gpu_integration -.-> ai::gpu_detector::GpuDetector
    root_crates_memory_examples_test_memory_gpu_integration -.-> ai::{AiConfig,_EmbeddingConfig,_RerankingConfig,_GpuConfig}
    root_crates_memory_examples_test_memory_gpu_integration -.-> memory::{MemoryService,_MemoryConfig,_Layer,_Record}
    root_crates_memory_examples_test_memory_gpu_integration -.-> std::path::PathBuf
    root_crates_memory_examples_test_memory_gpu_integration -.-> std::time::Instant
    root_crates_memory_examples_test_memory_gpu_integration -.-> tempfile::TempDir
    root_crates_memory_examples_test_memory_gpu_integration -.-> tracing::{info,_warn}
    root_crates_memory_examples_test_reranker -.-> ai::{RerankingConfig,_RerankingService}
    root_crates_memory_examples_test_reranker -.-> anyhow::Result
    root_crates_memory_examples_test_reranker -.-> tracing_subscriber
    root_crates_memory_examples_test_reranker_mock -.-> anyhow::Result
    root_crates_memory_examples_test_unified_api -.-> anyhow::Result
    root_crates_memory_examples_test_unified_api -.-> memory::{MemoryConfig,_MemoryService,_UnifiedMemoryAPI,_MemoryContext,_ApiSearchOptions,_Layer}
    root_crates_memory_examples_test_unified_api -.-> std::sync::Arc
    root_crates_memory_examples_test_unified_api -.-> tracing::info
    root_crates_memory_examples_test_vector_index_clean -.-> ai::AiConfig
    root_crates_memory_examples_test_vector_index_clean -.-> anyhow::Result
    root_crates_memory_examples_test_vector_index_clean -.-> memory::{MemoryService,_MemoryConfig,_Record,_Layer}
    root_crates_memory_examples_test_vector_index_clean -.-> std::path::PathBuf
    root_crates_memory_examples_vector_index_performance -.-> ai::AiConfig
    root_crates_memory_examples_vector_index_performance -.-> anyhow::Result
    root_crates_memory_examples_vector_index_performance -.-> memory::{MemoryService,_MemoryConfig,_Record,_Layer}
    root_crates_memory_examples_vector_index_performance -.-> std::path::PathBuf
    root_crates_memory_examples_vector_index_performance -.-> std::time::Instant
    root_crates_memory_models_bge_code_v1_embedding_specialized_quantize --> json
    root_crates_memory_models_bge_code_v1_embedding_specialized_quantize --> numpy_as_np
    root_crates_memory_models_bge_code_v1_embedding_specialized_quantize --> pathlib
    root_crates_memory_models_bge_code_v1_embedding_specialized_quantize --> pickle
    root_crates_memory_models_bge_code_v1_embedding_specialized_quantize --> sentence_transformers
    root_crates_memory_models_bge_code_v1_embedding_specialized_quantize --> sentence_transformers_quantization
    root_crates_memory_models_bge_code_v1_embedding_specialized_quantize --> time
    root_crates_memory_models_bge_code_v1_embedding_specialized_quantize --> torch
    root_crates_memory_models_bge_code_v1_embedding_specialized_quantize --> transformers
    root_crates_memory_models_bge_code_v1_mixed_precision_quantize --> numpy_as_np
    root_crates_memory_models_bge_code_v1_mixed_precision_quantize --> onnx
    root_crates_memory_models_bge_code_v1_mixed_precision_quantize --> onnxruntime_as_ort
    root_crates_memory_models_bge_code_v1_mixed_precision_quantize --> onnxruntime_quantization
    root_crates_memory_models_bge_code_v1_mixed_precision_quantize --> pathlib
    root_crates_memory_models_bge_code_v1_mixed_precision_quantize --> time
    root_crates_memory_models_bge_code_v1_mixed_precision_quantize --> transformers
    root_crates_memory_models_bge_code_v1_modern_quantize_2025 --> json
    root_crates_memory_models_bge_code_v1_modern_quantize_2025 --> numpy_as_np
    root_crates_memory_models_bge_code_v1_modern_quantize_2025 --> onnxruntime_as_ort
    root_crates_memory_models_bge_code_v1_modern_quantize_2025 --> os
    root_crates_memory_models_bge_code_v1_modern_quantize_2025 --> pathlib
    root_crates_memory_models_bge_code_v1_modern_quantize_2025 --> qllm
    root_crates_memory_models_bge_code_v1_modern_quantize_2025 --> qllm_nn_linear
    root_crates_memory_models_bge_code_v1_modern_quantize_2025 --> time
    root_crates_memory_models_bge_code_v1_modern_quantize_2025 --> torch
    root_crates_memory_models_bge_code_v1_modern_quantize_2025 --> transformers
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> (
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> json
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> logging
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> numpy_as_np
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> onnx
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> onnxruntime_as_ort
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> onnxruntime_quantization
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> os
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> pandas_as_pd
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> pathlib
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> shutil
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> sklearn_model_selection
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> sys
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> time
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> torch
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> tqdm
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> transformers
    root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer --> typing
    root_crates_memory_models_bge_code_v1_simple_best_quantize --> json
    root_crates_memory_models_bge_code_v1_simple_best_quantize --> numpy_as_np
    root_crates_memory_models_bge_code_v1_simple_best_quantize --> onnxruntime_as_ort
    root_crates_memory_models_bge_code_v1_simple_best_quantize --> onnxruntime_quantization
    root_crates_memory_models_bge_code_v1_simple_best_quantize --> pathlib
    root_crates_memory_models_bge_code_v1_simple_best_quantize --> shutil
    root_crates_memory_models_bge_code_v1_simple_best_quantize --> time
    root_crates_memory_models_bge_code_v1_simple_best_quantize --> transformers
    root_crates_memory_models_bge_code_v1_test_fp16_quality --> numpy_as_np
    root_crates_memory_models_bge_code_v1_test_fp16_quality --> onnxruntime_as_ort
    root_crates_memory_models_bge_code_v1_test_fp16_quality --> pathlib
    root_crates_memory_models_bge_code_v1_test_fp16_quality --> shutil
    root_crates_memory_models_bge_code_v1_test_fp16_quality --> transformers
    root_crates_memory_models_bge_code_v1_test_quality --> numpy_as_np
    root_crates_memory_models_bge_code_v1_test_quality --> onnxruntime_as_ort
    root_crates_memory_models_bge_code_v1_test_quality --> os
    root_crates_memory_models_bge_code_v1_test_quality --> pathlib
    root_crates_memory_models_bge_code_v1_test_quality --> time
    root_crates_memory_models_bge_code_v1_test_quality --> transformers
    root_crates_memory_models_the_stack_smol_xs_dataset_creation --> datasets
    root_crates_memory_models_the_stack_smol_xs_dataset_creation --> pandas_as_pd
    root_crates_memory_models_the_stack_smol_xs_the_stack_smol_xs --> datasets
    root_crates_memory_models_the_stack_smol_xs_the_stack_smol_xs --> json
    root_crates_memory_src_api -.-> anyhow::Result
    root_crates_memory_src_api -.-> crate::{
    root_crates_memory_src_api -.-> std::sync::Arc
    root_crates_memory_src_api -.-> uuid::Uuid
    root_crates_memory_src_batch_manager -.-> anyhow::Result
    root_crates_memory_src_batch_manager -.-> crate::metrics::MetricsCollector
    root_crates_memory_src_batch_manager -.-> crate::storage::VectorStore
    root_crates_memory_src_batch_manager -.-> crate::types::{Layer,_Record}
    root_crates_memory_src_batch_manager -.-> parking_lot::{Mutex,_RwLock}
    root_crates_memory_src_batch_manager ==> root_crates_memory_src_tests
    root_crates_memory_src_batch_manager -.-> std::collections::HashMap
    root_crates_memory_src_batch_manager -.-> std::sync::Arc
    root_crates_memory_src_batch_manager -.-> std::time::{Duration,_Instant}
    root_crates_memory_src_batch_manager -.-> super::*
    root_crates_memory_src_batch_manager -.-> tempfile::TempDir
    root_crates_memory_src_batch_manager -.-> tokio::sync::mpsc
    root_crates_memory_src_batch_manager -.-> tokio::time::interval
    root_crates_memory_src_batch_manager -.-> tracing::{debug,_info,_warn}
    root_crates_memory_src_batch_manager -.-> uuid::Uuid
    root_crates_memory_src_cache -.-> anyhow::{Context,_Result}
    root_crates_memory_src_cache -.-> parking_lot::RwLock
    root_crates_memory_src_cache ==> root_crates_memory_src_tests
    root_crates_memory_src_cache -.-> serde::{Deserialize,_Serialize}
    root_crates_memory_src_cache -.-> sled::Db
    root_crates_memory_src_cache -.-> std::collections::hash_map::DefaultHasher
    root_crates_memory_src_cache -.-> std::hash::{Hash,_Hasher}
    root_crates_memory_src_cache -.-> std::path::Path
    root_crates_memory_src_cache -.-> std::sync::Arc
    root_crates_memory_src_cache -.-> super::*
    root_crates_memory_src_cache -.-> tempfile::TempDir
    root_crates_memory_src_cache -.-> tracing::{debug,_info}
    root_crates_memory_src_cache_interface -.-> anyhow::Result
    root_crates_memory_src_cache_lru -.-> anyhow::{Context,_Result}
    root_crates_memory_src_cache_lru -.-> parking_lot::{RwLock,_Mutex}
    root_crates_memory_src_cache_lru ==> root_crates_memory_src_tests
    root_crates_memory_src_cache_lru -.-> serde::{Deserialize,_Serialize}
    root_crates_memory_src_cache_lru -.-> sled::Db
    root_crates_memory_src_cache_lru -.-> std::collections::hash_map::DefaultHasher
    root_crates_memory_src_cache_lru -.-> std::collections::{HashMap,_VecDeque}
    root_crates_memory_src_cache_lru -.-> std::hash::{Hash,_Hasher}
    root_crates_memory_src_cache_lru -.-> std::path::Path
    root_crates_memory_src_cache_lru -.-> std::sync::Arc
    root_crates_memory_src_cache_lru -.-> std::time::Duration
    root_crates_memory_src_cache_lru -.-> std::time::{SystemTime,_UNIX_EPOCH}
    root_crates_memory_src_cache_lru -.-> super::*
    root_crates_memory_src_cache_lru -.-> tempfile::TempDir
    root_crates_memory_src_cache_lru -.-> tracing::{debug,_info,_warn}
    root_crates_memory_src_cache_migration -.-> anyhow::Result
    root_crates_memory_src_cache_migration -.-> crate::{EmbeddingCache,_EmbeddingCacheLRU,_CacheConfig}
    root_crates_memory_src_cache_migration ==> root_crates_memory_src_tests
    root_crates_memory_src_cache_migration -.-> std::path::Path
    root_crates_memory_src_cache_migration -.-> super::*
    root_crates_memory_src_cache_migration -.-> tracing::{info,_warn}
    root_crates_memory_src_fallback -.-> anyhow::Result
    root_crates_memory_src_fallback ==> root_crates_memory_src_tests
    root_crates_memory_src_fallback -.-> sha2::{Sha256,_Digest}
    root_crates_memory_src_fallback -.-> std::collections::HashMap
    root_crates_memory_src_fallback -.-> super::*
    root_crates_memory_src_fallback -.-> tracing::{info,_warn,_error}
    root_crates_memory_src_gpu_accelerated -.-> ai::{GpuEmbeddingService,_CpuEmbeddingService,_EmbeddingConfig}
    root_crates_memory_src_gpu_accelerated -.-> anyhow::Result
    root_crates_memory_src_gpu_accelerated -.-> crate::cache_interface::EmbeddingCacheInterface
    root_crates_memory_src_gpu_accelerated ==> root_crates_memory_src_tests
    root_crates_memory_src_gpu_accelerated -.-> std::sync::Arc
    root_crates_memory_src_gpu_accelerated -.-> super::*
    root_crates_memory_src_gpu_accelerated -.-> tempfile::TempDir
    root_crates_memory_src_gpu_accelerated -.-> tokio::sync::{Mutex,_Semaphore}
    root_crates_memory_src_gpu_accelerated -.-> tracing::{info,_warn,_debug}
    root_crates_memory_src_health -.-> anyhow::Result
    root_crates_memory_src_health -.-> chrono::{DateTime,_Utc,_Duration}
    root_crates_memory_src_health -.-> serde::{Serialize,_Deserialize}
    root_crates_memory_src_health -.-> std::collections::{HashMap,_VecDeque}
    root_crates_memory_src_health -.-> std::sync::{Arc,_RwLock}
    root_crates_memory_src_health -.-> std::time::Instant
    root_crates_memory_src_health -.-> tokio::sync::mpsc
    root_crates_memory_src_health -.-> tracing::{info,_warn,_error}
    root_crates_memory_src_lib -.-> chrono::{DateTime,_Utc}
    root_crates_memory_src_lib ==> root_crates_memory_src_batch_manager
    root_crates_memory_src_lib ==> root_crates_memory_src_cache
    root_crates_memory_src_lib ==> root_crates_memory_src_cache_interface
    root_crates_memory_src_lib ==> root_crates_memory_src_cache_lru
    root_crates_memory_src_lib ==> root_crates_memory_src_cache_migration
    root_crates_memory_src_lib ==> root_crates_memory_src_metrics
    root_crates_memory_src_lib ==> root_crates_memory_src_promotion
    root_crates_memory_src_lib ==> root_crates_memory_src_service
    root_crates_memory_src_lib ==> root_crates_memory_src_storage
    root_crates_memory_src_lib ==> root_crates_memory_src_types
    root_crates_memory_src_lib ==> root_crates_memory_src_vector_index_hnswlib
    root_crates_memory_src_lib -.-> serde::{Deserialize,_Serialize}
    root_crates_memory_src_metrics -.-> parking_lot::RwLock
    root_crates_memory_src_metrics ==> root_crates_memory_src_tests
    root_crates_memory_src_metrics -.-> serde::{Deserialize,_Serialize}
    root_crates_memory_src_metrics -.-> std::collections::HashMap
    root_crates_memory_src_metrics -.-> std::sync::Arc
    root_crates_memory_src_metrics -.-> std::time::Duration
    root_crates_memory_src_metrics -.-> std::time::{Duration,_Instant}
    root_crates_memory_src_metrics -.-> super::*
    root_crates_memory_src_metrics -.-> tracing::{debug,_info}
    root_crates_memory_src_migration -.-> anyhow::Result
    root_crates_memory_src_migration -.-> chrono::{DateTime,_Utc}
    root_crates_memory_src_migration -.-> crate::types::{Layer,_Record}
    root_crates_memory_src_migration -.-> serde::{Deserialize,_Serialize}
    root_crates_memory_src_migration -.-> sled::Db
    root_crates_memory_src_migration -.-> std::path::Path
    root_crates_memory_src_migration -.-> tracing::{info,_warn,_error}
    root_crates_memory_src_promotion -.-> anyhow::Result
    root_crates_memory_src_promotion -.-> chrono::{DateTime,_Duration,_Utc}
    root_crates_memory_src_promotion -.-> crate::{
    root_crates_memory_src_promotion ==> root_crates_memory_src_tests
    root_crates_memory_src_promotion -.-> sled::{Db,_Tree}
    root_crates_memory_src_promotion -.-> std::collections::BTreeMap
    root_crates_memory_src_promotion -.-> std::sync::Arc
    root_crates_memory_src_promotion -.-> super::*
    root_crates_memory_src_promotion -.-> tracing::{debug,_info,_warn}
    root_crates_memory_src_service -.-> ai::{AiConfig,_ModelLoader,_RerankingService}
    root_crates_memory_src_service -.-> anyhow::Result
    root_crates_memory_src_service -.-> crate::{
    root_crates_memory_src_service -.-> std::path::PathBuf
    root_crates_memory_src_service -.-> std::sync::Arc
    root_crates_memory_src_service -.-> tracing::{debug,_info}
    root_crates_memory_src_storage -.-> anyhow::Result
    root_crates_memory_src_storage -.-> chrono::Utc
    root_crates_memory_src_storage -.-> crate::metrics::{MetricsCollector,_TimedOperation}
    root_crates_memory_src_storage -.-> crate::types::{Layer,_Record}
    root_crates_memory_src_storage -.-> crate::vector_index_hnswlib::{VectorIndexHnswRs,_HnswRsConfig}
    root_crates_memory_src_storage -.-> serde::{Deserialize,_Serialize}
    root_crates_memory_src_storage -.-> sled::Db
    root_crates_memory_src_storage -.-> std::collections::HashMap
    root_crates_memory_src_storage -.-> std::path::Path
    root_crates_memory_src_storage -.-> std::sync::Arc
    root_crates_memory_src_storage -.-> std::time::Instant
    root_crates_memory_src_storage -.-> tracing::{debug,_info}
    root_crates_memory_src_types -.-> chrono::{DateTime,_Utc}
    root_crates_memory_src_types -.-> serde::{Deserialize,_Serialize}
    root_crates_memory_src_types -.-> uuid::Uuid
    root_crates_memory_src_vector_index_hnswlib -.-> anyhow::{anyhow,_Result}
    root_crates_memory_src_vector_index_hnswlib -.-> hnsw_rs::hnsw::*
    root_crates_memory_src_vector_index_hnswlib -.-> hnsw_rs::prelude::*
    root_crates_memory_src_vector_index_hnswlib -.-> parking_lot::RwLock
    root_crates_memory_src_vector_index_hnswlib ==> root_crates_memory_src_tests
    root_crates_memory_src_vector_index_hnswlib -.-> serde::{Deserialize,_Serialize}
    root_crates_memory_src_vector_index_hnswlib -.-> std::collections::HashMap
    root_crates_memory_src_vector_index_hnswlib -.-> std::sync::Arc
    root_crates_memory_src_vector_index_hnswlib -.-> std::sync::atomic::{AtomicU64,_Ordering}
    root_crates_memory_src_vector_index_hnswlib -.-> std::time::Instant
    root_crates_memory_src_vector_index_hnswlib -.-> super::*
    root_crates_memory_src_vector_index_hnswlib -.-> tracing::{debug,_info,_warn}
    root_crates_memory_tests_integration_test -.-> anyhow::Result
    root_crates_memory_tests_integration_test -.-> memory::{Layer,_MemoryConfig,_MemoryService,_Record}
    root_crates_memory_tests_integration_test -.-> tempfile::TempDir
    root_crates_memory_tests_integration_test -.-> uuid::Uuid
    root_crates_memory_tests_performance_test -.-> memory::{
    root_crates_memory_tests_performance_test -.-> std::time::Instant
    root_crates_memory_tests_performance_test -.-> tokio
    root_crates_memory_tests_performance_test -.-> uuid::Uuid
    root_crates_memory_tests_test_hnsw_comparison -.-> anyhow::Result
    root_crates_memory_tests_test_hnsw_comparison -.-> memory::{
    root_crates_memory_tests_test_hnsw_comparison -.-> std::time::Instant
    root_crates_memory_tests_test_metrics_integration -.-> anyhow::Result
    root_crates_memory_tests_test_metrics_integration -.-> memory::{
    root_crates_memory_tests_test_metrics_integration -.-> std::time::Duration
    root_crates_memory_tests_test_metrics_integration -.-> tempfile::TempDir
    root_crates_memory_tests_test_promotion -.-> ai::AiConfig
    root_crates_memory_tests_test_promotion -.-> anyhow::Result
    root_crates_memory_tests_test_promotion -.-> chrono::{Duration,_Utc}
    root_crates_memory_tests_test_promotion -.-> memory::{
    root_crates_memory_tests_test_promotion -.-> std::sync::Arc
    root_crates_memory_tests_test_promotion -.-> tempfile::TempDir
    root_crates_memory_tests_test_promotion -.-> uuid::Uuid
    root_crates_memory_tests_test_two_stage_search -.-> anyhow::Result
    root_crates_memory_tests_test_two_stage_search -.-> memory::{Layer,_MemoryConfig,_MemoryService,_Record}
    root_crates_memory_tests_test_two_stage_search -.-> std::sync::atomic::{AtomicU64,_Ordering}
    root_crates_memory_tests_test_two_stage_search -.-> tempfile::TempDir
    root_crates_memory_tests_test_two_stage_search -.-> uuid::Uuid
    root_crates_router_src_lib -.-> anyhow::{anyhow,_Result}
    root_crates_router_src_lib -.-> llm::{LlmClient,_ActionPlannerAgent,_ToolSelectorAgent,_ParameterExtractorAgent}
    root_crates_router_src_lib ==> root_crates_router_src_tests
    root_crates_router_src_lib -.-> serde::{Deserialize,_Serialize}
    root_crates_router_src_lib -.-> std::collections::HashMap
    root_crates_router_src_lib -.-> super::*
    root_crates_router_src_lib -.-> tokio
    root_crates_router_src_lib -.-> tools::{ToolRegistry,_ToolInput,_ToolOutput}
    root_crates_todo_src_graph -.-> anyhow::Result
    root_crates_todo_src_graph -.-> crate::types::{TodoItem,_TaskState}
    root_crates_todo_src_graph -.-> petgraph::Direction
    root_crates_todo_src_graph -.-> petgraph::algo::toposort
    root_crates_todo_src_graph -.-> petgraph::graph::{DiGraph,_NodeIndex}
    root_crates_todo_src_graph ==> root_crates_todo_src_tests
    root_crates_todo_src_graph -.-> std::collections::HashMap
    root_crates_todo_src_graph -.-> std::sync::RwLock
    root_crates_todo_src_graph -.-> super::*
    root_crates_todo_src_graph -.-> uuid::Uuid
    root_crates_todo_src_graph_v2 -.-> anyhow::Result
    root_crates_todo_src_graph_v2 -.-> crate::types::{TodoItem,_TaskState}
    root_crates_todo_src_graph_v2 -.-> dashmap::DashMap
    root_crates_todo_src_graph_v2 -.-> parking_lot::RwLock
    root_crates_todo_src_graph_v2 -.-> petgraph::Direction
    root_crates_todo_src_graph_v2 -.-> petgraph::algo::{toposort,_has_path_connecting}
    root_crates_todo_src_graph_v2 -.-> petgraph::graph::{DiGraph,_NodeIndex}
    root_crates_todo_src_graph_v2 ==> root_crates_todo_src_tests
    root_crates_todo_src_graph_v2 -.-> std::collections::HashMap
    root_crates_todo_src_graph_v2 -.-> std::sync::Arc
    root_crates_todo_src_graph_v2 -.-> super::*
    root_crates_todo_src_graph_v2 -.-> uuid::Uuid
    root_crates_todo_src_lib -.-> anyhow::Result
    root_crates_todo_src_lib ==> root_crates_todo_src_tests
    root_crates_todo_src_lib -.-> std::path::Path
    root_crates_todo_src_lib -.-> super::*
    root_crates_todo_src_lib -.-> tempfile::TempDir
    root_crates_todo_src_service_v2 -.-> anyhow::Result
    root_crates_todo_src_service_v2 -.-> crate::graph_v2::{DependencyGraphV2,_GraphStats}
    root_crates_todo_src_service_v2 -.-> crate::store_v2::TodoStoreV2
    root_crates_todo_src_service_v2 -.-> crate::types::*
    root_crates_todo_src_service_v2 -.-> dashmap::DashMap
    root_crates_todo_src_service_v2 -.-> lru::LruCache
    root_crates_todo_src_service_v2 -.-> parking_lot::Mutex
    root_crates_todo_src_service_v2 ==> root_crates_todo_src_tests
    root_crates_todo_src_service_v2 -.-> std::num::NonZeroUsize
    root_crates_todo_src_service_v2 -.-> std::path::Path
    root_crates_todo_src_service_v2 -.-> std::sync::Arc
    root_crates_todo_src_service_v2 -.-> super::*
    root_crates_todo_src_service_v2 -.-> tempfile::TempDir
    root_crates_todo_src_service_v2 -.-> tokio::sync::mpsc
    root_crates_todo_src_service_v2 -.-> tracing::{debug,_info,_instrument}
    root_crates_todo_src_service_v2 -.-> uuid::Uuid
    root_crates_todo_src_store -.-> anyhow::{Context,_Result}
    root_crates_todo_src_store -.-> chrono::{DateTime,_Utc}
    root_crates_todo_src_store -.-> crate::types::*
    root_crates_todo_src_store -.-> rusqlite::{params,_Connection,_OptionalExtension}
    root_crates_todo_src_store -.-> std::path::Path
    root_crates_todo_src_store -.-> std::sync::Arc
    root_crates_todo_src_store -.-> tokio::sync::Mutex
    root_crates_todo_src_store -.-> uuid::Uuid
    root_crates_todo_src_store_v2 -.-> anyhow::{Context,_Result}
    root_crates_todo_src_store_v2 -.-> chrono::{DateTime,_Utc}
    root_crates_todo_src_store_v2 -.-> crate::types::*
    root_crates_todo_src_store_v2 -.-> r2d2::Pool
    root_crates_todo_src_store_v2 -.-> r2d2_sqlite::SqliteConnectionManager
    root_crates_todo_src_store_v2 -.-> rusqlite::{params,_Connection,_Row,_OptionalExtension}
    root_crates_todo_src_store_v2 -.-> serde_json
    root_crates_todo_src_store_v2 -.-> std::collections::HashMap
    root_crates_todo_src_store_v2 -.-> std::path::Path
    root_crates_todo_src_store_v2 -.-> std::sync::Arc
    root_crates_todo_src_store_v2 -.-> tracing::{debug,_instrument}
    root_crates_todo_src_store_v2 -.-> uuid::Uuid
    root_crates_todo_src_types -.-> chrono::{DateTime,_Utc}
    root_crates_todo_src_types -.-> memory::MemRef
    root_crates_todo_src_types -.-> serde::{Deserialize,_Serialize}
    root_crates_todo_src_types -.-> std::collections::HashMap
    root_crates_todo_src_types -.-> uuid::Uuid
    root_crates_tools_src_file_ops -.-> anyhow::{anyhow,_Result}
    root_crates_tools_src_file_ops -.-> crate::{Tool,_ToolInput,_ToolOutput,_ToolSpec}
    root_crates_tools_src_file_ops -.-> std::collections::HashMap
    root_crates_tools_src_file_ops -.-> std::fs
    root_crates_tools_src_file_ops -.-> std::path::{Path,_PathBuf}
    root_crates_tools_src_file_ops -.-> walkdir::WalkDir
    root_crates_tools_src_git_ops -.-> anyhow::Result
    root_crates_tools_src_git_ops -.-> crate::{Tool,_ToolInput,_ToolOutput,_ToolSpec}
    root_crates_tools_src_git_ops -.-> std::collections::HashMap
    root_crates_tools_src_git_ops -.-> tokio::process::Command
    root_crates_tools_src_lib -.-> anyhow::Result
    root_crates_tools_src_lib -.-> serde::{Deserialize,_Serialize}
    root_crates_tools_src_lib -.-> std::collections::HashMap
    root_crates_tools_src_shell_ops -.-> anyhow::Result
    root_crates_tools_src_shell_ops -.-> crate::{Tool,_ToolInput,_ToolOutput,_ToolSpec}
    root_crates_tools_src_shell_ops -.-> std::collections::HashMap
    root_crates_tools_src_shell_ops -.-> tokio::process::Command
    root_crates_tools_src_web_ops -.-> anyhow::Result
    root_crates_tools_src_web_ops -.-> crate::{Tool,_ToolInput,_ToolOutput,_ToolSpec}
    root_crates_tools_src_web_ops -.-> std::collections::HashMap
    root_docs_daemon_src_main -.-> jsonschema::{Draft,_JSONSchema}
    root_docs_daemon_src_main -.-> notify::{Watcher,_RecursiveMode,_Result_as_NotifyResult,_RecommendedWatcher,_Event}
    root_docs_daemon_src_main -.-> regex::Regex
    root_docs_daemon_src_main -.-> serde::{Deserialize,_Serialize}
    root_docs_daemon_src_main -.-> serde_json::Value
    root_docs_daemon_src_main -.-> sha2::{Sha256,_Digest}
    root_docs_daemon_src_main -.-> std::collections::HashMap
    root_docs_daemon_src_main -.-> std::fs
    root_docs_daemon_src_main -.-> std::path::{Path,_PathBuf}
    root_docs_daemon_src_main -.-> std::sync::mpsc::channel
    root_docs_daemon_src_main -.-> std::time::Duration
    root_docs_daemon_src_main -.-> walkdir::WalkDir
    root_scripts_convert_annotations --> glob
    root_scripts_convert_annotations --> json
    root_scripts_convert_annotations --> os
    root_scripts_convert_annotations --> re
    root_scripts_convert_annotations --> sys
    root_scripts_convert_annotations --> typing
    root_scripts_ctl --> argparse
    root_scripts_ctl --> datetime
    root_scripts_ctl --> json
    root_scripts_ctl --> os
    root_scripts_ctl --> pathlib
    root_scripts_ctl --> sys
    root_scripts_ctl --> typing
    root_scripts_ctl_converter --> datetime
    root_scripts_ctl_converter --> json
    root_scripts_ctl_converter --> re
    root_scripts_ctl_converter --> sys
    root_scripts_ctl_converter --> typing
    root_scripts_download_mxbai_tokenizer --> json
    root_scripts_download_mxbai_tokenizer --> os
    root_scripts_download_mxbai_tokenizer --> pathlib
    root_scripts_download_mxbai_tokenizer --> requests
    root_scripts_download_mxbai_tokenizer --> sys
    root_scripts_generate_ctl_metrics --> datetime
    root_scripts_generate_ctl_metrics --> glob
    root_scripts_generate_ctl_metrics --> json
    root_scripts_generate_ctl_metrics --> os
    root_scripts_generate_ctl_metrics --> re
    root_scripts_generate_ctl_metrics --> subprocess
    root_scripts_generate_ctl_metrics --> sys
    root_scripts_generate_ctl_metrics --> typing

    %% Module grouping
    subgraph Core["🔧 Core Modules"]
        root_crates_cli_src_commands
        root_crates_memory_examples_test_gpu_acceleration
        root_crates_ai_src_embeddings_cpu
        tokio
        root_crates_memory_tests_integration_test
        std::time::Instant
        root_crates_ai_tests_test_reranker_integration
        root_crates_memory_benches_vector_search_benchmarks
        root_crates_memory_src_gpu_accelerated
        tracing::info
        async_trait::async_trait
        reqwest
        anyhow::{Result,_Context}
        root_crates_memory_models_bge_code_v1_quantize_with_real_tokenizer
        root_crates_ai_examples_test_mxbai_real_tokenization
        std::sync::{Arc,_RwLock}
        std::str
        root_crates_memory_tests_test_promotion
        root_crates_cli_src_commands_quantize
        tokio::time::{sleep,_Duration}
        serde_json::Value
        root_crates_ai_src_reranking
        agent::{UnifiedAgent,_AgentResponse}
        anyhow::Result
        ai::{
        crate::LlmClient
        tokio::sync::mpsc
        root_crates_router_src_tests
        std::fs::File
        root_crates_todo_src_types
        sentence_transformers
        memory::MemRef
        anyhow::Result_as_AnyhowResult
        root_crates_memory_models_bge_code_v1_mixed_precision_quantize
        anyhow::{anyhow,_Result}
        dashmap::DashMap
        root_crates_ai_src_quantization_stack_dataset
        root_crates_ai_src_tokenization_tests
        json
        root_crates_todo_src_graph_v2
        glob
        parking_lot::RwLock
        std::sync::{Arc,_Mutex}
        root_crates_llm_src_agents_parameter_extractor
        ort::execution_providers::{CUDAExecutionProvider,_TensorRTExecutionProvider,_ExecutionProviderDispatch}
        tracing::{info,_debug,_warn}
        std::hash::{Hash,_Hasher}
        root_crates_memory_examples_tests
        std::path::{Path,_PathBuf}
        root_crates_memory_tests_test_metrics_integration
        lru::LruCache
        std::path::Path
        rusqlite::{params,_Connection,_OptionalExtension}
        tqdm
        root_crates_memory_src_cache_migration
        tracing::{info,_Level}
        root_crates_ai_src_embeddings_bge_m3
        criterion::{black_box,_criterion_group,_criterion_main,_Criterion,_BenchmarkId}
        crate::graph_v2::{DependencyGraphV2,_GraphStats}
        std::fs
        super::*
        root_crates_memory_models_bge_code_v1_test_fp16_quality
        root_crates_ai_src_embeddings_gpu
        crate::cache_interface::EmbeddingCacheInterface
        root_crates_memory_src_cache
        hnsw_rs::hnsw::*
        root_crates_memory_models_bge_code_v1_simple_best_quantize
        ndarray::{Array1,_Array2,_ArrayView1,_ArrayView2,_Axis}
        root_crates_todo_src_store_v2
        pandas_as_pd
        ort::{inputs,_session::Session,_value::Tensor}
        tracing::{info,_debug}
        root_crates_memory_models_bge_code_v1_modern_quantize_2025
        llm::{LlmClient,_IntentAnalyzerAgent}
        crate::memory_pool::{GLOBAL_MEMORY_POOL,_PoolStats}
        tokenizers::pre_tokenizers::bert::BertPreTokenizer
        crate::metrics::MetricsCollector
        tokio::runtime::Runtime
        jsonschema::{Draft,_JSONSchema}
        serde_json
        root_crates_ai_src_gpu_memory_pool
        root_crates_ai_examples_quantize_with_stack
        root_crates_ai_examples_test_real_tokenization
        onnx
        root_crates_todo_src_tests
        root_crates_memory_benches_vector_index_v3_benchmark
        std::path::PathBuf
        sled::Db
        tokio_stream::StreamExt
        std::cell::RefCell
        root_crates_memory_examples_test_vector_index_clean
        std::time::{SystemTime,_UNIX_EPOCH}
        root_crates_memory_src_vector_index_hnswlib
        sha2::{Sha256,_Digest}
        root_crates_memory_tests_performance_test
        ndarray::{Array1,_Array2,_ArrayView1,_ArrayView2}
        root_crates_memory_examples_test_health_monitoring
        crate::tokenization::{OptimizedTokenizer,_TokenizedInput_as_OptTokenizedInput,_BatchTokenized}
        sklearn_model_selection
        transformers
        tracing::{info,_warn}
        root_crates_memory_models_the_stack_smol_xs_the_stack_smol_xs
        ndarray::Array2
        crate::storage::VectorStore
        root_crates_ai_src_tokenizer
        time
        tokenizers::models::bpe::BPE
        root_crates_memory_examples_vector_index_performance
        qllm_nn_linear
        logging
        root_crates_ai_examples_quantize_bge_model
        serde::{Deserialize,_Serialize}
        ort::{session::Session,_inputs,_value::Tensor}
        tokio::time::sleep
        root_crates_memory_examples_test_reranker
        root_crates_ai_src_reranker_mxbai_optimized
        crate::types::{TodoItem,_TaskState}
        root_scripts_ctl
        root_crates_llm_src_agents_intent_analyzer
        chrono::{Duration,_Utc}
        serde::{Serialize,_Deserialize}
        ort::{Session,_SessionBuilder}
        router::SmartRouter
        std::collections::hash_map::DefaultHasher
        std::sync::atomic::{AtomicUsize,_Ordering}
        root_crates_ai_examples_full_quantization_pipeline
        root_crates_memory_examples_benchmark_hnsw_vs_linear
        requests
        chrono::{DateTime,_Utc}
        root_crates_llm_src_lib
        tracing::{info,_error}
        r2d2::Pool
        chrono::{DateTime,_Utc,_Duration}
        tokenizers::Tokenizer
        tracing::{info,_warn,_debug}
        std::fmt
        std::sync::Mutex
        rand::seq::SliceRandom
        crate::gpu_detector::{GpuDetector,_GpuOptimalParams}
        root_crates_ai_src_tokenization_mod
        clap::Parser
        ort::{session::Session,_value::Tensor,_inputs}
        root_crates_ai_src_tensorrt_cache
        root_crates_ai_src_gpu_detector
        std::time::{Duration,_Instant}
        root_crates_memory_src_lib
        root_crates_memory_src_types
        crate::metrics::{MetricsCollector,_TimedOperation}
        root_crates_todo_src_store
        root_crates_ai_src_models
        criterion::{black_box,_criterion_group,_criterion_main,_Criterion,_BenchmarkId,_Throughput}
        super::stack_dataset::{StackDatasetLoader,_StackSample,_QualityFilters}
        numpy_as_np
        anyhow::{Context,_Result}
        tracing::{debug,_info,_warn}
        commands::GpuCommand
        std::collections::VecDeque
        root_crates_memory_examples_memory_demo
        torch
        typing
        root_crates_memory_src_cache_interface
        root_crates_memory_tests_test_hnsw_comparison
        std::io::{self,_Write}
        tracing::{info,_debug,_error}
        root_crates_ai_src_quantization_onnx_quantizer
        std::time::Duration
        crate::types::*
        root_crates_cli_src_agent
        qllm
        root_crates_memory_examples_test_lru_cache
        root_crates_ai_src_errors
        root_scripts_ctl_converter
        tempfile::TempDir
        root_crates_memory_benches_memory_benchmarks
        crate::{
        petgraph::algo::toposort
        thread_local::ThreadLocal
        std::collections::HashMap
        super::calibration::CalibrationDataset
        root_crates_ai_src_quantization_optimizer
        indicatif::{ProgressBar,_ProgressStyle}
        root_crates_memory_src_batch_manager
        std::sync::RwLock
        pathlib
        notify::{Watcher,_RecursiveMode,_Result_as_NotifyResult,_RecommendedWatcher,_Event}
        regex::Regex
        tracing::{debug,_info}
        root_crates_memory_src_metrics
        root_crates_ai_src_auto_device_selector
        llm::LlmClient
        datetime
        root_crates_cli_src_main
        root_scripts_download_mxbai_tokenizer
        root_scripts_generate_ctl_metrics
        root_crates_llm_src_agents_action_planner
        root_crates_memory_benches_scalability_benchmarks
        onnxruntime_as_ort
        sled::{Db,_Tree}
        tokio::process::Command
        sys
        petgraph::graph::{DiGraph,_NodeIndex}
        chrono::{DateTime,_Duration,_Utc}
        datasets
        root_crates_router_src_lib
        root_crates_memory_src_fallback
        tracing::warn
        root_crates_ai_tests_test_adaround_quantization
        root_crates_memory_src_health
        root_crates_memory_examples_full_pipeline_test
        rusqlite::{params,_Connection,_Row,_OptionalExtension}
        root_crates_todo_src_graph
        root_crates_memory_src_migration
        tokio::io::AsyncWriteExt
        tokio::fs
        root_crates_memory_models_bge_code_v1_test_quality
        root_crates_memory_models_the_stack_smol_xs_dataset_creation
        root_crates_memory_examples_test_memory_gpu_integration
        hnsw_rs::prelude::*
        parking_lot::{RwLock,_Mutex}
        crate::{AiError,_Result}
        walkdir::WalkDir
        root_crates_ai_src_tests
        root_crates_memory_src_storage
        ai::quantization::{
        root_crates_ai_src_reranker_mxbai
        root_crates_cli_src_commands_gpu
        crate::store_v2::TodoStoreV2
        rand::{Rng,_SeedableRng}
        std::process::Command
        std::io::{BufReader,_BufRead}
        root_crates_memory_tests_test_two_stage_search
        crate::gpu_detector::GpuDetector
        root_docs_daemon_src_main
        petgraph::Direction
        re
        crate::types::{Layer,_Record}
        argparse
        std::collections::{HashMap,_VecDeque}
        root_crates_memory_models_bge_code_v1_embedding_specialized_quantize
        tracing_subscriber
        ort::{Session,_Value}
        std::sync::Arc
        root_scripts_convert_annotations
        tokio::sync::{Mutex,_Semaphore}
        tracing::{info,_warn,_error}
        ort::{Session,_SessionBuilder,_GraphOptimizationLevel}
        tokenizers::normalizers::BertNormalizer
        clap::{Parser,_Subcommand}
        console::{style,_Term}
        root_crates_memory_examples_test_graceful_degradation
        onnxruntime_quantization
        ai::gpu_detector::GpuDetector
        tokio::time::interval
        std::collections::BTreeMap
        root_crates_todo_src_lib
        r2d2_sqlite::SqliteConnectionManager
        root_crates_ai_src_memory_pool
        petgraph::algo::{toposort,_has_path_connecting}
        memory::{
        parking_lot::Mutex
        uuid::Uuid
        std::sync::atomic::{AtomicU64,_Ordering}
        root_crates_memory_examples_test_reranker_mock
        crate::model_downloader::ensure_model
        sentence_transformers_quantization
        parking_lot::{Mutex,_RwLock}
        tracing::{debug,_instrument}
        os
        root_crates_memory_src_promotion
        tracing::{info,_warn,_Level}
        root_crates_memory_src_cache_lru
        tokio::sync::Mutex
        std::num::NonZeroUsize
        root_crates_ai_src_quantization_adaround
        rand::thread_rng
        pickle
        clap::{Args,_Subcommand}
        std::sync::mpsc::channel
        subprocess
        root_crates_ai_src_model_downloader
        (
        crate::tokenization::OptimizedTokenizer
        rand_chacha::ChaCha8Rng
        anyhow::{Result,_anyhow}
        tokenizers::processors::template::TemplateProcessing
        root_crates_ai_src_quantization_calibration
        root_crates_memory_src_tests
        tracing::{debug,_info,_instrument}
        chrono::Utc
        root_crates_ai_tests_test_stack_dataset
        root_crates_ai_src_quantization_export_onnx
        std::time::SystemTime
    end
    subgraph Services["🚀 Services"]
        root_crates_memory_src_service
        ai::{GpuEmbeddingService,_CpuEmbeddingService,_EmbeddingConfig}
        ai::embeddings_gpu::GpuEmbeddingService
        ai::reranker_mxbai::BgeRerankerService
        memory::{MemoryService,_MemoryConfig,_Layer,_Record}
        root_crates_memory_src_api
        crate::embeddings_gpu::GpuEmbeddingService
        crate::{Result,_TokenizerService,_RerankingConfig,_models::OnnxSession}
        ai::{AiConfig,_ModelLoader,_RerankingService}
        memory::{MemoryConfig,_MemoryService,_UnifiedMemoryAPI,_MemoryContext,_ApiSearchOptions,_Layer}
        root_crates_todo_src_service_v2
        memory::{MemoryService,_MemoryConfig,_Record,_Layer}
        ai::BgeM3EmbeddingService
        crate::embeddings_cpu::CpuEmbeddingService
        crate::reranker_mxbai_optimized::OptimizedRerankingService
        ai::{RerankingConfig,_RerankingService}
        memory::fallback::{GracefulEmbeddingService,_EmbeddingProvider}
        root_crates_memory_examples_test_unified_api
        memory::{MemoryService,_MemoryConfig,_Record,_Layer,_BatchProcessorStats}
        memory::{Layer,_MemoryConfig,_MemoryService,_Record}
        crate::{AiError,_Result,_TokenizerService,_RerankingConfig}
        memory::{MemoryConfig,_MemoryService,_Layer,_Record}
    end
    subgraph Utils["🛠️ Utilities"]
        root_crates_llm_src_agents_tool_selector
        crate::{Tool,_ToolInput,_ToolOutput,_ToolSpec}
        llm::{LlmClient,_ActionPlannerAgent,_ToolSelectorAgent,_ParameterExtractorAgent}
        root_crates_tools_src_shell_ops
        tools::{ToolRegistry,_ToolInput,_ToolOutput}
        shutil
        root_crates_tools_src_lib
        root_crates_tools_src_git_ops
        root_crates_tools_src_web_ops
        root_crates_tools_src_file_ops
    end
    subgraph Config["⚙️ Configuration"]
        super::adaround::{AdaRoundConfig,_QuantizationResult}
        ai::{AiConfig,_EmbeddingConfig,_RerankingConfig,_GpuConfig}
        root_crates_ai_src_gpu_config
        crate::EmbeddingConfig
        std::env
        memory::{VectorIndexHnswRs,_HnswRsConfig}
        crate::vector_index_hnswlib::{VectorIndexHnswRs,_HnswRsConfig}
        root_crates_ai_src_config
        crate::{EmbeddingCache,_EmbeddingCacheLRU,_CacheConfig}
        ai::AiConfig
        root_crates_ai_src_quantization_config
        super::config::{QuantizationConfig,_CalibrationMethod}
        crate::{GpuConfig,_GpuInfo}
        crate::RerankingConfig
        memory::{VectorIndexV3,_VectorIndexConfigV3}
        crate::{EmbeddingConfig,_GpuConfig}
        memory::{VectorStore,_Layer,_Record,_VectorIndexHnswRs,_HnswRsConfig}
        crate::GpuConfig
        criterion::{black_box,_criterion_group,_criterion_main,_Criterion,_BenchmarkId,_PlotConfiguration}
    end
