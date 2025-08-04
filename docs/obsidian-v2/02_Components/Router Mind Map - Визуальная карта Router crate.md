# Router Mind Map - Визуальная карта Router crate

> Лист компонентного одуванчика - визуальная карта Router crate и его компонентов

[[_Components Hub - Центр всех компонентов системы]] → Router Mind Map

## 🧠 Полная карта Router System

```mermaid
mindmap
  root((Router System))
    Core Components
      SmartRouter[70%]
        Intent Classification
        Task Decomposition
        Route Selection
        Execution Orchestration
      ActionPlanner
        Multi-step Planning
        DAG Construction
        Dependency Resolution
        Parallel Execution
      ToolSelector
        Tool Matching
        Capability Analysis
        Parameter Extraction
        Safety Validation
    
    Routing Strategies
      Intent-based
        Chat Route
        Search Route
        Tool Route
        Smart Route
      Complexity-based
        Simple Tasks
        Multi-step Tasks
        Dependent Tasks
        Parallel Tasks
      Resource-based
        CPU Tasks
        GPU Tasks
        Memory Tasks
        Network Tasks
    
    Execution Engine
      Task Queue
        Priority Queue
        Dependency Graph
        Resource Limits
        Timeout Control
      Parallel Executor
        Thread Pool
        Async Runtime
        Result Aggregation
        Error Propagation
      Progress Tracking
        Step Completion
        Time Estimation
        Resource Usage
        Error Recovery
    
    Integration Points
      LLM Integration
        Query Enhancement
        Response Generation
        Context Management
      Memory Integration
        Context Loading
        Result Storage
        Pattern Learning
      Tool Integration
        Parameter Mapping
        Execution Control
        Result Processing
```

## 🔗 Взаимосвязи компонентов

```mermaid
graph TB
    subgraph "Entry Layer"
        REQ[Request]
        INTENT[Intent Analyzer]
    end
    
    subgraph "Planning Layer"
        ROUTER[Smart Router]
        PLANNER[Action Planner]
        SELECTOR[Tool Selector]
    end
    
    subgraph "Execution Layer"
        QUEUE[Task Queue]
        EXEC[Executor]
        MONITOR[Progress Monitor]
    end
    
    subgraph "Integration Layer"
        LLM[LLM Client]
        MEM[Memory Service]
        TOOLS[Tool Registry]
    end
    
    REQ --> INTENT
    INTENT --> ROUTER
    
    ROUTER --> PLANNER
    ROUTER --> SELECTOR
    
    PLANNER --> QUEUE
    SELECTOR --> QUEUE
    
    QUEUE --> EXEC
    EXEC --> MONITOR
    
    ROUTER <--> LLM
    ROUTER <--> MEM
    EXEC <--> TOOLS
    
    style ROUTER fill:#f96,stroke:#333,stroke-width:4px
    style EXEC fill:#69f,stroke:#333,stroke-width:4px
    style PLANNER fill:#9f6,stroke:#333,stroke-width:4px
```

## 📊 Маршруты и стратегии

### Дерево решений маршрутизации

```mermaid
graph TD
    START[Incoming Request] --> ANALYZE{Analyze Intent}
    
    ANALYZE --> CHAT{Chat Intent?}
    ANALYZE --> SEARCH{Search Intent?}
    ANALYZE --> TOOL{Tool Intent?}
    ANALYZE --> COMPLEX{Complex Task?}
    
    CHAT -->|Yes| CHAT_ROUTE[Chat Route]
    SEARCH -->|Yes| SEARCH_ROUTE[Search Route]
    TOOL -->|Yes| TOOL_ROUTE[Tool Route]
    COMPLEX -->|Yes| SMART_ROUTE[Smart Route]
    
    CHAT_ROUTE --> LLM_ONLY[LLM Processing]
    SEARCH_ROUTE --> VEC_SEARCH[Vector Search]
    TOOL_ROUTE --> TOOL_EXEC[Tool Execution]
    SMART_ROUTE --> PLANNING[Multi-step Planning]
    
    style ANALYZE decision fill:#ffd
    style SMART_ROUTE fill:#f96
    style PLANNING fill:#69f
```

### Классификация задач

```mermaid
pie title "Task Distribution by Type"
    "Simple Chat" : 40
    "Tool Execution" : 25
    "Vector Search" : 20
    "Complex Multi-step" : 15
```

## 🎯 Критические пути выполнения

### Path 1: Simple Task Routing

```mermaid
sequenceDiagram
    participant User
    participant Router
    participant Intent
    participant Executor
    participant Result
    
    User->>Router: "What is HNSW?"
    Router->>Intent: classify(query)
    Intent-->>Router: ChatIntent
    
    Router->>Executor: route_to_chat(query)
    Executor-->>Router: response
    
    Router-->>User: formatted_response
```

### Path 2: Multi-step Task Execution

```mermaid
flowchart LR
    TASK[Complex Task] --> DECOMPOSE[Decompose]
    
    DECOMPOSE --> T1[Subtask 1]
    DECOMPOSE --> T2[Subtask 2]
    DECOMPOSE --> T3[Subtask 3]
    
    subgraph "DAG Execution"
        T1 --> T3
        T2 --> T3
    end
    
    T3 --> AGGREGATE[Aggregate Results]
    AGGREGATE --> FINAL[Final Response]
    
    style DECOMPOSE fill:#f96
    style AGGREGATE fill:#69f
```

