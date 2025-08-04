# Data Flow - Потоки данных через систему

> Лист архитектурного одуванчика - поток данных через компоненты MAGRAY

[[_Architecture Hub - Центр архитектурной информации]] → Data Flow

## 🌊 Общий поток данных

```mermaid
graph TB
    subgraph "Entry Points"
        CLI[CLI Input]
        API[API Request]
        CRON[Scheduled Task]
    end
    
    subgraph "Processing Pipeline"
        UA[UnifiedAgent]
        INT[Intent Analysis]
        ROUTE[Smart Router]
        
        subgraph "Execution Paths"
            CHAT[Chat Path]
            SEARCH[Search Path]
            TOOL[Tool Path]
            SMART[Smart Path]
        end
    end
    
    subgraph "Data Operations"
        EMB[Embedding Generation]
        VEC[Vector Operations]
        MEM[Memory Storage]
        PROM[Promotion Engine]
    end
    
    subgraph "Output"
        RESP[Response]
        SIDE[Side Effects]
        METRICS[Metrics]
    end
    
    CLI --> UA
    API --> UA
    CRON --> UA
    
    UA --> INT
    INT --> ROUTE
    
    ROUTE --> CHAT
    ROUTE --> SEARCH
    ROUTE --> TOOL
    ROUTE --> SMART
    
    CHAT --> EMB
    SEARCH --> VEC
    TOOL --> SIDE
    SMART --> EMB
    
    EMB --> MEM
    VEC --> MEM
    MEM --> PROM
    
    CHAT --> RESP
    SEARCH --> RESP
    TOOL --> RESP
    SMART --> RESP
    
    PROM --> METRICS
    
    style EMB fill:#f9f,stroke:#333,stroke-width:4px
    style MEM fill:#9f9,stroke:#333,stroke-width:4px
```

## 📝 Детальные потоки

### 1. Chat Flow - Диалоговый режим

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant Agent
    participant LLM
    participant Memory
    
    User->>CLI: "Explain vector search"
    CLI->>Agent: ParseCommand(chat, query)
    Agent->>Memory: SearchContext(query)
    Memory-->>Agent: RelevantContext[]
    Agent->>LLM: GenerateResponse(query, context)
    LLM-->>Agent: Response
    Agent->>Memory: StoreInteraction(query, response)
    Agent-->>CLI: FormattedResponse
    CLI-->>User: Display
```

### 2. Search Flow - Векторный поиск

```mermaid
graph LR
    subgraph "Input Processing"
        Q[Query Text] --> TOK[Tokenize]
        TOK --> EMBED[Embed]
    end
    
    subgraph "Vector Search"
        EMBED --> HNSW[HNSW Search]
        HNSW --> CAND[Candidates]
        CAND --> FILTER[Filter]
        FILTER --> RERANK[Rerank]
    end
    
    subgraph "Result Enhancement"
        RERANK --> ENRICH[Enrich Metadata]
        ENRICH --> SORT[Sort by Score]
        SORT --> FORMAT[Format Output]
    end
    
    style EMBED fill:#ff9,stroke:#333,stroke-width:2px
    style HNSW fill:#9ff,stroke:#333,stroke-width:2px
    style RERANK fill:#f9f,stroke:#333,stroke-width:2px
```

### 3. Tool Flow - Выполнение инструментов

```mermaid
stateDiagram-v2
    [*] --> Parse: Tool Command
    Parse --> Validate: Extract Parameters
    
    state Validate {
        [*] --> CheckPermissions
        CheckPermissions --> CheckParams
        CheckParams --> [*]
    }
    
    Validate --> Execute: Valid
    Validate --> Error: Invalid
    
    state Execute {
        [*] --> Sandbox
        Sandbox --> RunTool
        RunTool --> CaptureOutput
        CaptureOutput --> [*]
    }
    
    Execute --> StoreResult
    StoreResult --> FormatResponse
    FormatResponse --> [*]
    
    Error --> [*]
