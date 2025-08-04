# Common Mind Map - Визуальная карта Common crate

> Лист компонентного одуванчика - визуальная карта Common crate и его компонентов

[[_Components Hub - Центр всех компонентов системы]] → Common Mind Map

## 🧠 Полная карта Common System

```mermaid
mindmap
  root((Common System))
    Core Utilities
      StructuredLogging[100%]
        JSON Format
        Log Levels
        Context Enrichment
        Performance Tracking
      ErrorHandling
        Error Types
        Error Chains
        Context Propagation
        Recovery Strategies
      Configuration
        Environment Vars
        Config Files
        Default Values
        Validation
    
    Type System
      CommonTypes
        Result Types
        ID Types
        Timestamp Types
        Score Types
      Serialization
        JSON Support
        Binary Format
        Schema Evolution
        Compression
      Validation
        Input Validation
        Type Constraints
        Business Rules
        Sanitization
    
    Performance Utils
      Metrics
        Counter Types
        Gauge Types
        Histogram Types
        Timer Types
      Profiling
        CPU Profiling
        Memory Profiling
        Async Profiling
        Flame Graphs
      Benchmarking
        Micro Benchmarks
        Load Testing
        Comparison Tools
        Regression Detection
    
    System Utilities
      FileSystem
        Path Helpers
        Temp Files
        Directory Utils
        File Locking
      Networking
        HTTP Helpers
        Retry Logic
        Connection Pool
        Timeout Control
      Process
        Signal Handling
        Process Management
        Environment Setup
        Resource Limits
```

## 🔗 Взаимосвязи компонентов

```mermaid
graph TB
    subgraph "Logging Layer"
        LOG[Structured Logger]
        FMT[Log Formatter]
        SINK[Log Sinks]
    end
    
    subgraph "Error Layer"
        ERR[Error Types]
        CHAIN[Error Chain]
        HANDLE[Error Handler]
    end
    
    subgraph "Type Layer"
        TYPES[Common Types]
        SERDE[Serialization]
        VALID[Validation]
    end
    
    subgraph "Metrics Layer"
        METRIC[Metrics Collector]
        EXPORT[Metrics Exporter]
        ALERT[Alert Manager]
    end
    
    LOG --> FMT
    FMT --> SINK
    
    ERR --> CHAIN
    CHAIN --> HANDLE
    HANDLE --> LOG
    
    TYPES --> SERDE
    TYPES --> VALID
    
    METRIC --> EXPORT
    EXPORT --> ALERT
    
    style LOG fill:#f96,stroke:#333,stroke-width:4px
    style TYPES fill:#69f,stroke:#333,stroke-width:4px
    style METRIC fill:#9f6,stroke:#333,stroke-width:4px
```

## 📊 Logging система

### Структурированные логи

```mermaid
graph LR
    subgraph "Log Entry"
        LEVEL[Level]
        MSG[Message]
        CTX[Context]
        TIME[Timestamp]
    end
    
    subgraph "Enrichment"
        TRACE[Trace ID]
        USER[User ID]
        REQ[Request ID]
        META[Metadata]
    end
    
    subgraph "Output Format"
        JSON[JSON Format]
        PRETTY[Pretty Print]
        COMPACT[Compact]
    end
    
    LEVEL --> JSON
    MSG --> JSON
    CTX --> JSON
    TIME --> JSON
    
    TRACE --> CTX
    USER --> CTX
    REQ --> CTX
    META --> CTX
    
    style JSON fill:#4f4
```

### Уровни логирования

```mermaid
stateDiagram-v2
    [*] --> TRACE: Detailed Debug
    TRACE --> DEBUG: Important Debug
    DEBUG --> INFO: Information
    INFO --> WARN: Warnings
    WARN --> ERROR: Errors
    ERROR --> FATAL: Critical
    
    state INFO {
        [*] --> Request
        Request --> Response
        Response --> [*]
    }
    
    state ERROR {
        [*] --> Capture
        Capture --> Context
        Context --> Stack
        Stack --> [*]
    }
```

## 🎯 Система ошибок

### Иерархия ошибок

```mermaid
graph TD
    subgraph "Base Error Types"
        APP[AppError]
        IO[IoError]
        NET[NetworkError]
        PARSE[ParseError]
    end
    
    subgraph "Domain Errors"
        MEM[MemoryError]
        LLM[LLMError]
        TOOL[ToolError]
        AI[AIError]
    end
    
    subgraph "Error Info"
        CODE[Error Code]
        MSG[Message]
        CTX[Context]
        CAUSE[Root Cause]
    end
    
    APP --> MEM
    APP --> LLM
    APP --> TOOL
    APP --> AI
    
    IO --> APP
    NET --> APP
    PARSE --> APP
    
    APP --> CODE
    APP --> MSG
    APP --> CTX
    APP --> CAUSE
    
    style APP fill:#f96
```