### Path 3: Tool Selection and Execution

```mermaid
stateDiagram-v2
    [*] --> ParseIntent
    ParseIntent --> ExtractTool
    
    state ExtractTool {
        [*] --> IdentifyTool
        IdentifyTool --> ValidateTool
        ValidateTool --> ExtractParams
    }
    
    ExtractTool --> CheckPermissions
    CheckPermissions --> ExecuteTool: Allowed
    CheckPermissions --> Reject: Denied
    
    state ExecuteTool {
        [*] --> Prepare
        Prepare --> Run
        Run --> Capture
        Capture --> Format
    }
    
    ExecuteTool --> ReturnResult
    ReturnResult --> [*]
    
    Reject --> [*]
```

## 🚀 Паттерны планирования

### DAG Construction

```mermaid
graph TD
    subgraph "Task: Refactor and Test Module"
        START[Start] --> ANALYZE[Analyze Code]
        ANALYZE --> PLAN[Plan Refactor]
        PLAN --> REFACTOR[Refactor Code]
        REFACTOR --> LINT[Run Linter]
        REFACTOR --> TEST[Run Tests]
        LINT --> FIX[Fix Issues]
        TEST --> FIX
        FIX --> COMMIT[Commit Changes]
        COMMIT --> END[End]
    end
    
    style START fill:#4f4
    style END fill:#f44
    style REFACTOR fill:#f96
    style TEST fill:#69f
```

### Parallel vs Sequential

```mermaid
graph LR
    subgraph "Sequential Tasks"
        S1[Read File] --> S2[Process Data]
        S2 --> S3[Write Result]
    end
    
    subgraph "Parallel Tasks"
        P0[Start] --> P1[Fetch API 1]
        P0 --> P2[Fetch API 2]
        P0 --> P3[Fetch API 3]
        P1 --> P4[Merge Results]
        P2 --> P4
        P3 --> P4
    end
    
    style S2 fill:#ffd
    style P4 fill:#dfd
```

## 📈 Оптимизации маршрутизации

### Кэширование решений

```mermaid
mindmap
  root((Route Cache))
    Intent Cache
      Query Patterns
      Classification Results
      TTL Management
    
    Plan Cache
      Task Decompositions
      DAG Templates
      Success Patterns
    
    Tool Cache
      Tool Capabilities
      Parameter Templates
      Validation Rules
    
    Result Cache
      Common Queries
      Tool Outputs
      Aggregated Results
```

### Адаптивная маршрутизация

```mermaid
graph TD
    subgraph "Learning System"
        HISTORY[Execution History]
        METRICS[Performance Metrics]
        FEEDBACK[User Feedback]
    end
    
    subgraph "Adaptation"
        ANALYZE_PERF[Analyze Performance]
        UPDATE_WEIGHTS[Update Weights]
        OPTIMIZE_ROUTES[Optimize Routes]
    end
    
    subgraph "Application"
        NEW_REQUEST[New Request]
        WEIGHTED_DECISION[Weighted Decision]
        IMPROVED_ROUTE[Improved Route]
    end
    
    HISTORY --> ANALYZE_PERF
    METRICS --> ANALYZE_PERF
    FEEDBACK --> ANALYZE_PERF
    
    ANALYZE_PERF --> UPDATE_WEIGHTS
    UPDATE_WEIGHTS --> OPTIMIZE_ROUTES
    
    NEW_REQUEST --> WEIGHTED_DECISION
    OPTIMIZE_ROUTES --> WEIGHTED_DECISION
    WEIGHTED_DECISION --> IMPROVED_ROUTE
```

## 🔧 Конфигурация и настройка

### Параметры маршрутизации

```yaml
router:
  intent_classification:
    confidence_threshold: 0.8
    fallback_route: "chat"
    
  planning:
    max_steps: 10
    parallel_limit: 5
    timeout_per_step: 30s
    
  execution:
    retry_attempts: 3
    backoff_multiplier: 2
    max_queue_size: 100
    
  optimization:
    cache_ttl: 3600
    learning_rate: 0.1
    history_window: 1000
```

### Метрики производительности

| Metric | Current | Target | Status |
|--------|---------|--------|---------|
| Intent Accuracy | 92% | 98% | 🟡 |
| Planning Speed | 100ms | 50ms | 🟡 |
| Execution Success | 85% | 95% | 🟠 |
| Cache Hit Rate | 60% | 80% | 🟠 |

## 🏷️ Теги компонентов

### По функциональности
- `#routing` - маршрутизация запросов
- `#planning` - планирование задач
- `#execution` - выполнение задач
- `#orchestration` - оркестрация
- `#optimization` - оптимизации

### По готовности
- `#active-development` - основная логика
- `#needs-optimization` - производительность
- `#planned` - адаптивное обучение

---
[[_Components Hub - Центр всех компонентов системы|← К центру компонентного одуванчика]]