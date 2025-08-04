# AI Mind Map - Визуальная карта AI crate

> Лист компонентного одуванчика - визуальная карта AI crate и его компонентов

[[_Components Hub - Центр всех компонентов системы]] → AI Mind Map

## 🧠 Полная карта AI System

```mermaid
mindmap
  root((AI System))
    Embedding Services
      EmbeddingsCPU[90%]
        ONNX Runtime
        BGE-M3 Model
        Thread Pool
        SIMD Optimization
      EmbeddingsGPU[95%]
        CUDA Integration
        Batch Processing
        Memory Pool
        Auto Fallback
      SimpleQwen3Tokenizer[95%]
        BPE Tokenization
        Vocabulary Handling
        Special Tokens
        Encoding/Decoding
    
    Model Management
      ModelRegistry[100%]
        Model Catalog
        Version Control
        Path Resolution
        Config Management
      ModelDownloader[95%]
        Auto Download
        Progress Tracking
        Checksum Verify
        Cache Management
      TensorRTCache[90%]
        Optimized Models
        Engine Caching
        Version Tracking
    
    GPU Infrastructure
      GpuDetector[95%]
        CUDA Detection
        Device Query
        Memory Info
        Capability Check
      GpuConfig[100%]
        Provider Config
        Memory Limits
        Execution Mode
      GpuFallback[100%]
        Graceful Degradation
        Error Recovery
        CPU Fallback
        Performance Tracking
      GpuMemoryPool[90%]
        Allocation Strategy
        Fragmentation Control
        Reuse Optimization
      GpuPipeline[95%]
        Parallel Batches
        Stream Management
        Sync Points
    
    Device Selection
      AutoDeviceSelector[95%]
        Hardware Detection
        Performance Scoring
        Load Balancing
        Dynamic Switching
      DeviceManager
        Multi-GPU Support
        Resource Allocation
        Health Monitoring
    
    Reranking
      RerankerOptimized[90%]
        MxBAI Model
        Score Calculation
        Batch Reranking
        Cache Integration
```

## 🔗 Взаимосвязи компонентов

```mermaid
graph TB
    subgraph "API Layer"
        ES[EmbeddingService]
        RS[RerankerService]
    end
    
    subgraph "Model Layer"
        REG[ModelRegistry]
        DL[ModelDownloader]
        CPU[EmbeddingsCPU]
        GPU[EmbeddingsGPU]
        RERANK[Reranker]
    end
    
    subgraph "Infrastructure"
        DETECT[GpuDetector]
        CONFIG[GpuConfig]
        FALLBACK[GpuFallback]
        POOL[GpuMemoryPool]
        PIPELINE[GpuPipeline]
    end
    
    subgraph "Optimization"
        SELECTOR[AutoDeviceSelector]
        CACHE[TensorRTCache]
    end
    
    ES --> SELECTOR
    SELECTOR --> CPU
    SELECTOR --> GPU
    
    GPU --> DETECT
    GPU --> CONFIG
    GPU --> POOL
    GPU --> PIPELINE
    
    CPU --> FALLBACK
    GPU --> FALLBACK
    
    ES --> REG
    REG --> DL
    
    GPU --> CACHE
    RS --> RERANK
    
    style ES fill:#f96,stroke:#333,stroke-width:4px
    style GPU fill:#69f,stroke:#333,stroke-width:4px
    style FALLBACK fill:#9f6,stroke:#333,stroke-width:4px
```

## 📊 Готовность компонентов

```mermaid
pie title "AI Component Readiness"
    "Production Ready (95-100%)" : 7
    "High Ready (90-94%)" : 4
    "In Progress (80-89%)" : 0
    "Needs Work (<80%)" : 0
```

### Детальный статус

```mermaid
graph LR
    subgraph "🟢 Fully Ready [100%]"
        P1[ModelRegistry]
        P2[GpuConfig]
        P3[GpuFallback]
    end
    
    subgraph "🟢 Production Ready [95%]"
        P4[EmbeddingsGPU]
        P5[AutoDeviceSelector]
        P6[GpuDetector]
        P7[GpuPipeline]
        P8[ModelDownloader]
        P9[SimpleQwen3Tokenizer]
    end
    
    subgraph "🟡 Almost Ready [90%]"
        A1[EmbeddingsCPU]
        A2[TensorRTCache]
        A3[GpuMemoryPool]
        A4[RerankerOptimized]
    end
```

## 🎯 Критические пути обработки

### Path 1: Генерация эмбеддингов (GPU)

```mermaid
sequenceDiagram
    participant Client
    participant Service
    participant Selector
    participant GPU
    participant Pool
    participant Model
    
    Client->>Service: embed_text("query")
    Service->>Selector: select_device()
    Selector-->>Service: GPU (score: 0.95)
    
    Service->>GPU: prepare_batch([text])
    GPU->>Pool: allocate_memory(size)
    Pool-->>GPU: memory_handle
    
    GPU->>Model: run_inference(batch)
    Model-->>GPU: embeddings[768D]
    
    GPU->>Pool: release_memory()
    GPU-->>Service: embeddings
    Service-->>Client: Vec<f32>
```

### Path 2: Fallback на CPU