### Обработка ошибок

```mermaid
flowchart LR
    ERR[Error Occurs] --> CAPTURE{Capture Context}
    
    CAPTURE --> LOG[Log Error]
    CAPTURE --> WRAP[Wrap with Context]
    
    WRAP --> PROPAGATE{Propagate?}
    
    PROPAGATE -->|Yes| CHAIN[Add to Chain]
    PROPAGATE -->|No| HANDLE[Handle Locally]
    
    CHAIN --> RETURN[Return Result]
    HANDLE --> RECOVER[Try Recovery]
    
    RECOVER --> SUCCESS[Continue]
    RECOVER --> FAIL[Graceful Fail]
    
    style CAPTURE decision fill:#ffd
    style RECOVER fill:#9f6
```

## 🚀 Метрики и мониторинг

### Типы метрик

```mermaid
mindmap
  root((Metrics))
    Counters
      Request Count
      Error Count
      Success Count
      Cache Hits
    
    Gauges
      Memory Usage
      CPU Usage
      Active Connections
      Queue Size
    
    Histograms
      Response Time
      Processing Time
      Queue Wait Time
      Size Distribution
    
    Timers
      Operation Duration
      Request Latency
      Processing Steps
      Total Runtime
```

### Экспорт метрик

```mermaid
graph TD
    subgraph "Collection"
        APP[Application] --> COL[Collector]
        COL --> AGG[Aggregator]
    end
    
    subgraph "Export Formats"
        PROM[Prometheus]
        JSON[JSON API]
        OTLP[OpenTelemetry]
        CUSTOM[Custom Format]
    end
    
    subgraph "Destinations"
        GRAFANA[Grafana]
        ELASTIC[ElasticSearch]
        CONSOLE[Console]
        FILE[File]
    end
    
    AGG --> PROM
    AGG --> JSON
    AGG --> OTLP
    AGG --> CUSTOM
    
    PROM --> GRAFANA
    JSON --> ELASTIC
    OTLP --> GRAFANA
    CUSTOM --> FILE
    
    style COL fill:#f96
    style PROM fill:#9f6
```

## 📈 Производительность утилит

### Профилирование

```mermaid
graph LR
    subgraph "Profile Types"
        CPU[CPU Profile]
        MEM[Memory Profile]
        ASYNC[Async Profile]
        BLOCK[Blocking Profile]
    end
    
    subgraph "Tools"
        PERF[perf/pprof]
        FLAME[Flamegraph]
        TRACE[Tracing]
        BENCH[Criterion]
    end
    
    subgraph "Analysis"
        HOT[Hotspots]
        LEAK[Memory Leaks]
        CONT[Contention]
        ALLOC[Allocations]
    end
    
    CPU --> PERF
    MEM --> TRACE
    ASYNC --> TRACE
    BLOCK --> FLAME
    
    PERF --> HOT
    TRACE --> LEAK
    TRACE --> CONT
    FLAME --> ALLOC
```

### Benchmark фреймворк

```rust
// Пример benchmark
#[bench]
fn bench_structured_log(b: &mut Bencher) {
    let logger = StructuredLogger::new();
    b.iter(|| {
        logger.info("test message")
            .with_context("request_id", "123")
            .with_metric("latency", 42.5)
            .log();
    });
}

// Результаты
// structured_log: 125 ns/iter (+/- 10)
// json_format:    450 ns/iter (+/- 25)
// file_write:     850 ns/iter (+/- 50)
```

## 🔧 Конфигурация

### Переменные окружения

```bash
# Logging
RUST_LOG=debug
LOG_FORMAT=json
LOG_OUTPUT=stdout

# Metrics
METRICS_ENABLED=true
METRICS_INTERVAL=60s
METRICS_ENDPOINT=:9090

# Performance
PROFILE_ENABLED=false
TRACE_ENABLED=false
BENCHMARK_RUNS=100
```

### Настройки по умолчанию

```yaml
common:
  logging:
    level: info
    format: json
    outputs:
      - stdout
      - file: /var/log/magray.log
    
  metrics:
    enabled: true
    interval: 60s
    exporters:
      - prometheus
      - json_api
    
  errors:
    capture_stack: true
    max_chain_depth: 10
    include_source: debug_only
```

## 🏷️ Теги компонентов

### По функциональности
- `#logging` - система логирования
- `#errors` - обработка ошибок
- `#metrics` - сбор метрик
- `#utilities` - общие утилиты
- `#performance` - инструменты производительности

### По готовности
- `#production-ready` - logging, базовые утилиты
- `#stable` - метрики, профилирование
- `#experimental` - расширенный трейсинг

---
[[_Components Hub - Центр всех компонентов системы|← К центру компонентного одуванчика]]