```

### 4. Smart Flow - Многошаговые задачи

```mermaid
mindmap
  root((Smart Task))
    Analysis
      Intent Detection
      Complexity Assessment
      Resource Planning
    
    Planning
      Task Decomposition
      DAG Creation
      Dependency Resolution
    
    Execution
      Parallel Tasks
        Tool Calls
        LLM Queries
        Memory Ops
      Sequential Tasks
        Dependent Steps
        Validation
        Aggregation
    
    Completion
      Result Assembly
      Memory Update
      Metric Recording
```

## 🔄 Embedding Pipeline

### Генерация векторов

```mermaid
graph TB
    subgraph "Text Input"
        T[Text] --> NORM[Normalize]
        NORM --> CHUNK[Chunk if needed]
    end
    
    subgraph "GPU Path"
        CHUNK --> GPU{GPU Available?}
        GPU -->|Yes| BATCH[Batch Queue]
        BATCH --> CUDA[CUDA Process]
        CUDA --> ONNX[ONNX Model]
    end
    
    subgraph "CPU Path"
        GPU -->|No| THREAD[Thread Pool]
        THREAD --> SIMD[SIMD Optimize]
        SIMD --> ONNX
    end
    
    subgraph "Output"
        ONNX --> VEC[768D Vector]
        VEC --> CACHE[Cache Result]
        CACHE --> RETURN[Return]
    end
    
    style GPU decision fill:#ffd,stroke:#333,stroke-width:2px
    style ONNX fill:#dfd,stroke:#333,stroke-width:2px
```

## 📊 Memory Operations Flow

### Запись и продвижение

```mermaid
graph TD
    subgraph "Write Path"
        W[New Record] --> VAL[Validate]
        VAL --> EMBED[Generate Embedding]
        EMBED --> ENRICH[Enrich Metadata]
        ENRICH --> STORE[Store in Layer]
    end
    
    subgraph "Promotion Path"
        STORE --> INTERACT[Interact Layer]
        INTERACT --> EVAL{Evaluate}
        EVAL -->|Score High| PROMOTE[Promote to Insights]
        EVAL -->|Score Low| STAY[Stay in Interact]
        EVAL -->|TTL Expired| DELETE[Delete]
        
        PROMOTE --> INSIGHTS[Insights Layer]
        INSIGHTS --> EVAL2{Evaluate}
        EVAL2 -->|Critical| ASSETS[Assets Layer]
        EVAL2 -->|Normal| STAY2[Stay in Insights]
        EVAL2 -->|TTL Expired| DELETE
    end
    
    style INTERACT fill:#fdd,stroke:#333
    style INSIGHTS fill:#dfd,stroke:#333
    style ASSETS fill:#ddf,stroke:#333
```

## 🚦 Control Flow Patterns

### Retry & Fallback

```mermaid
flowchart LR
    OP[Operation] --> TRY{Try}
    TRY -->|Success| OK[Return Result]
    TRY -->|Fail| RETRY{Retry Count?}
    
    RETRY -->|< Max| BACKOFF[Exponential Backoff]
    BACKOFF --> TRY
    
    RETRY -->|>= Max| FALLBACK{Fallback?}
    FALLBACK -->|Available| FB[Use Fallback]
    FALLBACK -->|None| ERR[Return Error]
    
    FB --> OK2[Return Degraded]
    
    style TRY decision fill:#ffd
    style FALLBACK decision fill:#dff
```

### Health Monitoring

```mermaid
graph LR
    subgraph "Continuous Monitoring"
        M1[Memory Health] --> AGG[Aggregator]
        M2[Vector Health] --> AGG
        M3[GPU Health] --> AGG
        M4[LLM Health] --> AGG
    end
    
    subgraph "Alert System"
        AGG --> THRESH{Threshold}
        THRESH -->|Warning| WARN[Log Warning]
        THRESH -->|Critical| ALERT[Send Alert]
        THRESH -->|OK| OK[Update Metrics]
    end
    
    subgraph "Actions"
        ALERT --> NOTIFY[Notifications]
        ALERT --> DEGRADE[Degrade Service]
        ALERT --> HEAL[Self Healing]
    end
```

## 🏷️ Теги

#dataflow #architecture #pipeline #flow #leaf

---
[[_Architecture Hub - Центр архитектурной информации|← К центру архитектурного одуванчика]]