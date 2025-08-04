# Memory Mind Map - Визуальная карта memory crate

> Лист компонентного одуванчика - визуальная карта memory crate и его компонентов

[[_Components Hub - Центр всех компонентов системы]] → Memory Mind Map

## 🧠 Полная карта Memory System

```mermaid
mindmap
  root((Memory System))
    Core Services
      MemoryService[70%]
        Main orchestrator
        API coordination
        Service management
      VectorStore[65%]
        HNSW storage
        Layer management
        Search interface
      PromotionEngine[75%]
        Time-based rules
        ML scoring
        Layer transitions
    
    Caching & Storage
      EmbeddingCache[85%]
        Sled persistence
        Cache interface
        Migration support
      EmbeddingCacheLRU[90%]
        Eviction policy
        Memory bounds
        Hit tracking
      DatabaseManager[90%]
        Sled pooling
        Concurrent access
        Resource lifecycle
    
    GPU Integration
      GpuBatchProcessor[95%]
        Batch queuing
        GPU acceleration
        CPU fallback
      ResourceManager[95%]
        Dynamic scaling
        Memory limits
        Auto-adjustment
    
    Advanced Systems
      StreamingAPI[95%]
        Real-time updates
        Async processing
        Backpressure handling
      MLPromotionEngine[95%]
        Neural scoring
        Pattern learning
        Smart promotion
      NotificationSystem[95%]
        Alert system
        Webhook integration
        Event handling
    
    Infrastructure
      HealthMonitor[80%]
        System health
        Alert thresholds
        Status tracking
      MetricsCollector[85%]
        Performance stats
        Usage analytics
        Trend analysis
      RetryManager[90%]
        Exponential backoff
        Circuit breaker
        Error recovery
```

## 🔗 Взаимосвязи компонентов

```mermaid
graph TB
    subgraph "API Layer"
        MS[MemoryService]
        STREAM[StreamingAPI]
    end
    
    subgraph "Storage Layer"
        VS[VectorStore]
        DB[DatabaseManager]
        CACHE[EmbeddingCache]
    end
    
    subgraph "Processing Layer"
        GPU[GpuBatchProcessor]
        PROM[PromotionEngine]
        ML[MLPromotionEngine]
    end
    
    subgraph "Support Layer"
        HM[HealthMonitor]
        RM[ResourceManager]
        RETRY[RetryManager]
    end
    
    MS --> VS
    MS --> CACHE
    MS --> GPU
    MS --> PROM
    MS --> HM
    
    VS --> DB
    CACHE --> DB
    
    GPU --> CACHE
    PROM --> VS
    PROM --> ML
    
    HM --> RM
    RETRY --> VS
    RETRY --> DB
    
    style MS fill:#f96,stroke:#333,stroke-width:4px
    style VS fill:#69f,stroke:#333,stroke-width:4px
```

## 📊 Готовность компонентов

```mermaid
pie title "Component Readiness Distribution"
    "Ready (90-100%)" : 8
    "Almost Ready (80-89%)" : 3
    "In Progress (70-79%)" : 2
    "Needs Work (60-69%)" : 1
```

### По категориям

```mermaid
graph LR
    subgraph "🟢 Production Ready [90-100%]"
        P1[GpuBatchProcessor]
        P2[ResourceManager]
        P3[NotificationSystem]
        P4[MLPromotionEngine]
        P5[StreamingAPI]
        P6[EmbeddingCacheLRU]
        P7[DatabaseManager]
        P8[RetryManager]
    end
    
    subgraph "🟡 High Ready [80-89%]"
        H1[EmbeddingCache]
        H2[MetricsCollector]
        H3[BackupManager]
        H4[HealthMonitor]
    end
    
    subgraph "🟠 Active Dev [70-79%]"
        A1[PromotionEngine]
        A2[MemoryService]
    end
    
    subgraph "🔴 Needs Work [<70%]"
        N1[VectorStore]
    end
```

## 🎯 Критические пути

### Path 1: Запись данных
```mermaid
flowchart LR
    Input --> MS[MemoryService]
    MS --> GPU[GpuBatchProcessor]
    GPU --> VS[VectorStore]
    VS --> Success
    
    style MS fill:#f96
    style GPU fill:#9f6
    style VS fill:#69f
```

### Path 2: Поиск
```mermaid
flowchart LR
    Query --> MS[MemoryService]
    MS --> CACHE{Cache Hit?}
    CACHE -->|Yes| Return
    CACHE -->|No| VS[VectorStore]
    VS --> HNSW[HNSW Search]
    HNSW --> Return
    
    style CACHE fill:#ff9
    style HNSW fill:#9ff
```

### Path 3: Продвижение
```mermaid
flowchart LR
    CRON[Scheduler] --> PE[PromotionEngine]
    PE --> EVAL[Evaluate Records]
    EVAL --> ML[ML Scoring]
    ML --> MOVE[Move Layers]
    MOVE --> UPDATE[Update Indexes]
    
    style PE fill:#f9f
    style ML fill:#9f9
```

## 🏷️ Теги

#memory #mindmap #components #leaf

---
[[_Components Hub - Центр всех компонентов системы|← К центру компонентного одуванчика]]