```mermaid
flowchart LR
    REQ[Embedding Request] --> CHECK{GPU Available?}
    
    CHECK -->|Yes| GPU_TRY{Try GPU}
    CHECK -->|No| CPU[CPU Processing]
    
    GPU_TRY -->|Success| GPU_RESULT[GPU Result]
    GPU_TRY -->|Fail| FALLBACK[Fallback Manager]
    
    FALLBACK --> LOG[Log Issue]
    LOG --> CPU
    
    CPU --> THREAD[Thread Pool]
    THREAD --> SIMD[SIMD Optimize]
    SIMD --> ONNX[ONNX Runtime]
    ONNX --> CPU_RESULT[CPU Result]
    
    GPU_RESULT --> CACHE[Cache Result]
    CPU_RESULT --> CACHE
    
    style CHECK decision fill:#ffd
    style FALLBACK fill:#f96
    style CACHE fill:#9f6
```

### Path 3: Batch обработка

```mermaid
graph TD
    subgraph "Input Queue"
        T1[Text 1]
        T2[Text 2]
        T3[Text 3]
        TN[Text N]
    end
    
    subgraph "Batching"
        COLLECT[Collect Batch]
        SIZE{Batch Size?}
        TIMEOUT{Timeout?}
    end
    
    subgraph "GPU Pipeline"
        PREP[Prepare Data]
        TRANSFER[GPU Transfer]
        COMPUTE[Parallel Compute]
        RETRIEVE[Get Results]
    end
    
    subgraph "Distribution"
        SPLIT[Split Results]
        R1[Result 1]
        R2[Result 2]
        R3[Result 3]
        RN[Result N]
    end
    
    T1 --> COLLECT
    T2 --> COLLECT
    T3 --> COLLECT
    TN --> COLLECT
    
    COLLECT --> SIZE
    SIZE -->|Full| PREP
    SIZE -->|Waiting| TIMEOUT
    TIMEOUT -->|Expired| PREP
    
    PREP --> TRANSFER
    TRANSFER --> COMPUTE
    COMPUTE --> RETRIEVE
    RETRIEVE --> SPLIT
    
    SPLIT --> R1
    SPLIT --> R2
    SPLIT --> R3
    SPLIT --> RN
    
    style COMPUTE fill:#69f,stroke:#333,stroke-width:4px
```

## 🚀 Оптимизации производительности

### GPU оптимизации

```mermaid
mindmap
  root((GPU Optimizations))
    Memory Management
      Pooling
        Pre-allocation
        Reuse buffers
        Defragmentation
      Pinned Memory
        Fast transfers
        Zero-copy
        Direct access
    
    Batch Processing
      Dynamic Batching
        Size optimization
        Latency balance
        Throughput max
      Pipeline
        Async transfers
        Overlap compute
        Stream sync
    
    Model Optimization
      TensorRT
        Graph fusion
        Kernel selection
        Precision tuning
      Quantization
        INT8 inference
        Mixed precision
        Calibration
```

### CPU оптимизации

```mermaid
graph LR
    subgraph "Thread Level"
        TH1[Thread Pool]
        TH2[Work Stealing]
        TH3[Affinity]
    end
    
    subgraph "Instruction Level"
        SIMD[SIMD/AVX2]
        CACHE[Cache Friendly]
        PREFETCH[Prefetching]
    end
    
    subgraph "Algorithm Level"
        BATCH[Micro-batching]
        FUSE[Op Fusion]
        SPARSE[Sparsity]
    end
    
    TH1 --> SIMD
    TH2 --> CACHE
    TH3 --> PREFETCH
    
    SIMD --> BATCH
    CACHE --> FUSE
    PREFETCH --> SPARSE
```

## 📈 Метрики производительности

### Сравнение устройств

```mermaid
graph TD
    subgraph "Throughput (embeddings/sec)"
        GPU_T[GPU: 1000+]
        CPU_T[CPU: 100-200]
    end
    
    subgraph "Latency (ms)"
        GPU_L[GPU: 5-10ms]
        CPU_L[CPU: 50-100ms]
    end
    
    subgraph "Memory (MB)"
        GPU_M[GPU: 2000-4000]
        CPU_M[CPU: 500-1000]
    end
    
    subgraph "Power (W)"
        GPU_P[GPU: 200-300]
        CPU_P[CPU: 50-100]
    end
    
    style GPU_T fill:#4f4
    style GPU_L fill:#4f4
    style CPU_M fill:#4f4
    style CPU_P fill:#4f4
```

## 🔧 Конфигурация и настройка

### Переменные окружения

```bash
# GPU Configuration
ONNX_GPU_DEVICE_ID=0
ONNX_GPU_MEM_LIMIT=2048
ONNX_EXECUTION_MODE=parallel
ONNX_GRAPH_OPTIMIZATION=all

# Model Paths
MAGRAY_MODEL_DIR=/models
MAGRAY_CACHE_DIR=/cache

# Performance
MAGRAY_BATCH_SIZE=32
MAGRAY_BATCH_TIMEOUT_MS=100
MAGRAY_USE_TENSORRT=true
```

### Архитектурные паттерны

```mermaid
graph TD
    subgraph "Service Pattern"
        API[Public API]
        IMPL[Implementation]
        FALLBACK[Fallback Chain]
    end
    
    subgraph "Resource Pattern"  
        POOL[Resource Pool]
        LEASE[Lease/Return]
        MONITOR[Health Monitor]
    end
    
    subgraph "Pipeline Pattern"
        STAGE1[Stage 1]
        STAGE2[Stage 2]
        STAGE3[Stage 3]
    end
    
    API --> IMPL
    IMPL --> FALLBACK
    
    POOL --> LEASE
    LEASE --> MONITOR
    
    STAGE1 --> STAGE2
    STAGE2 --> STAGE3
```

## 🏷️ Теги

#ai #gpu #mindmap #components #leaf

---
[[_Components Hub - Центр всех компонентов системы|← К центру компонентного одуванчика]]